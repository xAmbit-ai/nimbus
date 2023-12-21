pub mod secret;
pub mod storage;
pub mod task;

pub use secret::SecretManagerHelper;
pub use storage::StorageHelper;
pub use task::{CloudTaskHelper, TaskHelper};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NimbusError {
    #[error("SecretManager error: {0}")]
    Secret(#[from] secret::Error),
    #[error("Storage error: {0}")]
    Storage(#[from] storage::Error),
    #[error("CloudTasks error: {0}")]
    Task(#[from] task::Error),
    #[error("Error: {0}")]
    Other(String),
}