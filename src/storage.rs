use std::path::PathBuf;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::Object;
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
use tokio;

#[async_trait::async_trait]
pub trait StorageHelper {
    async fn new_with_config(config: ClientConfig) -> Self;

    async fn upload_from_bytes(&self, bucket: &str, key: &str, mime: Option<String>, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>>;

    async fn download_to_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

    async fn upload_file(&self, bucket: &str, key: &str, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let data = tokio::fs::read(path).await?;
        self.upload_from_bytes(bucket, key, None, data).await?;
        Ok(())
    }

    async fn download_file(&self, bucket: &str, key: &str, path_dir: PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let data = self.download_to_bytes(bucket, key).await?;
        let path = path_dir.join(key);
        tokio::fs::write(path.clone(), data).await?;

        Ok(path)
    }

    fn valid_file_type(file: &[u8], expected: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_type = infer::get(&file).expect("File type is unknown");

        if file_type.mime_type() != expected {
            return Err(format!("Invalid file type: {:?}", file_type.mime_type()).into())
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageHelper for Client {
    async fn new_with_config(config: ClientConfig) -> Self {
        Client::new(config)
    }

    async fn upload_from_bytes(&self, bucket: &str, key: &str, mime: Option<String>, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        let up_type = UploadType::Multipart(Box::new(Object {
            name: key.to_string(),
            content_type: mime,
            ..Default::default()
        }));

        let _ = self.upload_object(
            &UploadObjectRequest {
                bucket: bucket.to_string(),
                ..Default::default()
            },
            data,
            &up_type,
        ).await?;

        Ok(())
    }

    async fn download_to_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let a = self.download_object(
            &GetObjectRequest {
                bucket: bucket.to_owned(),
                object: key.to_owned(),
                ..Default::default()
            },
            &Range::default(),
        ).await?;

        Ok(a)
    }
}