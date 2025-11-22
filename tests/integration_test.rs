//! Integration tests for the Genesis DB client
//!
//! These tests require a running Genesis DB server.
//! Set the following environment variables to run:
//! - GENESISDB_API_URL
//! - GENESISDB_API_VERSION
//! - GENESISDB_AUTH_TOKEN
//!
//! Or run with: GENESISDB_INTEGRATION_TESTS=1 cargo test --test integration_test

use genesisdb_io_client::{Client, ClientConfig, CommitEvent, Precondition};
use serde_json::json;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

fn should_run_integration_tests() -> bool {
    env::var("GENESISDB_INTEGRATION_TESTS").is_ok()
}

fn create_integration_client() -> Option<Client> {
    if !should_run_integration_tests() {
        return None;
    }

    let api_url = env::var("GENESISDB_API_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let api_version = env::var("GENESISDB_API_VERSION")
        .unwrap_or_else(|_| "v1".to_string());
    let auth_token = env::var("GENESISDB_AUTH_TOKEN")
        .unwrap_or_else(|_| "secret".to_string());

    Some(
        Client::new(ClientConfig {
            api_url,
            api_version,
            auth_token,
        })
        .unwrap(),
    )
}

fn get_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[tokio::test]
async fn test_integration_ping() {
    let client = match create_integration_client() {
        Some(c) => c,
        None => {
            println!("Skipping integration test: GENESISDB_INTEGRATION_TESTS not set");
            return;
        }
    };

    let result = client.ping().await;
    assert!(result.is_ok(), "Ping failed: {:?}", result.err());
    let response = result.unwrap();
    assert!(!response.is_empty());
    println!("Ping response: {}", response);
}

#[tokio::test]
async fn test_integration_audit() {
    let client = match create_integration_client() {
        Some(c) => c,
        None => {
            println!("Skipping integration test: GENESISDB_INTEGRATION_TESTS not set");
            return;
        }
    };

    let result = client.audit().await;
    assert!(result.is_ok(), "Audit failed: {:?}", result.err());
    let response = result.unwrap();
    assert!(!response.is_empty());
    println!("Audit response: {}...", &response[..response.len().min(100)]);
}

#[tokio::test]
async fn test_integration_commit_and_stream_events() {
    let client = match create_integration_client() {
        Some(c) => c,
        None => {
            println!("Skipping integration test: GENESISDB_INTEGRATION_TESTS not set");
            return;
        }
    };

    let unique_id = format!("integration-test-{}", get_timestamp());
    let subject = format!("/test/integration/{}", unique_id);

    // Commit an event
    let result = client
        .commit_events(
            vec![CommitEvent {
                source: "io.genesisdb.test.integration".to_string(),
                subject: subject.clone(),
                event_type: "io.genesisdb.test.integration.created".to_string(),
                data: json!({
                    "message": "Integration test event",
                    "uniqueId": unique_id,
                    "timestamp": get_timestamp()
                }),
                options: None,
            }],
            None,
        )
        .await;

    assert!(result.is_ok(), "Commit failed: {:?}", result.err());

    // Small delay to ensure event is committed
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Stream events back
    let result = client.stream_events(&subject, None).await;
    assert!(result.is_ok(), "Stream failed: {:?}", result.err());

    let events = result.unwrap();
    let test_event = events.iter().find(|e| {
        e.event_type == "io.genesisdb.test.integration.created"
            && e.data
                .as_ref()
                .and_then(|d| d.get("uniqueId"))
                .and_then(|v| v.as_str())
                == Some(&unique_id)
    });

    assert!(test_event.is_some(), "Integration test event should be found");
    println!("Found integration test event: {}", test_event.unwrap().id);
}

#[tokio::test]
async fn test_integration_query() {
    let client = match create_integration_client() {
        Some(c) => c,
        None => {
            println!("Skipping integration test: GENESISDB_INTEGRATION_TESTS not set");
            return;
        }
    };

    let query = r#"
        FROM e IN events
        WHERE e.source == 'io.genesisdb.test.integration'
        ORDER BY e.time DESC
        TOP 5
        PROJECT INTO { id: e.id, type: e.type, subject: e.subject }
    "#;

    let result = client.q(query).await;
    assert!(result.is_ok(), "Query failed: {:?}", result.err());

    let results = result.unwrap();
    println!("Query returned {} results", results.len());

    if !results.is_empty() {
        assert!(results[0].get("id").is_some());
        assert!(results[0].get("type").is_some());
        assert!(results[0].get("subject").is_some());
    }
}

#[tokio::test]
async fn test_integration_commit_with_preconditions() {
    let client = match create_integration_client() {
        Some(c) => c,
        None => {
            println!("Skipping integration test: GENESISDB_INTEGRATION_TESTS not set");
            return;
        }
    };

    let unique_id = format!("precondition-test-{}", get_timestamp());
    let subject = format!("/test/precondition/{}", unique_id);

    // This should succeed since it's a new subject
    let result = client
        .commit_events(
            vec![CommitEvent {
                source: "io.genesisdb.test.precondition".to_string(),
                subject: subject.clone(),
                event_type: "io.genesisdb.test.precondition.created".to_string(),
                data: json!({
                    "message": "Precondition test event",
                    "uniqueId": unique_id
                }),
                options: None,
            }],
            Some(vec![Precondition {
                precondition_type: "isSubjectNew".to_string(),
                payload: json!({ "subject": subject }),
            }]),
        )
        .await;

    assert!(
        result.is_ok(),
        "Commit with precondition failed: {:?}",
        result.err()
    );
    println!("Successfully committed event with isSubjectNew precondition");
}

#[tokio::test]
async fn test_integration_observe_events() {
    let client = match create_integration_client() {
        Some(c) => c,
        None => {
            println!("Skipping integration test: GENESISDB_INTEGRATION_TESTS not set");
            return;
        }
    };

    let subject = "/test/observe";

    // Just test that the stream starts without error
    let result = client.observe_events(subject, None).await;
    assert!(
        result.is_ok(),
        "Observe events failed: {:?}",
        result.err()
    );

    println!("Observe test completed (stream initialized successfully)");
}
