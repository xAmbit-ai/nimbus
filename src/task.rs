use crate::NimbusError;
use chrono::{DateTime, Utc};
use google_cloudtasks2::api::{CreateTaskRequest, HttpRequest, OidcToken, Task};
use google_cloudtasks2::{oauth2::authenticator::Authenticator, CloudTasks};
use hyper::client::HttpConnector;
use hyper::{Body, Response};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error: {0}")]
    Other(String),
    #[error("CloudTasks error: {0}")]
    CloudTasks(#[from] google_cloudtasks2::Error),
}

#[async_trait::async_trait]
pub trait TaskHelper {
    /// Create a new Task
    fn new_task(
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
}

/// CloudTaskHelper trait
/// implemented for CloudTasks<HttpsConnector<HttpConnector>>
#[async_trait::async_trait]
pub trait CloudTaskHelper<S> {
    /// Create a new CloudTasks with an Authenticator
    async fn new_with_authenticator(authenticator: Authenticator<S>) -> Self;

    /// Push a task to a queue without creating a task first
    #[allow(clippy::too_many_arguments)]
    async fn push(
        &self,
        queue: &str,
        service: &str,
        method: &str,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
        name: Option<String>,
        schedule_time: Option<DateTime<Utc>>,
        oidc_token: Option<OidcToken>,
        res_view: Option<String>,
    ) -> Result<(Response<Body>, Task), NimbusError> {
        let task = Task::new_task(
            service,
            method,
            body,
            headers,
            name,
            schedule_time,
            oidc_token,
        );

        self.push_task(queue, task, res_view).await
    }

    /// Push a task to a queue, takes a Task
    async fn push_task(
        &self,
        queue: &str,
        task: Task,
        res_view: Option<String>,
    ) -> Result<(Response<Body>, Task), NimbusError>;
}

impl TaskHelper for Task {}

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

    async fn push_task(
        &self,
        queue: &str,
        task: Task,
        res_view: Option<String>,
    ) -> Result<(Response<Body>, Task), NimbusError> {
        let rq = CreateTaskRequest {
            task: Some(task),
            response_view: res_view,
        };

        let a = self
            .projects()
            .locations_queues_tasks_create(rq, queue)
            .doit()
            .await
            .map_err(|e| Error::CloudTasks(e))?;

        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::{Authenticator, CloudTaskHelper, CloudTasks, HashMap, Task, Utc};
    use google_auth_helper::helper::AuthHelper;
    #[tokio::test]
    async fn test_new_http_task() {
        use super::TaskHelper;
        let date = Utc::now();
        let task = Task::new_task(
            "https://example.com",
            "POST",
            None,
            Some(HashMap::new()),
            Some("test".to_owned()),
            Some(date),
            None,
        );

        assert_eq!(
            task.clone().http_request.unwrap().url.unwrap(),
            "https://example.com"
        );
        assert_eq!(
            task.clone().http_request.unwrap().http_method.unwrap(),
            "POST"
        );
        assert_eq!(task.clone().name.unwrap(), "test");
        assert_eq!(task.clone().schedule_time.unwrap(), date);
    }

    #[tokio::test]
    async fn cloud_task_helper() {
        use super::TaskHelper;
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
            h.insert(
                "Content-Type".to_owned(),
                "application/json; charset=UTF-8".to_owned(),
            );
            h
        };
        let queue = std::env::var("QUEUE").unwrap();
        let time_now = Utc::now();
        let time_now_int = time_now.timestamp();
        // xor shift algo
        let random_num =
            time_now_int ^ (time_now_int << 13) ^ (time_now_int >> 17) ^ (time_now_int << 5);
        let task_name = queue.clone() + "/tasks/test_task_" + &random_num.to_string();

        let task = Task::new_task(
            "https://jsonplaceholder.typicode.com/posts",
            "POST",
            Some(body.clone()),
            Some(headers.clone()),
            Some(task_name),
            None,
            None,
        );

        let (res, _task) = client.push_task(&queue, task, None).await.unwrap();

        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn cloud_task_helper_push() {
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
            h.insert(
                "Content-Type".to_owned(),
                "application/json; charset=UTF-8".to_owned(),
            );
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

        assert_eq!(res.status(), 200);
    }
}
