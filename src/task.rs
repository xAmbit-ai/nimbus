use chrono::{DateTime, Utc};
use google_cloudtasks2::api::{CreateTaskRequest, HttpRequest, OidcToken, Task};
use google_cloudtasks2::{oauth2::authenticator::Authenticator, CloudTasks};
use hyper::client::HttpConnector;
use hyper::{Body, Response};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use std::collections::HashMap;
use std::error::Error;

#[async_trait::async_trait]
pub trait CloudTaskHelper<S> {
    async fn new_with_authenticator(
        authenticator: Authenticator<S>,
    ) -> Self;

    fn new_http_task(
        service: &str,
        method: &str,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
        name: Option<String>,
        schedule_time: Option<DateTime<Utc>>,
        oidc_token: Option<OidcToken>,
    ) -> Task;
    async fn push_http_task(
        &self,
        queue: &str,
        task: Task,
        res_view: Option<String>,
    ) -> Result<(Response<Body>, Task), Box<dyn std::error::Error>>;
}

#[async_trait::async_trait]
impl CloudTaskHelper<HttpsConnector<HttpConnector>> for CloudTasks<HttpsConnector<HttpConnector>> {
    async fn new_with_authenticator(
        authenticator: Authenticator<HttpsConnector<HttpConnector>>,
    ) -> Self {
        CloudTasks::new(
            hyper::Client::builder().build(
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

    fn new_http_task(
        service: &str,
        method: &str,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
        name: Option<String>,
        schedule_time: Option<DateTime<Utc>>,
        oidc_token: Option<OidcToken>,
    ) -> Task {
        let http_request = HttpRequest {
            url: Some(service.to_owned()),
            body,
            http_method: Some(method.to_owned()),
            oidc_token,
            headers,
            ..Default::default()
        };

        Task {
            name,
            http_request: Some(http_request),
            schedule_time,
            ..Default::default()
        }
    }

    async fn push_http_task(
        &self,
        queue: &str,
        task: Task,
        res_view: Option<String>,
    ) -> Result<(Response<Body>, Task), Box<dyn Error>> {
        let rq = CreateTaskRequest {
            task: Some(task),
            response_view: res_view,
        };

        let a = self
            .projects()
            .locations_queues_tasks_create(rq, queue)
            .doit()
            .await?;

        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use google_auth_helper::helper::AuthHelper;
    #[tokio::test]
    async fn test_new_http_task() {
        let date = Utc::now();
        let task = CloudTasks::new_http_task(
            "https://example.com",
            "POST",
            None,
            Some(HashMap::new()),
            Some("test".to_owned()),
            Some(date),
            None,
        );

        assert_eq!(task.clone().http_request.unwrap().url.unwrap(), "https://example.com");
        assert_eq!(task.clone().http_request.unwrap().http_method.unwrap(), "POST");
        assert_eq!(task.clone().name.unwrap(), "test");
        assert_eq!(task.clone().schedule_time.unwrap(), date);
    }

    #[tokio::test]
    async fn cloud_task_helper() {
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

        let task = CloudTasks::new_http_task(
            "https://jsonplaceholder.typicode.com/posts",
            "POST",
            Some(body.clone()),
            Some(headers.clone()),
            None,
            None,
            None,
        );

        let queue = std::env::var("QUEUE").unwrap();

        let (res, _task) = client.push_http_task(&queue, task, None).await.unwrap();

        assert_eq!(res.status(), 200);
    }
}