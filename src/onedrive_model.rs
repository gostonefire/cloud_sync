use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct File {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

#[derive(Deserialize)]
pub struct Folder {
    #[serde(rename = "childCount")]
    pub child_count: i64,
}

#[derive(Deserialize)]
pub struct ParentReference {
    #[serde(rename = "driveType")]
    pub drive_type: String,
    #[serde(rename = "driveId")]
    pub drive_id: String,
    pub id: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "siteId")]
    pub site_id: Option<String>,
}

#[derive(Deserialize)]
pub struct Value {
    #[serde(rename = "createdDateTime")]
    pub created_date_time: String,
    pub id: String,
    #[serde(rename = "lastModifiedDateTime")]
    pub last_modified_date_time: String,
    pub name: String,
    pub size: i64,
    #[serde(rename = "parentReference")]
    pub parent_reference: ParentReference,
    #[serde(rename = "fileSystemInfo")]
    pub folder: Option<Folder>,
    pub file: Option<File>,
}

#[derive(Deserialize)]
pub struct Root {
    #[serde(rename = "@odata.context")]
    pub _odata_context: Option<String>,
    #[serde(rename = "@odata.nextLink")]
    pub _odata_next_link: Option<String>,
    #[serde(rename = "@odata.deltaLink")]
    pub _odata_delta_link: Option<String>,
    pub value: Option<Vec<Value>>,
}