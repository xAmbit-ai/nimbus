use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::delete::DeleteObjectRequest;
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use std::path::PathBuf;
use tokio;

#[async_trait::async_trait]
pub trait StorageHelper {
    async fn new_with_config(config: ClientConfig) -> Self;

    async fn upload_from_bytes(
        &self,
        bucket: &str,
        key: &str,
        mime: Option<String>,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    async fn download_to_bytes(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

    async fn delete_file(&self, bucket: &str, key: &str) -> Result<(), Box<dyn std::error::Error>>;

    async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let data = tokio::fs::read(path).await?;
        self.upload_from_bytes(bucket, key, None, data).await?;
        Ok(())
    }

    async fn download_file(
        &self,
        bucket: &str,
        key: &str,
        path_dir: PathBuf,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        if !path_dir.exists() {
            tokio::fs::create_dir_all(path_dir.clone())
                .await
                .expect("Failed to create directory");
        }

        if !path_dir.is_dir() {
            return Err("Path is not a directory".into());
        }

        let data = self.download_to_bytes(bucket, key).await?;
        let path = path_dir.join(key);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .expect("Failed to create directory");
        }

        tokio::fs::write(path.clone(), data)
            .await
            .expect("Failed to write file");

        Ok(path)
    }

    fn valid_file_type(
        &self,
        file: &[u8],
        expected: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_type = infer::get(file).expect("File type is unknown");

        if file_type.extension() != expected {
            return Err(format!(
                "File extension is not {} but {}",
                expected,
                file_type.extension()
            )
            .into());
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageHelper for Client {
    async fn new_with_config(config: ClientConfig) -> Self {
        Client::new(config)
    }

    async fn upload_from_bytes(
        &self,
        bucket: &str,
        key: &str,
        mime: Option<String>,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
            .await?;

        Ok(())
    }

    async fn download_to_bytes(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let a = self
            .download_object(
                &GetObjectRequest {
                    bucket: bucket.to_owned(),
                    object: key.to_owned(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await?;

        Ok(a)
    }

    async fn delete_file(&self, bucket: &str, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self
            .delete_object(&DeleteObjectRequest {
                bucket: bucket.to_owned(),
                object: key.to_owned(),
                ..Default::default()
            })
            .await?;

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
