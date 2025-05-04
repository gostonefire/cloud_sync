use serde::Deserialize;

#[derive(Deserialize)]
pub struct File {}

#[derive(Deserialize)]
pub struct ParentReference {
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct Value {
    pub id: String,
    #[serde(rename = "lastModifiedDateTime")]
    pub last_modified_date_time: String,
    pub name: String,
    pub size: u64,
    #[serde(rename = "parentReference")]
    pub parent_reference: ParentReference,
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