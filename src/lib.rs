pub mod secret;
pub mod storage;
pub mod task;

pub use secret::SecretManagerHelper;
pub use storage::StorageHelper;
pub use task::{CloudTaskHelper, TaskHelper};

#[derive(Debug)]
struct Error {
    message: String,
}

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}
