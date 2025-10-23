use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Json, Response},
    routing::get,
    Router,
};
use axum::body::to_bytes;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use swh_graph::{graph::{SwhGraphWithProperties, SwhLabeledForwardGraph, SwhUnidirectionalGraph}, mph::DynMphf, properties};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, error, debug};
use tracing_subscriber::fmt::init;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc as StdArc;
use crate::graph::{Graph, SerializationFormat};

/// CLI arguments for the SWH Graph API server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct ServerArgs {
    /// Port to bind the server to
    #[arg(short, long, default_value = "5000")]
    pub port: u16,

    /// Path to the graph data directory
    #[arg(short, long)]
    pub graph_path: String,

    /// Path to store cached data
    #[arg(short, long, default_value = "./data")]
    pub data_path: String,

    /// Host to bind the server to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Enable debug mode to log all HTTP requests
    #[arg(short, long)]
    pub log: bool,
}

// Struct pour encapsuler le serveur avec le type générique
pub struct GraphServer<G>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    graph: Arc<RwLock<Graph<G>>>,
}

impl<G> GraphServer<G>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    pub fn new(graph: Graph<G>) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
        }
    }

    pub fn create_router(&self, debug_mode: bool) -> Router {
        let mut router = Router::new()
            .route("/health", get(health_check))
            .route("/origins", get(get_origins_ids::<G>))
            .route("/origins/latest-commit-dates", get(get_all_latest_commit_dates::<G>))
            .route("/origins/commit-counts", get(get_all_commit_counts::<G>))
            .route("/origins/:id/url", get(get_origin_url::<G>))
            .route("/origins/:id/latest-commit-date", get(get_latest_commit_date::<G>))
            .route("/origins/:id/committer-count", get(get_committer_count::<G>))
            .route("/origins/:id/commit-count", get(get_commit_count::<G>))
            .layer(CorsLayer::permissive())
            .with_state(self.graph.clone());

        if debug_mode {
            router = router.layer(middleware::from_fn(log_requests_and_responses));
        }

        router
    }
}

// Custom middleware to log requests and responses including body content
async fn log_requests_and_responses(
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    debug!("Request: {} {}", method, uri);
    
    let response = next.run(request).await;
    let latency = start.elapsed();
    let status = response.status();
    
    // Extract the response body to log it
    let (parts, body) = response.into_parts();
    
    // Convert body to bytes
    match to_bytes(body, usize::MAX).await {
        Ok(bytes) => {
            // Try to parse as JSON for pretty logging
            if let Ok(json_str) = std::str::from_utf8(&bytes) {
                if let Ok(json_value) = serde_json::from_str::<Value>(json_str) {
                    debug!("Response: {} ({}ms)", status, latency.as_millis());
                    debug!("Response Body: {}", serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "Invalid JSON".to_string()));
                } else {
                    debug!("Response: {} ({}ms) - Body: {}", status, latency.as_millis(), json_str);
                }
            } else {
                debug!("Response: {} ({}ms) - Binary body ({} bytes)", status, latency.as_millis(), bytes.len());
            }
            
            // Reconstruct the response with the same body
            Response::from_parts(parts, Body::from(bytes))
        }
        Err(e) => {
            debug!("Response: {} ({}ms) - Failed to read body: {}", status, latency.as_millis(), e);
            Response::from_parts(parts, Body::empty())
        }
    }
}

// Fonction pour créer et lancer le serveur avec le type concret
pub async fn create_server() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = ServerArgs::parse();
    
    // Initialize tracing with appropriate level based on debug mode
    if args.log {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        init();
    }
    
    info!("Starting SWH Graph API server...");
    info!("Configuration:");
    info!("  Host: {}", args.host);
    info!("  Port: {}", args.port);
    info!("  Graph path: {}", args.graph_path);
    info!("  Data path: {}", args.data_path);
    info!("  Log mode: {}", args.log);
    
    // Load the graph with the provided path
    let internal_graph = SwhUnidirectionalGraph::new(&args.graph_path)?
        .load_all_properties::<DynMphf>()?
        .load_labels()?;
    
    let mut graph = Graph::with_serialization_format(
        &args.data_path,
        internal_graph,
        SerializationFormat::Bincode,
    );
    
    info!("Loading origins...");
    graph.get_origins_mut()?;
    
    // Créer le serveur avec le type concret
    let server = GraphServer::new(graph);
    
    // Create router with debug mode
    let app = server.create_router(args.log);
    
    // Start server with the provided host and port
    let bind_address = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    info!("Server listening on http://{}", bind_address);
    info!("Available endpoints:");
    info!("  GET /health - Health check");
    info!("  GET /origins - Get all origin IDs");
    info!("  GET /origins/latest-commit-dates - Get latest commit dates for all origins");
    info!("  GET /origins/commit-counts - Get commit counts for all origins");
    info!("  GET /origins/:id/url - Get origin URL");
    info!("  GET /origins/:id/latest-commit-date - Get latest commit date");
    info!("  GET /origins/:id/committer-count - Get committer count");
    info!("  GET /origins/:id/commit-count - Get commit count");
    
    if args.log {
        info!("Debug mode enabled - all HTTP requests will be logged");
    }
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Health check endpoint
async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "service": "swh-graph-api"
    })))
}

/// GET /origins - Get all origin IDs (filtered to exclude origins with 0 commits)
async fn get_origins_ids<G>(
    State(state): State<Arc<RwLock<Graph<G>>>>
) -> Result<Json<Value>, StatusCode>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    let mut graph = state.write().await;
    
    match graph.get_origins_mut() {
        Ok(origins) => {
            info!("Processing {} origins to filter by commit count...", origins.len());
            
            // Create progress bar
            let pb = StdArc::new(ProgressBar::new(origins.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) | ETA: {eta} | Rate: {per_sec}")
                    .unwrap()
                    .progress_chars("█▉▊▋▌▍▎▏  ")
            );
            pb.set_message("Filtering origins");
            
            let mut ids: Vec<usize> = Vec::new();
            
            // Process origins with progress tracking
            for origin in origins.iter() {
                let has_commits = origin.total_commit_latest_snp_read_only().unwrap_or(0) > 0;
                let has_commit_date = origin.get_latest_commit_date_read_only().is_some();
                
                if has_commits && has_commit_date {
                    ids.push(origin.id());
                }
                
                pb.inc(1);
            }
            
            pb.finish_with_message("✅ Origin filtering completed!");
            info!("Found {} origins with commits and commit dates", ids.len());
            
            Ok(Json(json!({
                "origin_ids": ids,
                "count": ids.len()
            })))
        }
        Err(e) => {
            error!("Failed to get origins: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /origins/:id/url - Get URL for a specific origin
async fn get_origin_url<G>(
    Path(id): Path<usize>,
    State(state): State<Arc<RwLock<Graph<G>>>>
) -> Result<Json<Value>, StatusCode>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    let mut graph = state.write().await;
    
    match graph.get_origins_mut() {
        Ok(origins) => {
            if let Some(origin) = origins.iter_mut().find(|o| o.id() == id) {
                let url = origin.get_url();
                Ok(Json(json!({
                    "origin_id": id,
                    "url": url
                })))
            } else {
                error!("Origin with id {} not found", id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!("Failed to get origins: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /origins/:id/latest-commit-date - Get latest commit date for a specific origin
async fn get_latest_commit_date<G>(
    Path(id): Path<usize>,
    State(state): State<Arc<RwLock<Graph<G>>>>
) -> Result<Json<Value>, StatusCode>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    let mut graph = state.write().await;
    
    match graph.get_origins_mut() {
        Ok(origins) => {
            if let Some(origin) = origins.iter_mut().find(|o| o.id() == id) {
                let latest_date = origin.get_latest_commit_date();
                Ok(Json(json!({
                    "origin_id": id,
                    "latest_commit_date": latest_date
                })))
            } else {
                error!("Origin with id {} not found", id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!("Failed to get origins: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /origins/:id/committer-count - Get committer count for a specific origin
async fn get_committer_count<G>(
    Path(id): Path<usize>,
    State(state): State<Arc<RwLock<Graph<G>>>>
) -> Result<Json<Value>, StatusCode>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    let mut graph = state.write().await;
    
    match graph.get_origins_mut() {
        Ok(origins) => {
            if let Some(origin) = origins.iter_mut().find(|o| o.id() == id) {
                let committer_count = origin.total_commiter_latest_snp();
                Ok(Json(json!({
                    "origin_id": id,
                    "committer_count": committer_count
                })))
            } else {
                error!("Origin with id {} not found", id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!("Failed to get origins: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /origins/:id/commit-count - Get commit count for a specific origin
async fn get_commit_count<G>(
    Path(id): Path<usize>,
    State(state): State<Arc<RwLock<Graph<G>>>>
) -> Result<Json<Value>, StatusCode>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    let mut graph = state.write().await;
    
    match graph.get_origins_mut() {
        Ok(origins) => {
            if let Some(origin) = origins.iter_mut().find(|o| o.id() == id) {
                let commit_count = origin.total_commit_latest_snp();
                Ok(Json(json!({
                    "origin_id": id,
                    "commit_count": commit_count
                })))
            } else {
                error!("Origin with id {} not found", id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!("Failed to get origins: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /origins/latest-commit-dates - Get latest commit dates for all origins
async fn get_all_latest_commit_dates<G>(
    State(state): State<Arc<RwLock<Graph<G>>>>
) -> Result<Json<HashMap<String, String>>, StatusCode>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    info!("Fetching latest commit dates for all origins");
    
    let mut graph = state.write().await;
    
    match graph.get_origins_mut() {
        Ok(origins) => {
            let total_origins = origins.len();
            
            // Create progress bar for processing all origins
            let pb = StdArc::new(ProgressBar::new(total_origins as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  {spinner:.cyan} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} origins ({percent}%) {msg}")
                    .unwrap()
                    .progress_chars("=>-")
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.set_message("Processing latest commit dates...");
            
            let mut result: HashMap<String, String> = HashMap::new();
            
            for (idx, origin) in origins.iter_mut().enumerate() {
                if let Some(latest_commit_date) = origin.get_latest_commit_date() {
                    result.insert(origin.id().to_string(), latest_commit_date.to_string());
                }
                
                pb.set_position((idx + 1) as u64);
                
                // Update message with current progress
                if idx % 100 == 0 || idx == total_origins - 1 {
                    pb.set_message(format!("Processed {}/{} origins", idx + 1, total_origins));
                }
            }
            
            pb.finish_with_message(format!("✓ Completed processing {} origins with latest commit dates", result.len()));
            
            info!("Successfully retrieved latest commit dates for {} out of {} origins", result.len(), total_origins);
            Ok(Json(result))
        }
        Err(e) => {
            error!("Failed to get origins: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /origins/commit-counts - Get commit counts for all origins
async fn get_all_commit_counts<G>(
    State(state): State<Arc<RwLock<Graph<G>>>>
) -> Result<Json<HashMap<String, String>>, StatusCode>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync + 'static,
{
    info!("Fetching commit counts for all origins");
    
    let mut graph = state.write().await;
    
    match graph.get_origins_mut() {
        Ok(origins) => {
            let total_origins = origins.len();
            
            // Create progress bar for processing all origins
            let pb = StdArc::new(ProgressBar::new(total_origins as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  {spinner:.cyan} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} origins ({percent}%) {msg}")
                    .unwrap()
                    .progress_chars("=>-")
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.set_message("Processing commit counts...");
            
            let mut result: HashMap<String, String> = HashMap::new();
            
            for (idx, origin) in origins.iter_mut().enumerate() {
                if let Some(commit_count) = origin.total_commit_latest_snp() {
                    result.insert(origin.id().to_string(), commit_count.to_string());
                }
                
                pb.set_position((idx + 1) as u64);
                
                // Update message with current progress
                if idx % 100 == 0 || idx == total_origins - 1 {
                    pb.set_message(format!("Processed {}/{} origins", idx + 1, total_origins));
                }
            }
            
            pb.finish_with_message(format!("✓ Completed processing {} origins with commit counts", result.len()));
            
            info!("Successfully retrieved commit counts for {} out of {} origins", result.len(), total_origins);
            Ok(Json(result))
        }
        Err(e) => {
            error!("Failed to get origins: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
