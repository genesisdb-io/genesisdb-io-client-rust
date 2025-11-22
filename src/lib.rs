//! Genesis DB Client SDK for Rust
//!
//! This crate provides a client for interacting with Genesis DB, an event sourcing database.
//!
//! # Example
//!
//! ```no_run
//! use genesisdb_io_client::{Client, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new(ClientConfig {
//!         api_url: "http://localhost:8080".to_string(),
//!         api_version: "v1".to_string(),
//!         auth_token: "your-token".to_string(),
//!     })?;
//!
//!     // Ping the server
//!     let response = client.ping().await?;
//!     println!("Ping response: {}", response);
//!
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod types;

pub use client::{Client, ClientConfig};
pub use error::{Error, Result};
pub use types::*;
