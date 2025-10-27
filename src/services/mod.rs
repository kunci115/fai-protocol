//! Services module for FAI Protocol
//!
//! This module provides various services that handle different aspects of the application:
//! - Branch management
//! - CLI operations
//! - Security and authentication
//! - Web interface

pub mod branch_service;
pub mod cli_service;
pub mod security_service;
pub mod web_service;

// Re-export commonly used items
pub use branch_service::{BranchService, BranchInfo};
pub use cli_service::CliService;
pub use security_service::{SecurityService, SecurityConfig, UserConfig, UserKeyPair};
pub use web_service::{WebService, WebServiceConfig};