use crate::graph::Graph;


mod utils;
mod origin;
mod graph;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph_path = "/home/sandbox/graph/2024-08-23-popular-500-python/graph";
    
    // Create and load the graph
    let mut graph = Graph::< >::new(graph_path);
    
    // Print graph statistics
    let (num_nodes, num_arcs) = graph.stats();
    println!("Graph loaded with {} nodes and {} arcs", num_nodes, num_arcs);
    
    // Get origins (will automatically load if not cached)
    let origins = graph.get_origins()?;
    println!("Found {} origins", origins.len());
    
    // Print the first 10 origins with URLs
    println!("\nFirst 10 origins:");
    for origin in origins.iter().take(10) {
        if let Some(url) = origin.get_url() {
            println!("  {}:{} -> {}", origin.id(),origin.swhid(), url);
        } else {
            println!("  {}:{} -> (no URL)", origin.id(),origin.swhid());
        }
    }
    
    Ok(())
}





