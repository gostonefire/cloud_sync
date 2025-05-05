use std::ops::Add;
use chrono::{DateTime, Local, NaiveTime, TimeDelta, Utc};
use log::{error, info, warn};
use tokio::time::{Instant, Duration};
use crate::aws_manager::AWS;
use crate::chunk::Chunk;
use crate::initialization::Config;
use crate::errors::CloudSyncError;
use crate::onedrive_manager::OneDrive;
use crate::token_manager::Tokens;

struct Mgr<'a> {
    one_drive: OneDrive,
    aws: AWS,
    tokens: Tokens,
    config: &'a Config,
}

/// Sync start point
/// This loop will never end unless some means of stopping it is implemented,but rather
/// report any errors encountered and after some wait try again
/// 
/// # Arguments
///
/// * 'config' - configuration struct
pub async fn sync(config: &Config) {
    loop {
        match sync_loop(&config).await {
            Ok(_) => {
                info!("sync terminated");
                break;
            },
            Err(e) => {
                match e {
                    CloudSyncError::TokenExpiredWarning => { 
                        warn!("token expired, visit <host>:8000/code to re-authorize") 
                    },
                    err => { error!("sync failed: {}", err.to_string()) },
                }
            }
        }
    }
}

/// Main cloud synchronization loop
///
/// # Arguments
///
/// * 'config' - configuration struct
async fn sync_loop(config: &Config) -> Result<(), CloudSyncError> {
    sleep_until_time(&config.general.sync_time).await;
    
    let tokens = Tokens::from_file(&config.onedrive.tokens_path).await?;
    let one_drive = OneDrive::new(&config.onedrive.delta_link_path, tokens.get_access_token())?;
    let aws = AWS::new(&config.aws.bucket).await;
    
    let mut mgr = Mgr {
        one_drive,
        aws,
        tokens,
        config,
    };
    
    loop {
        check_tokens(&mut mgr).await?;
        
        info!("get OneDrive deltas!");
        let deltas = mgr.one_drive.get_delta().await?;
        if !deltas.is_empty() {
            info!("get S3 objects!");
            let objects = mgr.aws.list_objects().await?;

            info!("checking objects!");
            for f in deltas.into_iter().filter(|f| f.file) {
                if let Some(t) = objects.iter().find(|o| f.filename == o.filename) {
                    if backup_needed(&mgr.aws, &t.filename, f.size, t.size, f.mtime).await? {
                        info!("updating file: {:?}", f.filename);
                        backup_file(&mut mgr, &f.item_id, &f.filename, f.size, &f.content_type, f.mtime).await?;
                    }
                } else {
                    info!("adding file: {:?}", f.filename);
                    backup_file(&mut mgr, &f.item_id, &f.filename, f.size, &f.content_type, f.mtime).await?;
                }
            }            
        }
        mgr.one_drive.save_delta_link().await?;
        info!("done checking objects!");

        sleep_until_time(&config.general.sync_time).await;
    }
}

/// Will sleep until next given time in local timezone
/// Avoid using hours 02 and 03 since they are behaving differently when passing between
/// normal time and daylight saving time
/// 
/// # Arguments
/// 
/// * 'time' - the time to wake up in format %H:%M:%S (e.g. 00:01:00)
async fn sleep_until_time(time: &str) {
    let now = Local::now();
    let mut proposed = Local::now().with_time(NaiveTime::parse_from_str(time, "%H:%M:%S").unwrap()).unwrap();

    if proposed <= now {
        proposed = proposed.add(TimeDelta::days(1));
    }

    info!("sleeps until: {}", proposed);
    let duration_as_secs = (proposed - now).num_seconds() as u64;
    tokio::time::sleep_until(Instant::now() + Duration::from_secs(duration_as_secs)).await;
}

/// Checks if tokens are valid and if not a refresh of tokens is attempted and
/// the OneDrive instance is accordingly updated
/// 
/// # Arguments
///
/// * 'mgr' - struct holding all managers and config
async fn check_tokens(mgr: &mut Mgr<'_>) -> Result<(), CloudSyncError> {
    if mgr.tokens.is_expired() {
        mgr.tokens.refresh_tokens(&mgr.config.onedrive).await?;
        mgr.one_drive.set_access_token(&mgr.tokens.get_access_token());
    }

    Ok(())
}

/// Returns true if there is a difference in a file between OneDrive and AWS
/// It first tries to get the last modification time from AWS and if there is a difference it returns true. 
/// If there wasn't any last modification time registered in AWS it checks if file sizes differs
/// 
/// # Arguments
/// 
/// * 'aws' - A references to the AWS struct instance
/// * 't-filename' - filename in AWS (to)
/// * 'f_size' - file size from OneDrive (from)
/// * 't_size' - file size from AWS (to)
/// * 'f_mtime' - last modification time as timestamp from OneDrive (from)
async fn backup_needed(aws: &AWS, t_filename: &str, f_size: u64, t_size: Option<u64>, f_mtime: i64) -> Result<bool, CloudSyncError> {
    if let Some(t_mtime) = aws.get_mtime(t_filename).await? {
        if f_mtime != t_mtime {
            return Ok(true);
        }
    } else if f_size != 0 && !t_size.is_some_and(|s| f_size == s) {
        return Ok(true);
    }
    
    Ok(false)
}

/// Backs up or sync a file from OneDrive to AWS
///
/// # Arguments
///
/// * 'mgr' - struct holding all managers and config
/// * 'item_id' - OneDrive item id representing the file to copy
/// * 'filename' - filename and path
/// * 'size' - size of the file on OneDrive
/// * 'content_type' - the file Content-Type
/// * 'mtime' - last modification datetime as a timestamp 
async fn backup_file(mgr: &mut Mgr<'_>, item_id: &str, filename: &str, size: u64, content_type: &Option<String>, mtime: i64) -> Result<(), CloudSyncError> {
    if size > AWS::get_chunk_size() {
        upload_file(mgr, item_id, filename, size, content_type, mtime).await?;
    } else {
        copy_file(mgr, item_id, filename, size, content_type, mtime).await?
    }
    
    Ok(())
}

/// Copies one file from OneDrive to AWS S3
/// Use this function for files less or equal to 10MB since it is reading and writing the
/// entire file in one go
/// 
/// # Arguments
///
/// * 'mgr' - struct holding all managers and config
/// * 'item_id' - OneDrive item id representing the file to copy
/// * 'filename' - filename and path
/// * 'size' - size of the file on OneDrive
/// * 'content_type' - the file Content-Type
/// * 'mtime' - last modification datetime as a timestamp 
async fn copy_file(mgr: &mut Mgr<'_>, item_id: &str, filename: &str, size: u64, content_type: &Option<String>, mtime: i64) -> Result<(), CloudSyncError> {
    check_tokens(mgr).await?;
    
    let download_url = mgr.one_drive.get_download_url(item_id).await?;
    let content = mgr.one_drive.get_file(&download_url).await?;
    if content.len() != size as usize {
        return Err(CloudSyncError::OneDrive("download size mismatch".to_string()));
    };
        
    mgr.aws.put_object(filename, &content_type, mtime, content).await?;
    
    Ok(())
}

/// Uploads one file from OneDrive to AWS S3
/// Use this function for files bigger than 10MB since it is reading and writing the
/// file in chunks of 10MB
///
/// # Arguments
///
/// * 'mgr' - struct holding all managers and config
/// * 'item_id' - OneDrive item id representing the file to copy
/// * 'filename' - filename and path
/// * 'size' - size of the file on OneDrive
/// * 'content_type' - the file Content-Type
/// * 'mtime' - last modification datetime as a timestamp 
async fn upload_file(mgr: &mut Mgr<'_>, item_id: &str, filename: &str, size: u64, content_type: &Option<String>, mtime: i64) -> Result<(), CloudSyncError> {
    AWS::check_for_multipart_upload(size)?;
    let chunk_size = AWS::get_chunk_size();

    let (mut url, mut create_url_time) = get_check_download_url(mgr, item_id, None).await?;
    let (mut upload_parts, upload_id) = mgr.aws.create_multipart_upload(filename, &content_type, mtime).await?;
    
    let chunk = Chunk::new(size, chunk_size);
    for (part, from, to) in chunk {
        (url, create_url_time) = get_check_download_url(mgr, item_id, Some((url, create_url_time))).await?;
        
        let bytes = mgr.one_drive.get_file_range(&url, from, to).await?;
        mgr.aws.upload_part(filename, &upload_id, part, bytes, &mut upload_parts).await?;
    }
    mgr.aws.complete_multipart_upload(filename, &upload_id, upload_parts).await?;
    
    Ok(())
}

/// Checks if a new download url is needed 
/// 
/// # Arguments
/// 
/// * 'mgr' - struct holding all managers and config
/// * 'item_id' - OneDrive item id representing the file to copy
/// * 'url_time' - tuple of url and create time to check
async fn get_check_download_url(mgr: &mut Mgr<'_>, item_id: &str, url_time: Option<(String, DateTime<Utc>)>) -> Result<(String, DateTime<Utc>), CloudSyncError> {
    check_tokens(mgr).await?;
    
    if let Some((url, time)) = url_time {
        if Utc::now() - time > TimeDelta::seconds(1800) {
            let url = mgr.one_drive.get_download_url(item_id).await?;
            Ok((url, Utc::now()))
        } else {
            Ok((url, time))
        }
    } else {
        Ok((mgr.one_drive.get_download_url(item_id).await?, Utc::now()))
    }
}