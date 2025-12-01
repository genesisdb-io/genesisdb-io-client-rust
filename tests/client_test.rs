//! Unit tests for the GenesisDB client using mockito

use genesisdb_io_client::{Client, ClientConfig, CommitEvent, CommitEventOptions, Precondition, StreamOptions};
use mockito::{Matcher, Server};
use serde_json::json;

fn create_test_client(server_url: &str) -> Client {
    Client::new(ClientConfig {
        api_url: server_url.to_string(),
        api_version: "v1".to_string(),
        auth_token: "test-token".to_string(),
    })
    .unwrap()
}

#[tokio::test]
async fn test_ping_success() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/status/ping")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_body("pong")
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.ping().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "pong");
}

#[tokio::test]
async fn test_ping_error() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/status/ping")
        .with_status(503)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.ping().await;

    mock.assert_async().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_audit_success() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/status/audit")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_body("Audit successful")
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.audit().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Audit successful");
}

#[tokio::test]
async fn test_stream_events_success() {
    let mut server = Server::new_async().await;

    let event = json!({
        "id": "1",
        "source": "test",
        "type": "test.event",
        "subject": "/test",
        "specversion": "1.0",
        "data": { "message": "test" }
    });

    let mock = server
        .mock("POST", "/api/v1/stream")
        .match_header("authorization", "Bearer test-token")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(json!({ "subject": "/test" })))
        .with_status(200)
        .with_body(format!("{}\n", event))
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.stream_events("/test", None).await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "1");
    assert_eq!(events[0].event_type, "test.event");
}

#[tokio::test]
async fn test_stream_events_with_options() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/stream")
        .match_body(Matcher::Json(json!({
            "subject": "/test",
            "options": {
                "lowerBound": "123",
                "includeLowerBoundEvent": true,
                "latestByEventType": "test.type"
            }
        })))
        .with_status(200)
        .with_body("")
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let options = StreamOptions {
        lower_bound: Some("123".to_string()),
        include_lower_bound_event: Some(true),
        latest_by_event_type: Some("test.type".to_string()),
    };
    let result = client.stream_events("/test", Some(options)).await;

    mock.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_stream_events_empty_response() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/stream")
        .with_status(200)
        .with_body("")
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.stream_events("/test", None).await;

    mock.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_stream_events_multiple_events() {
    let mut server = Server::new_async().await;

    let event1 = json!({
        "id": "1",
        "source": "test",
        "type": "test.event",
        "subject": "/test1",
        "specversion": "1.0",
        "data": { "n": 1 }
    });
    let event2 = json!({
        "id": "2",
        "source": "test",
        "type": "test.event",
        "subject": "/test2",
        "specversion": "1.0",
        "data": { "n": 2 }
    });

    let mock = server
        .mock("POST", "/api/v1/stream")
        .with_status(200)
        .with_body(format!("{}\n{}\n", event1, event2))
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.stream_events("/test", None).await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, "1");
    assert_eq!(events[1].id, "2");
}

#[tokio::test]
async fn test_stream_events_api_error() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/stream")
        .with_status(500)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.stream_events("/test", None).await;

    mock.assert_async().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_commit_events_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/commit")
        .match_header("authorization", "Bearer test-token")
        .match_body(Matcher::Json(json!({
            "events": [{
                "source": "test.source",
                "subject": "/test/subject",
                "type": "test.event.created",
                "data": { "name": "Test Event" }
            }]
        })))
        .with_status(200)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client
        .commit_events(
            vec![CommitEvent {
                source: "test.source".to_string(),
                subject: "/test/subject".to_string(),
                event_type: "test.event.created".to_string(),
                data: json!({ "name": "Test Event" }),
                options: None,
            }],
            None,
        )
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_commit_events_with_options() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/commit")
        .match_body(Matcher::Json(json!({
            "events": [{
                "source": "test.source",
                "subject": "/test/subject",
                "type": "test.event.created",
                "data": { "name": "Test Event" },
                "options": { "storeDataAsReference": true }
            }]
        })))
        .with_status(200)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client
        .commit_events(
            vec![CommitEvent {
                source: "test.source".to_string(),
                subject: "/test/subject".to_string(),
                event_type: "test.event.created".to_string(),
                data: json!({ "name": "Test Event" }),
                options: Some(CommitEventOptions {
                    store_data_as_reference: Some(true),
                }),
            }],
            None,
        )
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_commit_events_with_preconditions() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/commit")
        .match_body(Matcher::Json(json!({
            "events": [{
                "source": "test.source",
                "subject": "/test/subject",
                "type": "test.event.created",
                "data": { "name": "Test Event" }
            }],
            "preconditions": [{
                "type": "isSubjectNew",
                "payload": { "subject": "/test/subject" }
            }]
        })))
        .with_status(200)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client
        .commit_events(
            vec![CommitEvent {
                source: "test.source".to_string(),
                subject: "/test/subject".to_string(),
                event_type: "test.event.created".to_string(),
                data: json!({ "name": "Test Event" }),
                options: None,
            }],
            Some(vec![Precondition {
                precondition_type: "isSubjectNew".to_string(),
                payload: json!({ "subject": "/test/subject" }),
            }]),
        )
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_commit_events_api_error() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/commit")
        .with_status(400)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client
        .commit_events(
            vec![CommitEvent {
                source: "test".to_string(),
                subject: "/test".to_string(),
                event_type: "test.event".to_string(),
                data: json!({}),
                options: None,
            }],
            None,
        )
        .await;

    mock.assert_async().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_erase_data_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/erase")
        .match_header("authorization", "Bearer test-token")
        .match_body(Matcher::Json(json!({ "subject": "/test/subject" })))
        .with_status(200)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.erase_data("/test/subject").await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_erase_data_api_error() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/erase")
        .with_status(404)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.erase_data("/test/subject").await;

    mock.assert_async().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_query_success() {
    let mut server = Server::new_async().await;

    let result1 = json!({ "id": "1", "name": "Result 1" });
    let result2 = json!({ "id": "2", "name": "Result 2" });

    let mock = server
        .mock("POST", "/api/v1/q")
        .match_header("authorization", "Bearer test-token")
        .match_body(Matcher::Json(json!({ "query": "FROM e IN events PROJECT INTO e.data" })))
        .with_status(200)
        .with_body(format!("{}\n{}\n", result1, result2))
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.q("FROM e IN events PROJECT INTO e.data").await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["id"], "1");
    assert_eq!(results[1]["id"], "2");
}

#[tokio::test]
async fn test_query_empty_results() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/q")
        .with_status(200)
        .with_body("")
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.q("FROM e IN events WHERE false").await;

    mock.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_query_api_error() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/v1/q")
        .with_status(400)
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.q("INVALID QUERY").await;

    mock.assert_async().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_query_events_calls_q() {
    let mut server = Server::new_async().await;

    let result1 = json!({ "id": "1", "name": "Result" });

    let mock = server
        .mock("POST", "/api/v1/q")
        .match_body(Matcher::Json(json!({ "query": "FROM e IN events" })))
        .with_status(200)
        .with_body(format!("{}\n", result1))
        .create_async()
        .await;

    let client = create_test_client(&server.url());
    let result = client.query_events("FROM e IN events").await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 1);
}
