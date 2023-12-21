# Nimbus

#### Helper/utility functions for the cloud.

## Traits
 - ### CloudTaskHelper & TaskHelper (`google-cloudtasks2`)
 - ### StorageHelper (`google-cloud-storage`)
 - ### SecretManagerHelper (`google-secretmanager1`)

## Examples

### CloudTaskHelper
```rust
use nimbus::CloudTaskHelper;
use google_auth_helper::helpers::AuthHelper; // (provided by google_auth_helper)

use google_cloudtasks2::{
    oauth2::authenticator::Authenticator,
    CloudTasks,
};

#[tokio::main]
async fn main() {
    let auth = Authenticator::auth().await.unwrap();
    let client = CloudTasks::new_with_authenticator(auth).await;

    let body = "\
        {
            \"title\": \"foo task\",
            \"body\": \"bar task\",
            \"userId\": 1
        }";

    let body = body.as_bytes().to_vec();

    let headers = {
        let mut h = HashMap::new();
        h.insert("Content-Type".to_owned(), "application/json; charset=UTF-8".to_owned());
        h
    };
    
    let queue = std::env::var("QUEUE").unwrap();

    let (res, _task) = client
        .push(
            &queue,
            "https://jsonplaceholder.typicode.com/posts",
            "POST",
            Some(body.clone()),
            Some(headers.clone()),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    
    println!("{:?}", res.status());
}
```


### StorageHelper
```rust
use nimbus::StorageHelper;
use google_auth_helper::helpers::AuthHelper; // (provided by google_auth_helper)
use google_cloud_storage::client::{Client, ClientConfig};

#[tokio::main]
async fn main() {
    let config = ClientConfig::auth().await.unwrap();
    let storage = Client::new_with_config(config).await;

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
```

### SecretManagerHelper
```rust
use nimbus::SecretManagerHelper;
use google_auth_helper::helpers::AuthHelper; // (provided by google_auth_helper)
use google_secretmanager1::{SecretManager, oauth2::authenticator::Authenticator};

#[tokio::main]
async fn main() {
    let auth = Authenticator::auth().await.unwrap();
    let client = SecretManager::new_with_authenticator(auth).await;
    
    let project = std::env::var("PROJECT").unwrap();
    let secret = std::env::var("SECRET").unwrap();
    
    let secret = client.get_secret(&project, &secret).await.unwrap();
    let secret = String::from_utf8(secret);
    
    println!("{:?}", secret);
}
```