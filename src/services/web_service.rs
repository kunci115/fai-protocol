//! Web service for FAI Protocol
//!
//! Provides a simple web interface for repository management including:
//! - Basic HTTP server
//! - REST API endpoints
//! - File browsing and management
//! - Repository status and operations

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

// Import required HTTP/axum types
use axum::response::Html;
use axum::Json;
use tower_http::services::ServeDir;
use axum::serve;
use axum::http::StatusCode;

/// Web service configuration
#[derive(Debug, Clone)]
pub struct WebServiceConfig {
    pub host: String,
    pub port: u16,
    pub static_dir: Option<std::path::PathBuf>,
    pub enable_auth: bool,
}

impl Default for WebServiceConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            static_dir: None,
            enable_auth: false,
        }
    }
}

/// Web service for FAI Protocol
pub struct WebService {
    config: WebServiceConfig,
    repo_path: std::path::PathBuf,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WebService {
    /// Create a new web service instance
    pub fn new<P: AsRef<Path>>(repo_path: P, config: WebServiceConfig) -> Self {
        Self {
            config,
            repo_path: repo_path.as_ref().to_path_buf(),
            server_handle: None,
        }
    }

    /// Start the web server
    pub async fn start(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        println!("Starting FAI web server on http://{}", addr);

        let repo_path = self.repo_path.clone();
        let config = self.config.clone();

        let app = Self::create_router(repo_path, config).await?;

        let server_handle = tokio::spawn(async move {
            let addr: std::net::SocketAddr = addr.parse().expect("Invalid address");
            let listener = tokio::net::TcpListener::bind(addr).await.expect("Failed to bind");
            axum::serve(listener, app).await.expect("Failed to serve");
        });

        self.server_handle = Some(server_handle);
        Ok(())
    }

    /// Stop the web server
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        Ok(())
    }

    /// Create the web router
    async fn create_router(
        repo_path: std::path::PathBuf,
        config: WebServiceConfig,
    ) -> Result<axum::Router> {
        let shared_state = Arc::new(RwLock::new(WebState::new(repo_path.clone())));

        let app = axum::Router::new()
            // API routes
            .route("/api/status", axum::routing::get(status_handler))
            .route("/api/branches", axum::routing::get(branches_handler))
            .route("/api/commits", axum::routing::get(commits_handler))
            .route("/api/files", axum::routing::get(files_handler))
            .route("/api/log", axum::routing::get(log_handler))

            // Static file serving
            .nest_service("/static", axum::routing::get_service(
                ServeDir::new(config.static_dir.unwrap_or_else(|| {
                    std::path::PathBuf::from("static")
                }))
            ))

            // Web interface
            .route("/", axum::routing::get(index_handler))
            .route("/branches", axum::routing::get(branches_page_handler))
            .route("/commits", axum::routing::get(commits_page_handler))
            .route("/files", axum::routing::get(files_page_handler))

            // Shared state
            .with_state(shared_state);

        Ok(app)
    }
}

/// Shared web state
#[derive(Debug)]
pub struct WebState {
    repo_path: std::path::PathBuf,
}

impl WebState {
    pub fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}

// API Handlers

async fn status_handler(
    axum::extract::State(state): axum::extract::State<Arc<RwLock<WebState>>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    // Get repository status
    let fai = match crate::FaiProtocol::new_at(state.repo_path()) {
        Ok(fai) => fai,
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "status": "error",
                "message": "Repository not initialized"
            })));
        }
    };

    let status = fai.get_status().unwrap_or_else(|_| {
        Vec::new()
    });

    Ok(Json(serde_json::json!({
        "status": "ok",
        "repository": {
            "path": state.repo_path().to_string_lossy(),
            "staged_files_count": status.len(),
        }
    })))
}

async fn branches_handler(
    axum::extract::State(state): axum::extract::State<Arc<RwLock<WebState>>>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    let branch_service = crate::services::branch_service::BranchService::from_repo_path(
        &state.repo_path().join(".fai")
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let branches = branch_service.list_branches().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let branches_json: Vec<_> = branches.into_iter().map(|branch| {
        serde_json::json!({
            "name": branch.name,
            "head_commit": branch.head_commit,
            "is_current": branch.is_current,
            "is_empty": branch.is_empty,
            "short_hash": branch.short_hash(),
        })
    }).collect();

    Ok(axum::Json(serde_json::json!({
        "status": "ok",
        "branches": branches_json
    })))
}

async fn commits_handler(
    axum::extract::State(state): axum::extract::State<Arc<RwLock<WebState>>>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    let database = crate::database::DatabaseManager::new(&state.repo_path().join("db.sqlite"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let commits = database.get_all_commits().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let commits_json: Vec<_> = commits.into_iter().map(|commit| {
        serde_json::json!({
            "hash": commit.hash,
            "message": commit.message,
            "timestamp": commit.timestamp,
            "parents": commit.parents,
            "is_merge": commit.is_merge,
            "short_hash": &commit.hash[..8],
        })
    }).collect();

    Ok(axum::Json(serde_json::json!({
        "status": "ok",
        "commits": commits_json
    })))
}

async fn files_handler(
    axum::extract::State(state): axum::extract::State<Arc<RwLock<WebState>>>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let state = state.read().await;

    let fai = crate::FaiProtocol::new_at(state.repo_path()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let status = fai.get_status().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let files_json: Vec<_> = status.iter().map(|file| {
        serde_json::json!({
            "path": file.0,
            "hash": file.1,
            "size": file.2,
            "size_mb": file.2 as f64 / 1_048_576.0,
        })
    }).collect();

    Ok(axum::Json(serde_json::json!({
        "status": "ok",
        "files": files_json,
        "total_count": files_json.len()
    })))
}

async fn log_handler(
    axum::extract::State(state): axum::extract::State<Arc<RwLock<WebState>>>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    // Similar to commits_handler but with more detailed log information
    commits_handler(axum::extract::State(state)).await
}

// Page Handlers

async fn index_handler() -> axum::response::Html<String> {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>FAI Protocol - Web Interface</title>
    <meta charset="utf-8">
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { border-bottom: 2px solid #333; padding-bottom: 20px; margin-bottom: 30px; }
        .nav { margin-bottom: 30px; }
        .nav a { margin-right: 20px; text-decoration: none; color: #007acc; }
        .nav a:hover { text-decoration: underline; }
        .status { background: #f5f5f5; padding: 20px; border-radius: 5px; margin-bottom: 30px; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🔮 FAI Protocol</h1>
        <p>Decentralized Version Control for Large Files</p>
    </div>

    <div class="nav">
        <a href="/">Status</a>
        <a href="/branches">Branches</a>
        <a href="/commits">Commits</a>
        <a href="/files">Files</a>
    </div>

    <div class="status" id="status">
        <h2>Repository Status</h2>
        <p>Loading repository status...</p>
    </div>

    <script>
        // Fetch repository status
        fetch('/api/status')
            .then(response => response.json())
            .then(data => {
                const statusDiv = document.getElementById('status');
                if (data.status === 'ok') {
                    statusDiv.innerHTML = `
                        <h2>Repository Status</h2>
                        <p><strong>Path:</strong> ${data.repository.path}</p>
                        <p><strong>Current Commit:</strong> ${data.repository.current_commit || 'No commits'}</p>
                        <p><strong>Staged Files:</strong> ${data.repository.staged_files_count}</p>
                        <p><strong>Modified Files:</strong> ${data.repository.modified_files_count}</p>
                        <p><strong>Untracked Files:</strong> ${data.repository.untracked_files_count}</p>
                    `;
                } else {
                    statusDiv.innerHTML = `<p style="color: red;">Error: ${data.message}</p>`;
                }
            })
            .catch(error => {
                document.getElementById('status').innerHTML = `<p style="color: red;">Error: ${error.message}</p>`;
            });
    </script>
</body>
</html>
    "#;

    axum::response::Html(html.to_string())
}

async fn branches_page_handler() -> axum::response::Html<String> {
    let html = r#"
<!DOCTYPE html>
<html><head><title>Branches</title></head>
<body><h1>Branches</h1><p>Branch management interface coming soon...</p></body></html>
    "#;
    axum::response::Html(html.to_string())
}

async fn commits_page_handler() -> axum::response::Html<String> {
    let html = r#"
<!DOCTYPE html>
<html><head><title>Commits</title></head>
<body><h1>Commits</h1><p>Commit history interface coming soon...</p></body></html>
    "#;
    axum::response::Html(html.to_string())
}

async fn files_page_handler() -> axum::response::Html<String> {
    let html = r#"
<!DOCTYPE html>
<html><head><title>Files</title></head>
<body><h1>Files</h1><p>File management interface coming soon...</p></body></html>
    "#;
    axum::response::Html(html.to_string())
}