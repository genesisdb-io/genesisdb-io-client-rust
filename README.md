# Rust SDK

This is the official Rust SDK for GenesisDB, an awesome and production ready event store database system for building event-driven apps.

## GenesisDB Advantages

* Incredibly fast when reading, fast when writing
* Easy backup creation and recovery
* [CloudEvents](https://cloudevents.io/) compatible
* GDPR-ready
* Easily accessible via the HTTP interface
* Auditable. Guarantee database consistency
* Logging and metrics for Prometheus
* SQL like query language called GenesisDB Query Language (GDBQL)
* ...

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
genesisdb = "1.0.0"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
futures = "0.3"
```

or just run:

```
cargo add genesisdb
```

## Configuration

### Environment Variables
The following environment variables are required:
```
GENESISDB_AUTH_TOKEN=<secret>
GENESISDB_API_URL=http://localhost:8080
GENESISDB_API_VERSION=v1
```

### Basic Setup

```rust
use genesisdb_io_client::{Client, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the GenesisDB client
    let client = Client::from_env()?;

    Ok(())
}
```

## Streaming Events

### Basic Event Streaming

```rust
// Stream all events for a subject
let events = client.stream_events("/customer", None).await?;
```

### Stream Events from Lower Bound

```rust
use genesisdb_io_client::StreamOptions;

let events = client.stream_events("/", Some(StreamOptions {
    lower_bound: Some("2d6d4141-6107-4fb2-905f-445730f4f2a9".to_string()),
    include_lower_bound_event: Some(true),
    ..Default::default()
})).await?;
```

### Stream Latest Events by Event Type

```rust
use genesisdb_io_client::StreamOptions;

let events = client.stream_events("/", Some(StreamOptions {
    latest_by_event_type: Some("io.genesisdb.app.customer-updated".to_string()),
    ..Default::default()
})).await?;
```

This feature allows you to stream only the latest event of a specific type for each subject. Useful for getting the current state of entities.

## Committing Events

### Basic Event Committing

```rust
use genesisdb_io_client::CommitEvent;
use serde_json::json;

client.commit_events(vec![
    CommitEvent {
        source: "io.genesisdb.app".to_string(),
        subject: "/customer".to_string(),
        event_type: "io.genesisdb.app.customer-added".to_string(),
        data: json!({
            "firstName": "Bruce",
            "lastName": "Wayne",
            "emailAddress": "bruce.wayne@enterprise.wayne"
        }),
        options: None,
    },
    CommitEvent {
        source: "io.genesisdb.app".to_string(),
        subject: "/customer".to_string(),
        event_type: "io.genesisdb.app.customer-added".to_string(),
        data: json!({
            "firstName": "Alfred",
            "lastName": "Pennyworth",
            "emailAddress": "alfred.pennyworth@enterprise.wayne"
        }),
        options: None,
    },
    CommitEvent {
        source: "io.genesisdb.store".to_string(),
        subject: "/article".to_string(),
        event_type: "io.genesisdb.store.article-added".to_string(),
        data: json!({
            "name": "Tumbler",
            "color": "black",
            "price": 2990000.00
        }),
        options: None,
    },
    CommitEvent {
        source: "io.genesisdb.app".to_string(),
        subject: "/customer/fed2902d-0135-460d-8605-263a06308448".to_string(),
        event_type: "io.genesisdb.app.customer-personaldata-changed".to_string(),
        data: json!({
            "firstName": "Angus",
            "lastName": "MacGyver",
            "emailAddress": "angus.macgyer@phoenix.foundation"
        }),
        options: None,
    },
], None).await?;
```

## Preconditions

Preconditions allow you to enforce certain checks on the server before committing events. GenesisDB supports multiple precondition types:

### isSubjectNew
Ensures that a subject is new (has no existing events):

```rust
use genesisdb_io_client::{CommitEvent, Precondition};
use serde_json::json;

client.commit_events(vec![
    CommitEvent {
        source: "io.genesisdb.app".to_string(),
        subject: "/user/456".to_string(),
        event_type: "io.genesisdb.app.user-created".to_string(),
        data: json!({
            "firstName": "John",
            "lastName": "Doe",
            "email": "john.doe@example.com"
        }),
        options: None,
    }
], Some(vec![
    Precondition {
        precondition_type: "isSubjectNew".to_string(),
        payload: json!({
            "subject": "/user/456"
        }),
    }
])).await?;
```

### isSubjectExisting
Ensures that events exist for the specified subject:

```rust
use genesisdb_io_client::{CommitEvent, Precondition};
use serde_json::json;

client.commit_events(vec![
    CommitEvent {
        source: "io.genesisdb.app".to_string(),
        subject: "/user/456".to_string(),
        event_type: "io.genesisdb.app.user-created".to_string(),
        data: json!({
            "firstName": "John",
            "lastName": "Doe",
            "email": "john.doe@example.com"
        }),
        options: None,
    }
], Some(vec![
    Precondition {
        precondition_type: "isSubjectExisting".to_string(),
        payload: json!({
            "subject": "/user/456"
        }),
    }
])).await?;
```

### isQueryResultTrue
Evaluates a query and ensures the result is truthy. Supports the full GDBQL feature set including complex WHERE clauses, aggregations, and calculated fields.

**Basic uniqueness check:**
```rust
use genesisdb_io_client::{CommitEvent, Precondition};
use serde_json::json;

client.commit_events(vec![
    CommitEvent {
        source: "io.genesisdb.app".to_string(),
        subject: "/user/456".to_string(),
        event_type: "io.genesisdb.app.user-created".to_string(),
        data: json!({
            "firstName": "John",
            "lastName": "Doe",
            "email": "john.doe@example.com"
        }),
        options: None,
    }
], Some(vec![
    Precondition {
        precondition_type: "isQueryResultTrue".to_string(),
        payload: json!({
            "query": "STREAM e FROM events WHERE e.data.email == 'john.doe@example.com' MAP COUNT() == 0"
        }),
    }
])).await?;
```

**Business rule enforcement (transaction limits):**
```rust
use genesisdb_io_client::{CommitEvent, Precondition};
use serde_json::json;

client.commit_events(vec![
    CommitEvent {
        source: "io.genesisdb.banking".to_string(),
        subject: "/user/123/transactions".to_string(),
        event_type: "io.genesisdb.banking.transaction-processed".to_string(),
        data: json!({
            "amount": 500.00,
            "currency": "EUR"
        }),
        options: None,
    }
], Some(vec![
    Precondition {
        precondition_type: "isQueryResultTrue".to_string(),
        payload: json!({
            "query": "STREAM e FROM events WHERE e.subject UNDER '/user/123' AND e.type == 'transaction-processed' AND e.time >= '2024-01-01T00:00:00Z' MAP SUM(e.data.amount) + 500 <= 10000"
        }),
    }
])).await?;
```

**Complex validation with aggregations:**
```rust
use genesisdb_io_client::{CommitEvent, Precondition};
use serde_json::json;

client.commit_events(vec![
    CommitEvent {
        source: "io.genesisdb.events".to_string(),
        subject: "/conference/2024/registrations".to_string(),
        event_type: "io.genesisdb.events.registration-created".to_string(),
        data: json!({
            "attendeeId": "att-789",
            "ticketType": "premium"
        }),
        options: None,
    }
], Some(vec![
    Precondition {
        precondition_type: "isQueryResultTrue".to_string(),
        payload: json!({
            "query": "STREAM e FROM events WHERE e.subject UNDER '/conference/2024/registrations' AND e.type == 'registration-created' GROUP BY e.data.ticketType HAVING e.data.ticketType == 'premium' MAP COUNT() < 50"
        }),
    }
])).await?;
```

**Supported GDBQL Features in Preconditions:**
- WHERE conditions with AND/OR/IN/BETWEEN operators
- Hierarchical subject queries (UNDER, DESCENDANTS)
- Aggregation functions (COUNT, SUM, AVG, MIN, MAX)
- GROUP BY with HAVING clauses
- ORDER BY and LIMIT clauses
- Calculated fields and expressions
- Nested field access (e.data.address.city)
- String concatenation and arithmetic operations

If a precondition fails, the commit returns HTTP 412 (Precondition Failed) with details about which condition failed.

## GDPR Compliance

### Store Data as Reference

```rust
use genesisdb_io_client::{CommitEvent, CommitEventOptions};
use serde_json::json;

client.commit_events(vec![
    CommitEvent {
        source: "io.genesisdb.app".to_string(),
        subject: "/user/456".to_string(),
        event_type: "io.genesisdb.app.user-created".to_string(),
        data: json!({
            "firstName": "John",
            "lastName": "Doe",
            "email": "john.doe@example.com"
        }),
        options: Some(CommitEventOptions {
            store_data_as_reference: Some(true),
        }),
    }
], None).await?;
```

### Delete Referenced Data

```rust
client.erase_data("/user/456").await?;
```

## Observing Events

### Basic Event Observation

```rust
use futures::StreamExt;

let mut stream = client.observe_events("/customer", None).await?;

while let Some(result) = stream.next().await {
    match result {
        Ok(event) => println!("Received event: {:?}", event),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Observe Events from Lower Bound (Message Queue)

```rust
use futures::StreamExt;
use genesisdb_io_client::StreamOptions;

let mut stream = client.observe_events("/customer", Some(StreamOptions {
    lower_bound: Some("2d6d4141-6107-4fb2-905f-445730f4f2a9".to_string()),
    include_lower_bound_event: Some(true),
    ..Default::default()
})).await?;

while let Some(result) = stream.next().await {
    match result {
        Ok(event) => println!("Received event: {:?}", event),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Observe Latest Events by Event Type (Message Queue)

```rust
use futures::StreamExt;
use genesisdb_io_client::StreamOptions;

let mut stream = client.observe_events("/customer", Some(StreamOptions {
    latest_by_event_type: Some("io.genesisdb.app.customer-updated".to_string()),
    ..Default::default()
})).await?;

while let Some(result) = stream.next().await {
    match result {
        Ok(event) => println!("Received latest event: {:?}", event),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Querying Events

```rust
let results = client.query_events(r#"STREAM e FROM events WHERE e.type == "io.genesisdb.app.customer-added" ORDER BY e.time DESC LIMIT 20 MAP { subject: e.subject, firstName: e.data.firstName }"#).await?;
println!("Query results: {:?}", results);
```

## Health Checks

```rust
// Check API status
let ping_response = client.ping().await?;
println!("Ping response: {}", ping_response);

// Run audit to check event consistency
let audit_response = client.audit().await?;
println!("Audit response: {}", audit_response);
```

## License

MIT

## Author

* E-Mail: mail@genesisdb.io
* URL: https://www.genesisdb.io
* Docs: https://docs.genesisdb.io
