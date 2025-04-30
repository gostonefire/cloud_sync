use crate::errors::OneDriveError;
use crate::onedrive_model::{Root, Value};

pub async fn get_delta(access_token: &str) -> Result<(Vec<Value>, String), OneDriveError> {
    let auth = format!("Bearer {}", access_token);
    let client = reqwest::Client::new();
    let mut url: String = "https://graph.microsoft.com/v1.0/me/drive/root/delta".to_owned();
    
    let mut deltas: Vec<Value> = Vec::new();
    loop {
        let res = client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(OneDriveError(format!("Status: {}", res.status())));
        }
        
        let json = res.text().await?;

        let delta: Root = serde_json::from_str(&json)?;
        if let Some(value) = delta.value {
            value.into_iter().for_each(|v| deltas.push(v));
        }
        
        if let Some(next_url) = delta._odata_next_link {
            url = next_url;
            continue;
        } else if let Some(delta_url) = delta._odata_delta_link {
            return Ok((deltas, delta_url));
        } else {
            return Err(OneDriveError("no next or delta link returned".to_string()));
        }
    }
}

