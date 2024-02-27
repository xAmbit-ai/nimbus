use aws_config::BehaviorVersion;

#[cfg(feature = "gcp")]
use google_secretmanager1::{
    api::{AddSecretVersionRequest, Automatic, Replication, Secret, SecretPayload},
    hyper::{client::HttpConnector, Client},
    hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
    oauth2::authenticator::Authenticator,
    SecretManager,
};

#[cfg(feature = "aws")]
use aws_sdk_secretsmanager::Client;

use thiserror::Error;

use crate::NimbusError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No data in payload from AccessSecretVersionResponse")]
    NoData,
    #[error("No payload in AccessSecretVersionResponse")]
    NoPayload,
    #[error("Error: {0}")]
    Other(String),
    #[cfg(feature = "gcp")]
    #[error("SecretManager error: {0}")]
    SecretManager(#[from] google_secretmanager1::Error),
    #[cfg(feature = "aws")]
    #[error("SecretManager error: {0}")]
    SecretManager(String),
}

/// SecretManagerHelper trait
/// implemented for SecretManager<HttpsConnector<HttpConnector>>
#[async_trait::async_trait]
pub trait SecretManagerHelper<S> {
    /// Create a new SecretManager with an Authenticator
    /// Deals with boilerplate of creating a new SecretManager
    #[cfg(feature = "gcp")]
    async fn new_with_authenticator(authenticator: Authenticator<S>) -> Self;

    /// Get the latest version of a secret
    async fn get_secret(&self, project: &str, secret: &str) -> Result<Vec<u8>, NimbusError>;

    /// Creates a new secret
    async fn create_secret(
        &self,
        project: &str,
        secret_name: &str,
        secret_val: &str,
    ) -> Result<(), NimbusError>;

    /// Get a specific version of a secret
    async fn get_secret_version(
        &self,
        project: &str,
        secret: &str,
        version: &str,
    ) -> Result<Vec<u8>, NimbusError>;
}

#[cfg(feature = "aws")]
pub async fn secret_manager_client() -> Client {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    Client::new(&config)
}

#[cfg(feature = "aws")]
#[async_trait::async_trait]
impl SecretManagerHelper<()> for aws_sdk_secretsmanager::Client {
    async fn get_secret(&self, _: &str, secret: &str) -> Result<Vec<u8>, NimbusError> {
        let res = match self.get_secret_value().secret_id(secret).send().await {
            Ok(res) => {
                if let Some(data) = res.secret_string {
                    data
                } else {
                    return Err(NimbusError::from(Error::SecretManager(
                        "invalid secret".to_string(),
                    )));
                }
            }
            Err(e) => return Err(NimbusError::from(Error::SecretManager(e.to_string()))),
        };

        Ok(res.as_bytes().to_vec())
    }

    /// Get a specific version of a secret
    async fn get_secret_version(
        &self,
        _: &str,
        secret: &str,
        version: &str,
    ) -> Result<Vec<u8>, NimbusError> {
        let res = match self
            .get_secret_value()
            .secret_id(secret)
            .version_stage(version)
            .send()
            .await
        {
            Ok(res) => {
                if let Some(data) = res.secret_binary {
                    data
                } else {
                    return Err(NimbusError::from(Error::SecretManager(
                        "invalid secret".to_string(),
                    )));
                }
            }
            Err(e) => return Err(NimbusError::from(Error::SecretManager(e.to_string()))),
        };

        Ok(res.into_inner())
    }

    async fn create_secret(
        &self,
        _: &str,
        secret_name: &str,
        secret_val: &str,
    ) -> Result<(), NimbusError> {
        if let Err(e) = self
            .create_secret()
            .secret_string(secret_val)
            .name(secret_name)
            .send()
            .await
        {
            return Err(NimbusError::from(Error::SecretManager(e.to_string())));
        }

        Ok(())
    }
}

#[cfg(feature = "gcp")]
#[async_trait::async_trait]
impl SecretManagerHelper<HttpsConnector<HttpConnector>>
    for SecretManager<HttpsConnector<HttpConnector>>
{
    async fn new_with_authenticator(
        authenticator: Authenticator<HttpsConnector<HttpConnector>>,
    ) -> Self {
        SecretManager::new(
            Client::builder().build(
                HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .https_only()
                    .enable_http1()
                    .enable_http2()
                    .build(),
            ),
            authenticator,
        )
    }

    async fn get_secret(&self, project: &str, secret: &str) -> Result<Vec<u8>, NimbusError> {
        let secret_name = format!("projects/{}/secrets/{}/versions/latest", project, secret);
        let (_r, s) = self
            .projects()
            .secrets_versions_access(&secret_name)
            .doit()
            .await
            .map_err(Error::SecretManager)?;

        let secret = if let Some(pl) = s.payload {
            if let Some(data) = pl.data {
                data
            } else {
                return Err(Error::NoData.into());
            }
        } else {
            return Err(Error::NoPayload.into());
        };

        Ok(secret)
    }

    async fn create_secret(
        &self,
        project: &str,
        secret_name: &str,
        secret_val: &str,
    ) -> Result<(), NimbusError> {
        self.projects()
            .secrets_create(
                Secret {
                    replication: Some(Replication {
                        automatic: Some(Automatic::default()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                format!("projects/{project}").as_str(),
            )
            .secret_id(secret_name)
            .doit()
            .await
            .map_err(Error::SecretManager)?;

        let vrq = AddSecretVersionRequest {
            payload: Some(SecretPayload {
                data: Some(secret_val.as_bytes().to_vec()),
                ..Default::default()
            }),
        };

        let parent = format!("projects/{project}/secrets/{secret_name}");
        self.projects()
            .secrets_add_version(vrq, &parent)
            .doit()
            .await
            .map_err(Error::SecretManager)?;

        Ok(())
    }

    async fn get_secret_version(
        &self,
        project: &str,
        secret: &str,
        version: &str,
    ) -> Result<Vec<u8>, NimbusError> {
        let secret_name = format!(
            "projects/{}/secrets/{}/versions/{}",
            project, secret, version
        );
        let (_, s) = self
            .projects()
            .secrets_versions_access(&secret_name)
            .doit()
            .await
            .map_err(Error::SecretManager)?;

        let secret = if let Some(pl) = s.payload {
            if let Some(data) = pl.data {
                data
            } else {
                return Err(Error::NoData.into());
            }
        } else {
            return Err(Error::NoPayload.into());
        };

        Ok(secret)
    }
}

#[cfg(feature = "gcp")]
#[cfg(test)]
mod tests {
    use google_auth_helper::helper::AuthHelper;

    use super::*;

    #[tokio::test]
    async fn get_secret_test() {
        let auth = Authenticator::auth().await.unwrap();
        let secret_manager = SecretManager::new_with_authenticator(auth).await;

        let project = std::env::var("PROJECT").unwrap();
        let secret = std::env::var("SECRET_NAME").unwrap();

        let _secret = secret_manager.get_secret(&project, &secret).await.unwrap();
    }

    #[tokio::test]
    async fn get_secret_version_test() {
        let auth = Authenticator::auth().await.unwrap();
        let secret_manager = SecretManager::new_with_authenticator(auth).await;

        let project = std::env::var("PROJECT").unwrap();
        let secret = std::env::var("SECRET_NAME").unwrap();
        let version = std::env::var("SECRET_VERSION").unwrap();

        let _secret = secret_manager
            .get_secret_version(&project, &secret, &version)
            .await
            .unwrap();
    }
}

#[cfg(feature = "aws")]
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_secret_test() {
        let secret_manager = secret_manager_client().await;
        let d = secret_manager
            .get_secret("", "<some test data>")
            .await
            .expect("failed to get secret");

        print!("{}", String::from_utf8(d).expect("secret to string failed"));
    }
}
