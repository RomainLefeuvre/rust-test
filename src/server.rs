use axum::{
    extract::State,
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

use crate::graph::{Graph, SerializationFormat};

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
            .layer(CorsLayer::permissive())
            .with_state(self.graph.clone())
    }
}

// Fonction pour créer et lancer le serveur avec le type concret
pub async fn create_server() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init();
    
    info!("Starting SWH Graph API server...");
    
    let graph_path = "/home/sandbox/graph/partial/graph";
    
    // Load the graph exactly like in main.rs
    let internal_graph = SwhUnidirectionalGraph::new(graph_path)?
        .load_all_properties::<DynMphf>()?
        .load_labels()?;
    
    let mut graph = Graph::with_serialization_format(
        "./data",
        internal_graph,
        SerializationFormat::Bincode,
    );
    
    info!("Loading origins...");
    graph.get_origins_mut()?;
    
    // Créer le serveur avec le type concret
    let server = GraphServer::new(graph);
    
    // Create router
    let app = server.create_router();
    
    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("Server listening on http://127.0.0.1:3000");
    info!("Available endpoints:");
    info!("  GET /health");
    info!("  GET /origins - Get all origin IDs");
    
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

/// GET /origins - Get all origin IDs
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
            let ids: Vec<usize> = origins.iter().map(|o| o.id()).collect();
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
