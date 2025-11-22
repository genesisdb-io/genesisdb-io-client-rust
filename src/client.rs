//! Genesis DB client implementation

use crate::error::{Error, Result};
use crate::types::*;
use futures::stream::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde_json::Value;
use std::env;
use std::pin::Pin;

/// Configuration for the Genesis DB client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// API URL (e.g., "http://localhost:8080")
    pub api_url: String,
    /// API version (e.g., "v1")
    pub api_version: String,
    /// Authentication token
    pub auth_token: String,
}

impl ClientConfig {
    /// Create a new configuration from environment variables
    ///
    /// Required environment variables:
    /// - `GENESISDB_API_URL`
    /// - `GENESISDB_API_VERSION`
    /// - `GENESISDB_AUTH_TOKEN`
    pub fn from_env() -> Result<Self> {
        let api_url = env::var("GENESISDB_API_URL")
            .map_err(|_| Error::EnvError("GENESISDB_API_URL not set".to_string()))?;
        let api_version = env::var("GENESISDB_API_VERSION")
            .map_err(|_| Error::EnvError("GENESISDB_API_VERSION not set".to_string()))?;
        let auth_token = env::var("GENESISDB_AUTH_TOKEN")
            .map_err(|_| Error::EnvError("GENESISDB_AUTH_TOKEN not set".to_string()))?;

        Ok(Self {
            api_url,
            api_version,
            auth_token,
        })
    }
}

/// Genesis DB client
#[derive(Debug, Clone)]
pub struct Client {
    config: ClientConfig,
    http_client: reqwest::Client,
}

impl Client {
    /// Create a new Genesis DB client with the given configuration
    pub fn new(config: ClientConfig) -> Result<Self> {
        if config.api_url.is_empty() {
            return Err(Error::MissingConfig("api_url".to_string()));
        }
        if config.api_version.is_empty() {
            return Err(Error::MissingConfig("api_version".to_string()));
        }
        if config.auth_token.is_empty() {
            return Err(Error::MissingConfig("auth_token".to_string()));
        }

        let http_client = reqwest::Client::new();

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Create a new Genesis DB client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = ClientConfig::from_env()?;
        Self::new(config)
    }

    fn build_url(&self, path: &str) -> String {
        format!(
            "{}/api/{}/{}",
            self.config.api_url, self.config.api_version, path
        )
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.auth_token)
    }

    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&self.auth_header()).unwrap(),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("genesisdb-sdk"));
        headers
    }

    /// Ping the Genesis DB server
    ///
    /// Returns "pong" if the server is healthy
    pub async fn ping(&self) -> Result<String> {
        let url = self.build_url("status/ping");

        let mut headers = self.default_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));

        let response = self
            .http_client
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                status_text: response.status().canonical_reason().unwrap_or("Unknown").to_string(),
            });
        }

        Ok(response.text().await?)
    }

    /// Get audit information from the Genesis DB server
    pub async fn audit(&self) -> Result<String> {
        let url = self.build_url("status/audit");

        let mut headers = self.default_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));

        let response = self
            .http_client
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                status_text: response.status().canonical_reason().unwrap_or("Unknown").to_string(),
            });
        }

        Ok(response.text().await?)
    }

    /// Stream events for a given subject
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject to stream events for
    /// * `options` - Optional streaming options
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genesisdb_io_client::{Client, ClientConfig, StreamOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Client::new(ClientConfig {
    /// #     api_url: "http://localhost:8080".to_string(),
    /// #     api_version: "v1".to_string(),
    /// #     auth_token: "token".to_string(),
    /// # })?;
    /// let events = client.stream_events("/user/123", None).await?;
    /// for event in events {
    ///     println!("Event: {:?}", event);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stream_events(
        &self,
        subject: &str,
        options: Option<StreamOptions>,
    ) -> Result<Vec<CloudEvent>> {
        let url = self.build_url("stream");

        let mut headers = self.default_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/x-ndjson"));

        let request_body = StreamRequest {
            subject: subject.to_string(),
            options,
        };

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                status_text: response.status().canonical_reason().unwrap_or("Unknown").to_string(),
            });
        }

        let text = response.text().await?;

        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let events: Vec<CloudEvent> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Commit events to Genesis DB
    ///
    /// # Arguments
    ///
    /// * `events` - Events to commit
    /// * `preconditions` - Optional preconditions to check before committing
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genesisdb_io_client::{Client, ClientConfig, CommitEvent};
    /// # use serde_json::json;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Client::new(ClientConfig {
    /// #     api_url: "http://localhost:8080".to_string(),
    /// #     api_version: "v1".to_string(),
    /// #     auth_token: "token".to_string(),
    /// # })?;
    /// client.commit_events(
    ///     vec![CommitEvent {
    ///         source: "io.genesisdb.app".to_string(),
    ///         subject: "/user/123".to_string(),
    ///         event_type: "io.genesisdb.app.user-created".to_string(),
    ///         data: json!({ "name": "John" }),
    ///         options: None,
    ///     }],
    ///     None,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn commit_events(
        &self,
        events: Vec<CommitEvent>,
        preconditions: Option<Vec<Precondition>>,
    ) -> Result<()> {
        let url = self.build_url("commit");

        let mut headers = self.default_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let internal_events: Vec<CommitEventInternal> = events
            .into_iter()
            .map(|e| CommitEventInternal {
                source: e.source,
                subject: e.subject,
                event_type: e.event_type,
                data: e.data,
                options: e.options,
            })
            .collect();

        let request_body = CommitRequest {
            events: internal_events,
            preconditions,
        };

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                status_text: response.status().canonical_reason().unwrap_or("Unknown").to_string(),
            });
        }

        Ok(())
    }

    /// Erase data for a subject (GDPR compliance)
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject to erase data for
    pub async fn erase_data(&self, subject: &str) -> Result<()> {
        let url = self.build_url("erase");

        let mut headers = self.default_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let request_body = EraseRequest {
            subject: subject.to_string(),
        };

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                status_text: response.status().canonical_reason().unwrap_or("Unknown").to_string(),
            });
        }

        Ok(())
    }

    /// Execute a query against Genesis DB
    ///
    /// # Arguments
    ///
    /// * `query` - The query string to execute
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genesisdb_io_client::{Client, ClientConfig};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Client::new(ClientConfig {
    /// #     api_url: "http://localhost:8080".to_string(),
    /// #     api_version: "v1".to_string(),
    /// #     auth_token: "token".to_string(),
    /// # })?;
    /// let results = client.q("FROM e IN events WHERE e.type == 'user-created' TOP 10").await?;
    /// for result in results {
    ///     println!("{}", result);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn q(&self, query: &str) -> Result<Vec<Value>> {
        let url = self.build_url("q");

        let mut headers = self.default_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/x-ndjson"));

        let request_body = QueryRequest {
            query: query.to_string(),
        };

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                status_text: response.status().canonical_reason().unwrap_or("Unknown").to_string(),
            });
        }

        let text = response.text().await?;

        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let results: Vec<Value> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Query events (alias for `q`)
    ///
    /// # Arguments
    ///
    /// * `query` - The query string to execute
    pub async fn query_events(&self, query: &str) -> Result<Vec<Value>> {
        self.q(query).await
    }

    /// Observe events for a given subject
    ///
    /// Returns a stream of CloudEvents that will yield events as they are received
    /// from the server in real-time.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject to observe events for
    /// * `options` - Optional streaming options
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use genesisdb_io_client::{Client, ClientConfig};
    /// # use futures::StreamExt;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Client::new(ClientConfig {
    /// #     api_url: "http://localhost:8080".to_string(),
    /// #     api_version: "v1".to_string(),
    /// #     auth_token: "token".to_string(),
    /// # })?;
    /// let mut stream = client.observe_events("/user/123", None).await?;
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(event) => println!("Event: {:?}", event),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn observe_events(
        &self,
        subject: &str,
        options: Option<StreamOptions>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CloudEvent>> + Send>>> {
        let url = self.build_url("observe");

        let mut headers = self.default_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/x-ndjson"));

        let request_body = StreamRequest {
            subject: subject.to_string(),
            options,
        };

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                status_text: response.status().canonical_reason().unwrap_or("Unknown").to_string(),
            });
        }

        let byte_stream = response.bytes_stream();

        let event_stream = async_stream::stream! {
            let mut buffer = String::new();

            futures::pin_mut!(byte_stream);

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let text = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&text);

                        while let Some(newline_idx) = buffer.find('\n') {
                            let line = buffer[..newline_idx].trim().to_string();
                            buffer = buffer[newline_idx + 1..].to_string();

                            if line.is_empty() {
                                continue;
                            }

                            // Handle SSE format with "data: " prefix
                            let json_str = if line.starts_with("data: ") {
                                &line[6..]
                            } else {
                                &line
                            };

                            // Skip heartbeat messages
                            if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                                if parsed.get("payload") == Some(&Value::String(String::new()))
                                   && parsed.as_object().map(|o| o.len()) == Some(1) {
                                    continue;
                                }
                            }

                            match serde_json::from_str::<CloudEvent>(json_str) {
                                Ok(event) => yield Ok(event),
                                Err(e) => yield Err(Error::JsonError(e)),
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(Error::RequestError(e));
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(event_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_validation() {
        // Valid config
        let config = ClientConfig {
            api_url: "http://localhost:8080".to_string(),
            api_version: "v1".to_string(),
            auth_token: "token".to_string(),
        };
        assert!(Client::new(config).is_ok());

        // Empty api_url
        let config = ClientConfig {
            api_url: "".to_string(),
            api_version: "v1".to_string(),
            auth_token: "token".to_string(),
        };
        assert!(matches!(Client::new(config), Err(Error::MissingConfig(_))));

        // Empty api_version
        let config = ClientConfig {
            api_url: "http://localhost:8080".to_string(),
            api_version: "".to_string(),
            auth_token: "token".to_string(),
        };
        assert!(matches!(Client::new(config), Err(Error::MissingConfig(_))));

        // Empty auth_token
        let config = ClientConfig {
            api_url: "http://localhost:8080".to_string(),
            api_version: "v1".to_string(),
            auth_token: "".to_string(),
        };
        assert!(matches!(Client::new(config), Err(Error::MissingConfig(_))));
    }

    #[test]
    fn test_build_url() {
        let config = ClientConfig {
            api_url: "http://localhost:8080".to_string(),
            api_version: "v1".to_string(),
            auth_token: "token".to_string(),
        };
        let client = Client::new(config).unwrap();

        assert_eq!(
            client.build_url("status/ping"),
            "http://localhost:8080/api/v1/status/ping"
        );
        assert_eq!(
            client.build_url("stream"),
            "http://localhost:8080/api/v1/stream"
        );
    }

    #[test]
    fn test_auth_header() {
        let config = ClientConfig {
            api_url: "http://localhost:8080".to_string(),
            api_version: "v1".to_string(),
            auth_token: "my-secret-token".to_string(),
        };
        let client = Client::new(config).unwrap();

        assert_eq!(client.auth_header(), "Bearer my-secret-token");
    }
}
