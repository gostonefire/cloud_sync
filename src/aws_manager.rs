use std::str::FromStr;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::operation::head_object::{HeadObjectError, HeadObjectOutput};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;
use aws_smithy_runtime_api::client::result::SdkError;
use crate::errors::AWSError;

const CHUNK_SIZE: u64 = 1024 * 1024 * 10;
const MAX_CHUNKS: u64 = 10000;

pub struct ObjectInfo {
    pub mtime: Option<i64>,
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
    /// # Arguments
    ///
    /// * 'object_name' - name and path to be used in the S3 bucket
    /// * 'content_type' - the file Content-Type
    /// * 'mtime' - last modification datetime as a timestamp
    /// * 'bytes' - the file content
    pub async fn put_object(&self, object_name: &str, content_type: &Option<String>, mtime: i64, bytes: Vec<u8>) -> Result<(), AWSError> {
        let body = ByteStream::from(bytes);
        let _ = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(object_name)
            .metadata("mtime", mtime.to_string())
            .set_content_type(content_type.clone())
            .body(body)
            .send()
            .await?;

        Ok(())
    }

    /// Returns object information och which the mtime attribute is a timestamp
    /// reflecting the last modified date time
    ///
    /// # Arguments
    ///
    /// * 'object_name' - name and path to the S3 object
    pub async fn get_object_info(&self, object_name: &str) -> Result<Option<ObjectInfo>, AWSError> {
        let result = self.client
            .head_object()
            .bucket(&self.bucket)
            .key(object_name)
            .send()
            .await;

        let response: Option<ObjectInfo> = match result {
            Ok(head) => { 
                Some(Self::construct_object_info(head))
            },
            Err(err) => {
                Self::construct_object_info_error(err)?
            }
        };

        Ok(response)
    }

    /// Construct an ObjectInfo instance from the HeadObjectOutput result from
    /// an AWS S3 head_object() function call
    ///
    /// # Arguments
    ///
    /// * 'head' - a HeadObjectOutput instance 
    fn construct_object_info(head: HeadObjectOutput) -> ObjectInfo {
        let mtime = match head.metadata {
            Some(metadata) => {
                if let Some(mtime) = metadata.get("mtime") {
                    let trimmed = if mtime.contains('.') {
                        mtime.split_once('.').unwrap().0
                    } else {
                        &mtime
                    };

                    i64::from_str(trimmed).ok()
                } else {
                    None
                }
            },
            None => None
        };
        
        ObjectInfo {
            mtime,
            size: head.content_length.map(|x| x as u64),
        }
    }
    
    /// Constructs an AWSError or a None response depending on whether the error is due to
    /// missing file or an actual error
    /// 
    /// # Arguments
    /// 
    /// * 'err' - a SdkError<HeadObjectError, HttpResponse> instance
    fn construct_object_info_error(err: SdkError<HeadObjectError, HttpResponse>) -> Result<Option<ObjectInfo>, AWSError> {
        match err {
            SdkError::ServiceError(service_err) => {
                let http = service_err.raw();
                match http.status().as_u16() {
                    404 => {
                        Ok(None)
                    },
                    status => Err(AWSError(format!("HttpStatus: {}", status))),
                }
            }
            _ => Err(AWSError::from(err)),
        }
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
    /// * 'content_type' - the file Content-Type
    /// * 'mtime' - last modification datetime as a timestamp
    pub async fn create_multipart_upload(&self, object_name: &str, content_type: &Option<String>, mtime: i64) -> Result<(Vec<CompletedPart>, String), AWSError> {
        let multipart_upload_res: CreateMultipartUploadOutput = self.client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(object_name)
            .metadata("mtime", mtime.to_string())
            .set_content_type(content_type.clone())
            .send()
            .await?;

        let upload_id = multipart_upload_res.upload_id().ok_or({
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