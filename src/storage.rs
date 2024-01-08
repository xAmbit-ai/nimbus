use crate::NimbusError;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::delete::DeleteObjectRequest;
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use std::path::PathBuf;
use thiserror::Error;
use tokio;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Storage auth error: {0}")]
    StorageAuth(#[from] google_cloud_storage::client::google_cloud_auth::error::Error),
    #[error("Storage error: {0}")]
    Storage(#[from] google_cloud_storage::http::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("File Type Validation Error: {0}")]
    InvalidFileType(String),
    #[error("Error: {0}")]
    Other(String),
}

#[async_trait::async_trait]
pub trait StorageHelper: Sized {
    /// upload from bytes to a bucket
    async fn upload_from_bytes(
        &self,
        bucket: &str,
        key: &str,
        mime: Option<String>,
        data: Vec<u8>,
    ) -> Result<(), NimbusError>;

    /// download to bytes from a bucket
    async fn download_to_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>, NimbusError>;

    /// delete a file from a bucket
    async fn delete_file(&self, bucket: &str, key: &str) -> Result<(), NimbusError>;

    /// upload a file from a path to a bucket
    /// takes a PathBuf to file and key
    /// file name does not matter as key will be used to create the file in the bucket
    async fn upload_file(&self, bucket: &str, key: &str, path: PathBuf) -> Result<(), NimbusError> {
        let data = tokio::fs::read(path).await.map_err(|e| Error::IO(e))?;
        self.upload_from_bytes(bucket, key, None, data).await?;
        Ok(())
    }

    /// download a file from a bucket to a path to given destination directory
    async fn download_file(
        &self,
        bucket: &str,
        key: &str,
        path_dir: PathBuf,
    ) -> Result<PathBuf, NimbusError> {
        if !path_dir.exists() {
            tokio::fs::create_dir_all(path_dir.clone())
                .await
                .map_err(|e| Error::IO(e))?;
        }

        if !path_dir.is_dir() {
            return Err(
                Error::Other(format!("Path {} is not a directory", path_dir.display())).into(),
            );
        }

        let data = self.download_to_bytes(bucket, key).await?;
        let path = path_dir.join(key);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::IO(e))?;
        }

        tokio::fs::write(path.clone(), data)
            .await
            .map_err(|e| Error::IO(e))?;

        Ok(path)
    }

    /// check if file type is valid
    fn valid_file_type(&self, file: &[u8], expected: &str) -> Result<(), NimbusError> {
        let file_type = infer::get(file)
            .ok_or_else(|| Error::InvalidFileType("Failed to get file type".to_owned()))?;

        if file_type.extension() != expected {
            return Err(Error::InvalidFileType(format!(
                "File type is not valid. Expected: {}, got: {}",
                expected,
                file_type.extension()
            ))
            .into());
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageHelper for Client {
    async fn upload_from_bytes(
        &self,
        bucket: &str,
        key: &str,
        mime: Option<String>,
        data: Vec<u8>,
    ) -> Result<(), NimbusError> {
        let up_type = UploadType::Multipart(Box::new(Object {
            name: key.to_string(),
            content_type: mime,
            ..Default::default()
        }));

        let _ = self
            .upload_object(
                &UploadObjectRequest {
                    bucket: bucket.to_string(),
                    ..Default::default()
                },
                data,
                &up_type,
            )
            .await
            .map_err(|e| Error::Storage(e))?;

        Ok(())
    }

    async fn download_to_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>, NimbusError> {
        let a = self
            .download_object(
                &GetObjectRequest {
                    bucket: bucket.to_owned(),
                    object: key.to_owned(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
            .map_err(|e| Error::Storage(e))?;

        Ok(a)
    }

    async fn delete_file(&self, bucket: &str, key: &str) -> Result<(), NimbusError> {
        let _ = self
            .delete_object(&DeleteObjectRequest {
                bucket: bucket.to_owned(),
                object: key.to_owned(),
                ..Default::default()
            })
            .await
            .map_err(|e| Error::Storage(e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use google_auth_helper::helper::AuthHelper;

    #[tokio::test]
    async fn upload_download_delete_test() {
        let auth = ClientConfig::auth().await.unwrap();
        let storage = Client::new(auth);

        let bucket = std::env::var("BUCKET").unwrap();
        let key = std::env::var("KEY").unwrap();

        let data = b"Hello World".to_vec();
        storage
            .upload_from_bytes(&bucket, &key, None, data.clone())
            .await
            .unwrap();

        let data2 = storage.download_to_bytes(&bucket, &key).await.unwrap();
        assert_eq!(data, data2);

        storage.delete_file(&bucket, &key).await.unwrap();
    }

    #[tokio::test]
    async fn upload_file_download_file_test() {
        let auth = ClientConfig::auth().await.unwrap();
        let storage = Client::new(auth);

        let bucket = std::env::var("BUCKET").unwrap();
        let key = std::env::var("KEY_FILE").unwrap();

        let filename = "test.txt";
        let dir_name = "dir_test";

        tokio::fs::write(filename, "Hello World from file")
            .await
            .unwrap();

        let path = PathBuf::from(filename);
        storage
            .upload_file(&bucket, &key, path.clone())
            .await
            .unwrap();

        let path2 = PathBuf::from(dir_name);
        let dest = storage
            .download_file(&bucket, &key, path2.clone())
            .await
            .expect("Failed to download file");
        assert_eq!(dest, path2.join(key.clone()));

        let data = tokio::fs::read(path.clone()).await.unwrap();
        let data2 = tokio::fs::read(dest).await.unwrap();
        assert_eq!(data, data2);

        storage.delete_file(&bucket, &key).await.unwrap();
        tokio::fs::remove_dir_all(dir_name).await.unwrap();
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn valid_file_type_test() {
        let buf = [0xFF, 0xD8, 0xFF, 0xAA];
        let path = PathBuf::from("test.jpg");
        tokio::fs::write(path.clone(), &buf).await.unwrap();

        let data = tokio::fs::read(path.clone()).await.unwrap();
        let data = data.as_slice();

        let auth = ClientConfig::auth().await.unwrap();
        let storage = Client::new(auth);
        storage.valid_file_type(data, "jpg").unwrap();
        tokio::fs::remove_file(path).await.unwrap();
    }
}
