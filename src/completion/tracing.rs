use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug)]
pub enum TracingError {
    HttpError(String),
    Other(String),
}

impl std::fmt::Display for TracingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TracingError::HttpError(msg) => write!(f, "Tracing HttpError: {}", msg),
            TracingError::Other(msg) => write!(f, "Tracing Other: {}", msg),
        }
    }
}

// Now it implements the standard Error trait
impl std::error::Error for TracingError {}

#[async_trait]
pub trait TracingProvider: Send + Sync {
    async fn start_trace(
        &self,
        trace_id: Uuid,
        name: &str,
        trace_type: &str,
        inputs: &Value,
        parent_trace_id: Option<Uuid>,
        start_time: Option<SystemTime>,
    ) -> Result<(), TracingError>;

    async fn end_trace(
        &self,
        trace_id: Uuid,
        outputs: &Value,
        end_time: Option<SystemTime>,
    ) -> Result<(), TracingError>;
}


/// An example implementer that communicates with the LangSmith API.
pub struct LangSmithTracer {
    pub base_url: String,   // e.g. "https://api.smith.langchain.com"
    pub api_key: String,    // "x-api-key" header
    pub http_client: Client,
}

impl LangSmithTracer {
    pub fn new(api_key: String) -> Self {
        Self {
            base_url: "https://api.smith.langchain.com".to_string(),
            api_key,
            http_client: Client::new(),
        }
    }
}

#[derive(Serialize)]
struct StartRunBody {
    id: String,
    name: String,
    #[serde(rename = "run_type")]
    trace_type: String,
    inputs: Value,
    start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_run_id: Option<String>,
}

#[derive(Serialize)]
struct EndRunBody {
    outputs: Value,
    end_time: String,
}

// Optionally parse API error details
#[derive(Deserialize, Debug)]
struct ApiError {
    detail: Option<String>,
}

#[async_trait]
impl TracingProvider for LangSmithTracer {
    async fn start_trace(
        &self,
        trace_id: Uuid,
        name: &str,
        trace_type: &str,
        inputs: &Value,
        parent_trace_id: Option<Uuid>,
        start_time: Option<SystemTime>,
    ) -> Result<(), TracingError> {
        // If start_time isn't provided, use current time
        let start_time = match start_time {
            Some(t) => t,
            None => SystemTime::now(),
        };
        let start_time_str = DateTime::<Utc>::from(start_time)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let body = StartRunBody {
            id: trace_id.to_string(),
            name: name.into(),
            trace_type: trace_type.into(),
            inputs: inputs.clone(),
            start_time: start_time_str,
            parent_run_id: parent_trace_id.map(|p| p.to_string()),
        };

        let url = format!("{}/runs", self.base_url);
        let resp = self
            .http_client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| TracingError::HttpError(err.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "No response body".to_string());
            return Err(TracingError::HttpError(format!(
                "start_trace failed: HTTP {} – {}",
                status, text
            )));
        }

        Ok(())
    }

    async fn end_trace(
        &self,
        trace_id: Uuid,
        outputs: &Value,
        end_time: Option<SystemTime>,
    ) -> Result<(), TracingError> {
        let end_time = match end_time {
            Some(t) => t,
            None => SystemTime::now(),
        };
        let end_time_str = DateTime::<Utc>::from(end_time)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let body = EndRunBody {
            outputs: outputs.clone(),
            end_time: end_time_str,
        };

        let url = format!("{}/runs/{}", self.base_url, trace_id);
        let resp = self
            .http_client
            .patch(&url)
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| TracingError::HttpError(err.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "No response body".to_string());
            return Err(TracingError::HttpError(format!(
                "end_trace failed: HTTP {} – {}",
                status, text
            )));
        }

        Ok(())
    }
}