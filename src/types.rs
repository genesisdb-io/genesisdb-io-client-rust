//! Types used by the Genesis DB client

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A CloudEvent as used by Genesis DB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEvent {
    /// Event ID
    pub id: String,

    /// Event source
    pub source: String,

    /// Event type
    #[serde(rename = "type")]
    pub event_type: String,

    /// Event subject
    pub subject: String,

    /// Event time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,

    /// Event data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// CloudEvents spec version
    #[serde(default = "default_spec_version")]
    pub specversion: String,

    /// Data content type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datacontenttype: Option<String>,
}

fn default_spec_version() -> String {
    "1.0".to_string()
}

/// Event to be committed to Genesis DB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitEvent {
    /// Event source
    pub source: String,

    /// Event subject
    pub subject: String,

    /// Event type
    #[serde(rename = "type")]
    pub event_type: String,

    /// Event data
    pub data: Value,

    /// Event options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<CommitEventOptions>,
}

/// Options for committing an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitEventOptions {
    /// Store data as reference for GDPR compliance
    #[serde(rename = "storeDataAsReference", skip_serializing_if = "Option::is_none")]
    pub store_data_as_reference: Option<bool>,
}

/// Precondition for committing events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Precondition {
    /// Precondition type
    #[serde(rename = "type")]
    pub precondition_type: String,

    /// Precondition payload
    pub payload: Value,
}

/// Options for streaming events
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamOptions {
    /// Lower bound event ID
    #[serde(rename = "lowerBound", skip_serializing_if = "Option::is_none")]
    pub lower_bound: Option<String>,

    /// Include the lower bound event in results
    #[serde(rename = "includeLowerBoundEvent", skip_serializing_if = "Option::is_none")]
    pub include_lower_bound_event: Option<bool>,

    /// Get latest event by event type
    #[serde(rename = "latestByEventType", skip_serializing_if = "Option::is_none")]
    pub latest_by_event_type: Option<String>,
}

/// Request body for streaming events
#[derive(Debug, Serialize)]
pub(crate) struct StreamRequest {
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<StreamOptions>,
}

/// Request body for committing events
#[derive(Debug, Serialize)]
pub(crate) struct CommitRequest {
    pub events: Vec<CommitEventInternal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<Vec<Precondition>>,
}

/// Internal representation of an event for committing
#[derive(Debug, Serialize)]
pub(crate) struct CommitEventInternal {
    pub source: String,
    pub subject: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<CommitEventOptions>,
}

/// Request body for erasing data
#[derive(Debug, Serialize)]
pub(crate) struct EraseRequest {
    pub subject: String,
}

/// Request body for queries
#[derive(Debug, Serialize)]
pub(crate) struct QueryRequest {
    pub query: String,
}
