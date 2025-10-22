use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use swh_graph::{graph::{SwhGraphWithProperties, SwhLabeledForwardGraph, SwhUnidirectionalGraph}, mph::DynMphf, properties};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, error};
use tracing_subscriber::fmt::init;
use clap::Parser;

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

    pub fn create_router(&self) -> Router {
        Router::new()
            .route("/health", get(health_check))
            .route("/origins", get(get_origins_ids::<G>))
            .route("/origins/:id/url", get(get_origin_url::<G>))
            .route("/origins/:id/latest-commit-date", get(get_latest_commit_date::<G>))
            .route("/origins/:id/committer-count", get(get_committer_count::<G>))
            .route("/origins/:id/commit-count", get(get_commit_count::<G>))
            .layer(CorsLayer::permissive())
            .with_state(self.graph.clone())
    }
}

// Fonction pour créer et lancer le serveur avec le type concret
pub async fn create_server() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = ServerArgs::parse();
    
    // Initialize tracing
    init();
    
    info!("Starting SWH Graph API server...");
    info!("Configuration:");
    info!("  Host: {}", args.host);
    info!("  Port: {}", args.port);
    info!("  Graph path: {}", args.graph_path);
    info!("  Data path: {}", args.data_path);
    
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
    
    // Create router
    let app = server.create_router();
    
    // Start server with the provided host and port
    let bind_address = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    info!("Server listening on http://{}", bind_address);
    info!("Available endpoints:");
    info!("  GET /health - Health check");
    info!("  GET /origins - Get all origin IDs");
    info!("  GET /origins/:id/url - Get origin URL");
    info!("  GET /origins/:id/latest-commit-date - Get latest commit date");
    info!("  GET /origins/:id/committer-count - Get committer count");
    info!("  GET /origins/:id/commit-count - Get commit count");
    
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
            let ids: Vec<usize> = origins
                .iter_mut()
                .filter(|o| o.total_commit_latest_snp_read_only().unwrap_or(0) > 0)
                .map(|o| o.id())
                .collect();
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
