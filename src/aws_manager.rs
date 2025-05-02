use std::path::Path;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::primitives::{ByteStream, Length};
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use crate::errors::AWSError;

const CHUNK_SIZE: u64 = 1024 * 1024 * 10;
const MAX_CHUNKS: u64 = 10000;

pub struct ObjectInfo {
    pub filename: String,
    pub size: Option<i64>,
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
    pub async fn new(bucket: String) -> Self {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&config);

        AWS { client, bucket }
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
    pub async fn put_object(&self, object_name: &str, ext_mod_date: &str, bytes: Vec<u8>) -> Result<i64, AWSError> {
        let body = ByteStream::from(bytes);
        let response = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(object_name)
            .set_tagging(Some(format!("ext_mod_date={}", ext_mod_date)))
            .body(body)
            .send()
            .await?;

        response.size().ok_or(AWSError("zero size reported".to_owned()))
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
                                size: object.size,
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
    
    
    /// Uploads an object to the S3 bucket using a multipart upload
    /// Should be used for objects larger than 10MB, otherwise use the put object function
    /// 
    /// # Arguments
    ///
    /// * 'object_name' - name and path to be used in the S3 bucket
    /// * 'ext_mod_date' - a string representing a datetime from the source
    /// * '' - REPLACE ONCE SUITABLE METHOD IS FOUND
    pub async fn upload_object(&self, object_name: &str, ext_mod_date: &str, bytes: Vec<u8>) -> Result<(), AWSError> {
        // Create multipart upload
        let multipart_upload_res: CreateMultipartUploadOutput = self.client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(object_name)
            .set_tagging(Some(format!("ext_mod_date={}", ext_mod_date)))
            .send()
            .await?;

        let upload_id = multipart_upload_res.upload_id()
            .ok_or(AWSError("upload id not retrieved".to_owned()))?;

        let path = Path::new("C:/Slask/tmp/test.tif");
        let file_size = tokio::fs::metadata(path)
            .await
            .expect("it exists I swear")
            .len();

        let mut chunk_count = (file_size / CHUNK_SIZE) + 1;
        let mut size_of_last_chunk = file_size % CHUNK_SIZE;
        if size_of_last_chunk == 0 {
            size_of_last_chunk = CHUNK_SIZE;
            chunk_count -= 1;
        }

        if file_size == 0 {
            eprintln!("Bad file size.");
            return Ok(());
        }
        if chunk_count > MAX_CHUNKS {
            eprintln!("Too many chunks [{}]! Try increasing your chunk size.", chunk_count);
            return Ok(());
        }

        let mut upload_parts: Vec<CompletedPart> = Vec::new();

        for chunk_index in 0..chunk_count {
            let this_chunk = if chunk_count - 1 == chunk_index {
                size_of_last_chunk
            } else {
                CHUNK_SIZE
            };
            let stream = ByteStream::read_from()
                .path(path)
                .offset(chunk_index * CHUNK_SIZE)
                .length(Length::Exact(this_chunk))
                .build()
                .await
                .unwrap();

            // Chunk index needs to start at 0, but part numbers start at 1.
            let part_number = (chunk_index as i32) + 1;
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
        }

        // upload_parts: Vec<aws_sdk_s3::types::CompletedPart>
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