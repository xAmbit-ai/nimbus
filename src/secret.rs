use super::Error;
use google_secretmanager1::{oauth2::authenticator::Authenticator, SecretManager};
use hyper::client::HttpConnector;
use hyper::Client;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

#[async_trait::async_trait]
pub trait SecretManagerHelper<S> {
    async fn new_with_authenticator(authenticator: Authenticator<S>) -> Self;

    async fn get_secret(
        &self,
        project: &str,
        secret: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

    async fn get_secret_version(
        &self,
        project: &str,
        secret: &str,
        version: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

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

    async fn get_secret(
        &self,
        project: &str,
        secret: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let secret_name = format!("projects/{}/secrets/{}/versions/latest", project, secret);
        let (_r, s) = self
            .projects()
            .secrets_versions_access(&secret_name)
            .doit()
            .await?;

        let secret = if let Some(pl) = s.payload {
            if let Some(data) = pl.data {
                data
            } else {
                return Err(Error::new("No data in payload").into());
            }
        } else {
            return Err(Error::new("No payload").into());
        };

        Ok(secret)
    }

    async fn get_secret_version(
        &self,
        project: &str,
        secret: &str,
        version: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let secret_name = format!(
            "projects/{}/secrets/{}/versions/{}",
            project, secret, version
        );
        let (_, s) = self
            .projects()
            .secrets_versions_access(&secret_name)
            .doit()
            .await?;

        let secret = if let Some(pl) = s.payload {
            if let Some(data) = pl.data {
                data
            } else {
                return Err(Error::new("No data in payload: {}").into());
            }
        } else {
            return Err(Error::new("No payload").into());
        };

        Ok(secret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use google_auth_helper::helper::AuthHelper;

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
