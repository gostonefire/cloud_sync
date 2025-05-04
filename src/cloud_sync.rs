use log::{error, info};
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
                tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;
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
        let access_token = get_token(&config).await?;
        one_drive.set_access_token(&access_token);
        
        println!("Get OneDrive deltas!");
        let deltas = one_drive.get_delta().await?;
        println!("Get S3 objects!");
        let objects = aws.list_objects().await?;

        println!("Checking objects!");
        for d in deltas.iter().filter(|f| f.file) {
            if objects.iter().find(|o| o.filename == d.filename && o.size.unwrap_or(0) == d.size).is_none() {
                println!("OneDrive: {:?}", d);
                if d.size > AWS::get_chunk_size() {
                    upload_file(&one_drive, &aws, &d.item_id, &d.filename, d.size, &d.ext_mod_date).await?;
                } else {
                    copy_file(&one_drive, &aws, &d.item_id, &d.filename, d.size, &d.ext_mod_date).await?
                }
            }
        }
        println!("Done Checking objects!");

        tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;
    }
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
/// * 'ext_mod_date' - last modification date from OneDrive, will end up as a tag on the S3 object
async fn copy_file(one_drive: &OneDrive, aws: &AWS, item_id: &str, filename: &str, size: u64, ext_mod_date: &str) -> Result<(), CloudSyncError> {
    let download_url = one_drive.get_download_url(item_id).await?;
    let content = one_drive.get_file(&download_url).await?;
    if content.len() != size as usize {
        error!("download size mismatch");
        return Err(CloudSyncError::OneDrive("download size mismatch".to_string()));
    };
        
    aws.put_object(filename, ext_mod_date, content).await?;
    
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
/// * 'ext_mod_date' - last modification date from OneDrive, will end up as a tag on the S3 object
async fn upload_file(one_drive: &OneDrive, aws: &AWS, item_id: &str, filename: &str, size: u64, ext_mod_date: &str) -> Result<(), CloudSyncError> {
    AWS::check_for_multipart_upload(size)?;
    let chunk_size = AWS::get_chunk_size();

    let url = one_drive.get_download_url(item_id).await?;
    let (mut upload_parts, upload_id) = aws.create_multipart_upload(filename, ext_mod_date).await?;
    
    let chunk = Chunk::new(size, chunk_size);
    for (part, from, to) in chunk {
        let bytes = one_drive.get_file_range(&url, from, to).await?;
        aws.upload_part(filename, &upload_id, part, bytes, &mut upload_parts).await?;
    }
    aws.complete_multipart_upload(filename, &upload_id, upload_parts).await?;
    
    Ok(())
}

