use log::error;
use crate::initialization::Config;
use crate::errors::CloudSyncError;
use crate::onedrive_manager::get_delta;
use crate::token_manager::get_token;

/// Main cloud synchronization loop
///
/// # Arguments
///
/// * 'config' - configuration struct
pub async fn sync_loop(config: &Config) -> Result<(), CloudSyncError> {
    
    loop {
        let access_token = get_token(&config).await?;
        
        match get_delta(&access_token).await {
            Err(e) => { error!("{}", e.to_string()); },
            Ok((deltas, delta_url)) => {
                println!("Deltas: {}, DeltaUrl: {}", deltas.len(), delta_url);
            },
        }
        
        
        tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;
    }
    Ok(())
}

