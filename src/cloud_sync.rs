use std::ops::Add;
use chrono::{Local, NaiveTime, TimeDelta};
use log::{error, info};
use tokio::time::{Instant, Duration};
use crate::aws_manager::AWS;
use crate::chunk::Chunk;
use crate::initialization::Config;
use crate::errors::CloudSyncError;
use crate::onedrive_manager::OneDrive;
use crate::token_manager::get_token;

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
                error!("sync failed: {}", e);
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
    let mut one_drive = OneDrive::new(&config.onedrive.delta_link_path)?;
    let aws = AWS::new(&config.aws.bucket).await;
    
    loop {
        sleep_until_time(1, 0, 0).await;
        
        let access_token = get_token(&config).await?;
        one_drive.set_access_token(&access_token);
        
        info!("get OneDrive deltas!");
        let deltas = one_drive.get_delta().await?;
        if !deltas.is_empty() {
            info!("get S3 objects!");
            let objects = aws.list_objects().await?;

            info!("checking objects!");
            for f in deltas.into_iter().filter(|f| f.file) {
                if let Some(t) = objects.iter().find(|o| f.filename == o.filename) {
                    if backup_needed(&aws, &t.filename, f.size, t.size, f.mtime).await? {
                        backup_file(&one_drive, &aws, &f.item_id, &f.filename, f.size, &f.content_type, f.mtime).await?;
                    }
                } else {
                    backup_file(&one_drive, &aws, &f.item_id, &f.filename, f.size, &f.content_type, f.mtime).await?;
                }
            }            
        }
        one_drive.save_delta_link().await?;
        info!("done checking objects!");

        
    }
}

/// Will sleep until next given time in local timezone
/// Avoid using hours 02 and 03 since they are behaving differently when passing between
/// normal time and daylight saving time
/// 
/// # Arguments
/// 
/// * 'hour' - the hour to wake up
/// * 'min' - the minute to wake up
/// * 'sec' - the second to wake up
async fn sleep_until_time(hour: u32, min: u32, sec: u32) {
    let now = Local::now();
    let mut proposed = Local::now().with_time(NaiveTime::from_hms_opt(hour, min, sec).unwrap()).unwrap();

    if proposed <= now {
        proposed = proposed.add(TimeDelta::days(1));
    }

    info!("sleeps until: {:?}", proposed);
    let duration_as_secs = (proposed - now).num_seconds() as u64;
    tokio::time::sleep_until(Instant::now() + Duration::from_secs(duration_as_secs)).await;
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
/// * 'one_drive' - OneDrive manager struct
/// * 'aws' - AWS manager struct
/// * 'item_id' - OneDrive item id representing the file to copy
/// * 'filename' - filename and path
/// * 'size' - size of the file on OneDrive
/// * 'content_type' - the file Content-Type
/// * 'mtime' - last modification datetime as a timestamp 
async fn backup_file(one_drive: &OneDrive, aws: &AWS, item_id: &str, filename: &str, size: u64, content_type: &Option<String>, mtime: i64) -> Result<(), CloudSyncError> {
    info!("syncing: {:?}", filename);
    if size > AWS::get_chunk_size() {
        upload_file(one_drive, aws, item_id, filename, size, content_type, mtime).await?;
    } else {
        copy_file(one_drive, aws, item_id, filename, size, content_type, mtime).await?
    }
    
    Ok(())
}

/// Copies one file from OneDrive to AWS S3
/// Use this function for files less or equal to 10MB since it is reading and writing the
/// entire file in one go
/// 
/// # Arguments
/// 
/// * 'one_drive' - OneDrive manager struct
/// * 'aws' - AWS manager struct
/// * 'item_id' - OneDrive item id representing the file to copy
/// * 'filename' - filename and path
/// * 'size' - size of the file on OneDrive
/// * 'content_type' - the file Content-Type
/// * 'mtime' - last modification datetime as a timestamp 
async fn copy_file(one_drive: &OneDrive, aws: &AWS, item_id: &str, filename: &str, size: u64, content_type: &Option<String>, mtime: i64) -> Result<(), CloudSyncError> {
    let download_url = one_drive.get_download_url(item_id).await?;
    let content = one_drive.get_file(&download_url).await?;
    if content.len() != size as usize {
        return Err(CloudSyncError::OneDrive("download size mismatch".to_string()));
    };
        
    aws.put_object(filename, &content_type, mtime, content).await?;
    
    Ok(())
}

/// Uploads one file from OneDrive to AWS S3
/// Use this function for files bigger than 10MB since it is reading and writing the
/// file in chunks of 10MB
///
/// # Arguments
///
/// * 'one_drive' - OneDrive manager struct
/// * 'aws' - AWS manager struct
/// * 'item_id' - OneDrive item id representing the file to copy
/// * 'filename' - filename and path
/// * 'size' - size of the file on OneDrive
/// * 'content_type' - the file Content-Type
/// * 'mtime' - last modification datetime as a timestamp 
async fn upload_file(one_drive: &OneDrive, aws: &AWS, item_id: &str, filename: &str, size: u64, content_type: &Option<String>, mtime: i64) -> Result<(), CloudSyncError> {
    AWS::check_for_multipart_upload(size)?;
    let chunk_size = AWS::get_chunk_size();
    
    let url = one_drive.get_download_url(item_id).await?;
    let (mut upload_parts, upload_id) = aws.create_multipart_upload(filename, &content_type, mtime).await?;
    
    let chunk = Chunk::new(size, chunk_size);
    for (part, from, to) in chunk {
        let bytes = one_drive.get_file_range(&url, from, to).await?;
        aws.upload_part(filename, &upload_id, part, bytes, &mut upload_parts).await?;
    }
    aws.complete_multipart_upload(filename, &upload_id, upload_parts).await?;
    
    Ok(())
}

