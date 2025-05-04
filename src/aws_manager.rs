use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use log::error;
use crate::errors::AWSError;

const CHUNK_SIZE: u64 = 1024 * 1024 * 10;
const MAX_CHUNKS: u64 = 10000;

pub struct ObjectInfo {
    pub filename: String,
    pub size: Option<u64>,
}

pub struct AWS {
    client: Client,
    bucket: String,
}

impl AWS {

    /// Creates a new AWS struct
    ///
    /// # Arguments
    ///
    /// * 'bucket' - the AWS S3 bucket to use
    pub async fn new(bucket: &str) -> Self {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&config);

        AWS { client, bucket: bucket.to_string() }
    }

    /// Puts an object to the S3 bucket
    /// Should only be used for smaller objects such as 10MB or smaller, otherwise use the
    /// multipart upload functions
    ///
    /// If returned response indicates zero size an error is returned
    ///
    /// # Arguments
    ///
    /// * 'object_name' - name and path to be used in the S3 bucket
    /// * 'ext_mod_date' - a string representing a datetime from the source
    /// * 'bytes' - the file content
    pub async fn put_object(&self, object_name: &str, ext_mod_date: &str, bytes: Vec<u8>) -> Result<(), AWSError> {
        let file_size = bytes.len() as u64;
        let body = ByteStream::from(bytes);
        let response = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(object_name)
            .set_tagging(Some(format!("ext_mod_date={}", ext_mod_date)))
            .body(body)
            .send()
            .await?;
        
        match response.size() {
            None => {
                error!("no response size returned");
                Err(AWSError::from("no response size returned"))
            }
            Some(size) => {
                if file_size != size as u64 {
                    error!("response size mismatch");
                    Err(AWSError::from("response size mismatch"))
                } else {
                    Ok(())
                }
            }
        }
    }
    
    /// Returns the ext_mod_date tag value if it exists on the object in the S3 bucket
    ///
    /// # Arguments
    ///
    /// * 'object_name' - name and path for the object in the S3 bucket 
    pub async fn get_ext_mod_date(&self, object_name: &str) -> Result<Option<String>, AWSError> {
        let response = self.client
            .get_object_tagging()
            .bucket(&self.bucket)
            .key(object_name)
            .send()
            .await?;

        let value = response.tag_set()
            .iter()
            .filter(|t| t.key == "ext_mod_date")
            .map(|t| &t.value)
            .last()
            .map(|v| v.clone());
        
        Ok(value)
    }
    
    /// Lists all objects in the S3 bucket.
    ///
    pub async fn list_objects(&self) -> Result<Vec<ObjectInfo>, AWSError> {
        let mut response = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .max_keys(100) 
            .into_paginator()
            .send();

        let mut objects: Vec<ObjectInfo> = Vec::new();
        while let Some(result) = response.next().await {
            match result {
                Ok(output) => {
                    for object in output.contents() {
                        if let Some(key) = &object.key {
                            objects.push(ObjectInfo{
                                filename: key.clone(),
                                size: object.size.map(|v| v as u64),
                            })
                        }
                    }
                }
                Err(err) => {
                    return Err(err.into());
                }
            }
        }
        Ok(objects)
    }

    /// Checks so the file size won't exceed max number of parts
    /// 
    /// # Arguments
    /// 
    /// * 'file_size' - size of file to upload
    pub fn check_for_multipart_upload(file_size: u64) -> Result<(), AWSError> {
        let mut chunk_count = (file_size / CHUNK_SIZE) + 1;
        let size_of_last_chunk = file_size % CHUNK_SIZE;
        if size_of_last_chunk == 0 {
            chunk_count -= 1;
        }

        if file_size == 0 {
            Err(AWSError::from("file size is zero"))
        } else if chunk_count > MAX_CHUNKS {
            Err(AWSError::from("chunk count exceeded maximum"))
        } else {
            Ok(())
        }
    }
    
    /// Returns the chunk size
    /// 
    pub fn get_chunk_size() -> u64 {
        CHUNK_SIZE
    }
    
    /// Creates a multipart upload
    /// This function is the starting point of a multipart file upload
    ///
    /// It returns a tuple of Vec<CompletedPart> and an upload id to be later used
    /// 
    /// # Arguments
    ///
    /// * 'object_name' - name and path to be used in the S3 bucket
    /// * 'ext_mod_date' - a string representing a datetime from the source
    pub async fn create_multipart_upload(&self, object_name: &str, ext_mod_date: &str) -> Result<(Vec<CompletedPart>, String), AWSError> {
        let multipart_upload_res: CreateMultipartUploadOutput = self.client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(object_name)
            .set_tagging(Some(format!("ext_mod_date={}", ext_mod_date)))
            .send()
            .await?;

        let upload_id = multipart_upload_res.upload_id()
            .ok_or({
                error!("upload id not retrieved");
                AWSError::from("upload id not retrieved")
            })?;
        
        let upload_parts: Vec<CompletedPart> = Vec::new();
        
        Ok((upload_parts, upload_id.to_string()))
    }

    /// Uploads a part given as a vector of bytes
    /// It also needs a mutable reference to the vector upload_parts which will be updated
    /// for each call to this function
    ///
    /// # Arguments
    ///
    /// * 'object_name' - name and path to be used in the S3 bucket
    /// * 'upload_id' - id retrieved from the call to create_multipart_upload function
    /// * 'part_number' - part number starting with 1 and shall increment by one for each call
    /// * 'bytes' - a vector of file data
    /// * 'upload_parts' - a mutable reference to upload_parts retrieved from the call to create_multipart_upload function
    pub async fn upload_part(&self, object_name: &str, upload_id: &str, part_number: i32, bytes: Vec<u8>, upload_parts: &mut Vec<CompletedPart>) -> Result<(), AWSError> {
        let stream = ByteStream::from(bytes);
        
        let upload_part_res = self.client
            .upload_part()
            .key(object_name)
            .bucket(&self.bucket)
            .upload_id(upload_id)
            .body(stream)
            .part_number(part_number)
            .send()
            .await?;

        upload_parts.push(
            CompletedPart::builder()
                .e_tag(upload_part_res.e_tag.unwrap_or_default())
                .part_number(part_number)
                .build(),
        );
        
        Ok(())
    }

    /// Completes a multipart upload
    ///
    /// # Arguments
    ///
    /// * 'object_name' - name and path to be used in the S3 bucket
    /// * 'upload_id' - id retrieved from the call to create_multipart_upload function
    /// * 'upload_parts' - the final upload_parts
    pub async fn complete_multipart_upload(&self, object_name: &str, upload_id: &str, upload_parts: Vec<CompletedPart>) -> Result<(), AWSError> {
        let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
            .set_parts(Some(upload_parts))
            .build();

        let _complete_multipart_upload_res = self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(object_name)
            .multipart_upload(completed_multipart_upload)
            .upload_id(upload_id)
            .send()
            .await?;
        
        Ok(())
    }
}