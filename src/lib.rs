//! # Nimbus
//! Helper library for Cloud
//!
//! provides helper functions and re-exports for:
//! - [google-cloudtask2](https://docs.rs/google-cloudtasks2)
//! - [google-cloudsecretmanager1](https://docs.rs/google-cloudsecretmanager1)
//! - [google-cloudstorage1](https://docs.rs/google-cloudstorage1)
//!
//! Traits:
//! - [`secret::SecretManagerHelper`] trait for [`google_secretmanager1::SecretManager`]
//! - [`storage::StorageHelper`] trait for [`google_storage1::Storage`]
//! - [`task::TaskHelper`] trait for [`google_cloudtasks2::api::Task`]
//! - [`task::CloudTaskHelper`] trait for [`google_cloudtasks2::CloudTasks`]
//!
//! # Examples
//!
//! ## SecretManager
//!
//! ```
//! use nimbus::SecretManagerHelper;
//! use nimbus::{ SecretManager, Authenticator };
//! use google_auth_helper::helper::AuthHelper; // [`google_auth_helper`] crate is not re-exported
//!
//! #[tokio::main]
//! async fn main() {
//!    let auth = Authenticator::auth().await.unwrap();
//!    let secret_manager = SecretManager::new_with_authenticator(auth).await;
//!
//!    let secret = secret_manager.get_secret("project", "secret").await.unwrap();
//!    let secret = String::from_utf8(secret).unwrap();
//!    println!("{}", secret);
//! }
//! ```
//!
//! ## Storage
//!
//! ```
//! use nimbus::StorageHelper;
//! use nimbus::{ ClientConfig, Client };
//! use google_auth_helper::helper::AuthHelper; // [`google_auth_helper`] crate is not re-exported
//!
//! #[tokio::main]
//! async fn main() {
//!    let config = ClientConfig::auth().await.unwrap();
//!    let client = Client::new(config);
//!
//!    client.upload_from_bytes("bucket", "key", None, b"test".to_vec()).await.unwrap();
//!    let data = client.download_to_bytes("bucket", "key").await.unwrap();
//!
//!    assert_eq!(data, b"test".to_vec());
//! }
//! ```
//!
//! ## CloudTasks
//!
//! ```
//! use nimbus::{CloudTaskHelper, TaskHelper};
//! use nimbus::{ CloudTasks, Authenticator, Task };
//! use google_auth_helper::helper::AuthHelper; // [`google_auth_helper`] crate is not re-exported
//!
//! #[tokio::main]
//! async fn main() {
//!    let auth = Authenticator::auth().await.unwrap();
//!    let client = CloudTasks::new_with_authenticator(auth).await;
//!
//!    let url = "https://example.com";
//!    let method = "GET";
//!
//!    let task = Task::new_task(url, method, None, None, None, None, None);
//!    let (res, task) = client.push_task("queue", task, None).await.unwrap();
//!
//!    assert_eq!(res.status(), 200);
//! }
//! ```
pub mod secret;
pub mod storage;
pub mod task;

pub use secret::SecretManagerHelper;
pub use storage::StorageHelper;
pub use task::{CloudTaskHelper, TaskHelper};

// Re-Export crates
pub use google_cloudtasks2;
pub use google_cloudtasks2::{CloudTasks, api::{Task, OidcToken}};
pub use google_secretmanager1;
pub use google_secretmanager1::SecretManager;
pub use google_cloud_storage;
pub use google_cloud_storage::client::{Client,ClientConfig};
pub use yup_oauth2;
pub use yup_oauth2::authenticator::Authenticator;
pub use yup_oauth2::hyper::client::HttpConnector;
pub use yup_oauth2::hyper_rustls::HttpsConnector;

// custom types

pub type CloudTaskClient = CloudTasks<HttpsConnector<HttpConnector>>;
pub type SecretManagerClient = SecretManager<HttpsConnector<HttpConnector>>;
pub type DefaultConnector = HttpsConnector<HttpConnector>;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NimbusError {
    #[error("SecretManager error: {0}")]
    SecretManager(#[from] secret::Error),
    #[error("Storage error: {0}")]
    StorageClient(#[from] storage::Error),
    #[error("CloudTasks error: {0}")]
    TasksClient(#[from] task::Error),
    #[error("Error: {0}")]
    Other(String),
}