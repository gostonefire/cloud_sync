use std::path::Path;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::errors::OneDriveError;
use crate::onedrive_model::{Root, Value};

#[derive(Debug)]
pub struct ItemInfo {
    pub filename: String,
    pub item_id: String,
    pub size: u64,
    pub mtime: String,
    pub content_type: Option<String>,
    pub file: bool,
}

#[derive(Serialize, Deserialize)]
struct DataDeltaLink {
    data_delta_link: String,
    date_time: DateTime<Utc>
}

pub struct OneDrive {
    client: reqwest::Client,
    access_token: String,
    delta_link_path: String,
    delta_link: DataDeltaLink,
}

impl OneDrive {
    
    /// Returns a new OneDrive struct
    /// 
    pub fn new(delta_link_path: &str) -> Result<Self, OneDriveError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        Ok(OneDrive {
            client,
            access_token: String::default(),
            delta_link_path: delta_link_path.to_string(),
            delta_link: DataDeltaLink {
                data_delta_link: String::default(),
                date_time: Default::default(),
            }
        })
    }

    /// Sets an access token
    /// It is up to the caller to ensure that this is done when needed
    /// 
    /// # Arguments
    /// 
    /// * 'access_token' - access token to set
    pub fn set_access_token(&mut self, access_token: &str) {
        self.access_token = access_token.to_string();
    }
    
    /// Returns the download url for the given item id
    ///
    /// # Arguments
    ///
    /// * 'item_id' - the item id for the file to get download url for
    pub async fn get_download_url(&self, item_id: &str) -> Result<String, OneDriveError> {
        let auth = format!("Bearer {}", self.access_token);
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
    pub async fn get_file_range(&self, url: &str, from: u64, to: u64) -> Result<Vec<u8>, OneDriveError> {
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
    pub async fn get_delta(&mut self) -> Result<Vec<ItemInfo>, OneDriveError> {
        let auth = format!("Bearer {}", self.access_token);

        let mut url: String = if let Some(delta_link) = self.get_delta_link().await? {
            delta_link.to_string()
        } else {
            "https://graph.microsoft.com/v1.0/me/drive/root/delta".to_string()
        };

        let mut deltas: Vec<ItemInfo> = Vec::new();
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
                value.into_iter()
                    .filter(|v| v.parent_reference.path.is_some() && v.deleted.is_none())
                    .for_each(|v| deltas.push(OneDrive::item_info(v)));
            }

            if let Some(next_url) = delta._odata_next_link {
                url = next_url;
                continue;
            } else if let Some(delta_link) = delta._odata_delta_link {
                self.store_delta_link(delta_link);
                return Ok(deltas);
            } else {
                return Err(OneDriveError("no next or delta link returned".to_string()));
            }
        }
    }

    /// Loads and returns any existing data delta link
    /// 
    async fn get_delta_link(&self) -> Result<Option<String>, OneDriveError> {
        let path = Path::new(&self.delta_link_path);
        if path.exists() {
            let json = tokio::fs::read_to_string(path).await?;
            let link: DataDeltaLink = serde_json::from_str(&json)?;
            
            Ok(Some(link.data_delta_link))
        } else {
            Ok(None)
        }
    }
    
    /// Stores the delta link in self
    /// 
    /// # Arguments
    /// 
    /// * 'delta_link' - the data delta link to store
    fn store_delta_link(&mut self, delta_link: String) {
        self.delta_link = DataDeltaLink {
            data_delta_link: delta_link,
            date_time: Utc::now(),
        }
    }
    
    /// Saves the data delta link
    /// If this function is called before calling the function get_delta, the previous
    /// existing data delta link will be saved 
    /// 
    pub async fn save_delta_link(&self) -> Result<(), OneDriveError> {
        let json = serde_json::to_string_pretty(&self.delta_link)?;
        tokio::fs::write(&self.delta_link_path, json).await?;
        
        Ok(())
    }
    
    /// Converts a Value struct to an ItemInfo struct
    /// 
    /// # Arguments
    /// 
    /// * 'value' - the Value struct to convert
    fn item_info(value: Value) -> ItemInfo {
        let mut filename = value.parent_reference.path
            .unwrap()
            .split_once(':')
            .unwrap().1
            .to_string() + "/" + &value.name.unwrap();
        
        filename = filename.trim_start_matches('/').to_string();

        let (file, content_type) = if let Some(file) = value.file {
            (true, file.mime_type)
        } else {
            (false, None)
        };
        
        ItemInfo {
            filename,
            item_id: value.id,
            size: value.size,
            mtime: value.last_modified_date_time.unwrap().timestamp().to_string(),
            content_type,
            file,
        }
    }
}
