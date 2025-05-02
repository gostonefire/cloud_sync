use log::error;
use crate::aws_manager::AWS;
use crate::initialization::Config;
use crate::errors::CloudSyncError;
use crate::onedrive_manager::OneDrive;
use crate::token_manager::get_token;

/// Main cloud synchronization loop
///
/// # Arguments
///
/// * 'config' - configuration struct
pub async fn sync_loop(config: &Config) -> Result<(), CloudSyncError> {
    let one_drive = OneDrive::new()?;
    let aws = AWS::new("test.stonefire.se".to_string()).await;
    
    loop {
        let access_token = get_token(&config).await?;
        
        copy_file(&one_drive, &access_token, &aws, "ADD18EB3F0B272FF!217790", "20231022_103657.jpg", 2912772).await?;
        
        /*
        match one_drive.get_download_url(&access_token, "ADD18EB3F0B272FF!217790").await {
            Ok(url) => {
                println!("Download url: {}", url);
                match one_drive.get_file_range(&url, 0, 2047).await {
                    Ok(file_range) => {
                        println!("File range: {:?}", file_range.len());
                    },
                    Err(e) => { error!("{}", e.to_string()); }
                }
                match one_drive.get_file_range(&url, 2048, 4095).await {
                    Ok(file_range) => {
                        println!("File range: {:?}", file_range.len());
                    },
                    Err(e) => { error!("{}", e.to_string()); }
                }
            },
            Err(e) => { error!("{}", e.to_string()); }
        }
        
         */
        
        /*
        match get_delta(&access_token).await {
            Err(e) => { error!("{}", e.to_string()); },
            Ok((deltas, delta_url)) => {
                println!("Deltas: {}, DeltaUrl: {}", deltas.len(), delta_url);
            },
        }
        */
        
        tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;
    }
    Ok(())
}

async fn copy_file(one_drive: &OneDrive, access_token: &str, aws: &AWS, item_id: &str, file_name: &str, size: i64) -> Result<(), CloudSyncError> {
    if size <= 1024 * 1024 * 10 {
        let download_url = one_drive.get_download_url(access_token, item_id).await?;
        let content = one_drive.get_file(&download_url).await?;
        
        let uploaded_size = aws.put_object(file_name, "2025-05-02", content).await?;
        
        if uploaded_size != size {
            Err(CloudSyncError::AWS("upload size mismatch".to_string()))
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}