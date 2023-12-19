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
        task: Vec<u8>,
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
        task: Vec<u8>,
        headers: Option<HashMap<String, String>>,
        name: Option<String>,
        schedule_time: Option<DateTime<Utc>>,
        oidc_token: Option<OidcToken>,
    ) -> Task {
        let http_request = HttpRequest {
            url: Some(service.to_owned()),
            body: Some(task),
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
