use crate::errors::OneDriveError;
use crate::onedrive_model::{Root, Value};

pub struct OneDrive {
    client: reqwest::Client,
}

impl OneDrive {
    
    /// Returns a new OneDrive struct
    /// 
    pub fn new() -> Result<Self, OneDriveError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        Ok(OneDrive {
            client,
        })
    }

    /// Returns the download url for the given item id
    ///
    /// # Arguments
    ///
    /// * 'access_token' - access token ot use
    /// * 'item_id' - the item id for the file to get download url for
    pub async fn get_download_url(&self, access_token: &str, item_id: &str) -> Result<String, OneDriveError> {
        let auth = format!("Bearer {}", access_token);
        let url: &str = &format!("https://graph.microsoft.com/v1.0/me/drive/items/{}/content", item_id);

        // Get download url which comes as the Location header value from a redirect 
        let res = self.client
            .get(url)
            .header("Authorization", &auth)
            .send()
            .await?;

        if !res.status().is_redirection() {
            return Err(OneDriveError(format!("get download url status: {}", res.status())));
        }

        if let Some(location) = res.headers().get("Location") {
            Ok(location.to_str()?.to_string())
        } else {
            Err(OneDriveError(format!("get Location header value: {:?}", res.headers())))
        }
    }

    /// Returns a range from a file
    ///
    /// # Arguments
    ///
    /// * 'url' - the download url as gotten from get_download_url
    /// * 'from' - first byte to read
    /// * 'to' - last byte to read
    pub async fn get_file_range(&self, url: &str, from: i64, to: i64) -> Result<Vec<u8>, OneDriveError> {
        let res = self.client
            .get(url)
            .header("Range", format!("bytes={}-{}", from, to))
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(OneDriveError(format!("get file status: {}", res.status())));
        }

        Ok(res.bytes().await?.to_vec())
    }

    /// Returns a file
    ///
    /// # Arguments
    ///
    /// * 'url' - the download url as gotten from get_download_url
    pub async fn get_file(&self, url: &str) -> Result<Vec<u8>, OneDriveError> {
        let res = self.client
            .get(url)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(OneDriveError(format!("get file status: {}", res.status())));
        }

        Ok(res.bytes().await?.to_vec())
    }
    
    /// Returns all deltas since last call for deltas
    ///
    /// # Arguments
    ///
    /// * 'access_token' - access token to use
    /// * 'delta_link' - if given the deltas returned are only changes since the delta link was retrieved
    pub async fn get_delta(&self, access_token: &str, delta_link: Option<&str>) -> Result<(Vec<Value>, String), OneDriveError> {
        let auth = format!("Bearer {}", access_token);

        let mut url: String = if let Some(delta_link) = delta_link {
            delta_link.to_string()
        } else {
            "https://graph.microsoft.com/v1.0/me/drive/root/delta".to_string()
        };

        let mut deltas: Vec<Value> = Vec::new();
        loop {
            let res = self.client
                .get(&url)
                .header("Authorization", &auth)
                .send()
                .await?;

            if !res.status().is_success() {
                return Err(OneDriveError(format!("Get delta status: {}", res.status())));
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
}
