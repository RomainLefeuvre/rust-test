use std::path::PathBuf;

use crate::graph::Graph;


mod utils;
mod origin;
mod graph;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph_path = "/home/sandbox/graph/2024-08-23-popular-500-python/graph";
    let base_path:PathBuf = graph_path.into();

    // Create and load the graph
    let mut graph = Graph::<>::new(graph_path,swh_graph::graph::load_full::<swh_graph::mph::DynMphf>(&base_path).unwrap());
    
    // Print graph statistics
    let (num_nodes, num_arcs) = graph.stats();
    println!("Graph loaded with {} nodes and {} arcs", num_nodes, num_arcs);
    
    // // Get origins (will automatically load if not cached)
     let origins = graph.get_origins_mut()?;
    // println!("Found {} origins", origins.len());
    
    // // Print the first 10 origins with URLs
    // println!("\nFirst 10 origins:");
    // for origin in origins.iter().take(10) {
    //     if let Some(url) = origin.get_url() {
    //         println!("  {}:{} -> {}", origin.id(),origin.swhid(), url);
    //     } else {
    //         println!("  {}:{} -> (no URL)", origin.id(),origin.swhid());
    //     }
    // }


    // //print all head revisions of the latest snapshots for the first 10 origins
    // println!("\nAll head revisions of the latest snapshots for first 10 origins:");
    // for origin in origins.iter().take(10) {
    //     let revisions = origin.get_all_latest_snapshots_revisions();
    //     if !revisions.is_empty() {
    //         println!("  Origin {}:{} -> Head Revisions of Latest Snapshot: {:?}", origin.id(),origin.swhid(), revisions);
    //     } else {
    //         println!("  Origin {}:{} -> No head revisions found for latest snapshot", origin.id(),origin.swhid());
    //     }
    // }

    // //print all the latest commit date for the first 10 origins as ISO 8601 date
    // println!("\nLatest commit dates for first 10 origins:");
    // for origin in origins.iter_mut().take(10) {
    //     if let Some(timestamp) = origin.get_latest_commit_date() {
    //         let datetime = chrono::NaiveDateTime::from_timestamp(timestamp as i64, 0);
    //         let iso_date = datetime.format("%Y-%m-%d %H:%M)").to_string();
    //         println!("  Origin {}:{} -> Latest Commit Date: {}", origin.id(),origin.swhid(), iso_date);
    //     } else {
    //         println!("  Origin {}:{} -> No commit date found", origin.id(),origin.swhid());
    //     }
    // }

    // //Count accessible revisions from the latest snapshot for the first 10 origins
    // println!("\nAccessible revisions from latest snapshot for first 10 origins:");
    // for origin in origins.iter_mut().take(10) {
    //     if let Some(count) = origin.total_commit_latest_snp() {
    //         println!("  Origin {}:{} -> Accessible Revisions from Latest Snapshot: {}", origin.id(),origin.swhid(), count);
    //     } else {
    //         println!("  Origin {}:{} -> No accessible revisions found for latest snapshot", origin.id(),origin.swhid());
    //     }   
    // }

    // //Count unique commiters from the latest snapshot for the first 10 origins
    // println!("\nUnique commiters from latest snapshot for first 10 origins:");
    // for origin in origins.iter_mut().take(10) {     
    //     if let Some(count) = origin.total_commiter_latest_snp() {
    //         println!("  Origin {}:{} -> Unique Commiters from Latest Snapshot: {}", origin.id(),origin.swhid(), count);
    //     } else {
    //         println!("  Origin {}:{} -> No unique commiters found for latest snapshot", origin.id(),origin.swhid());
    //     }   
    // }
    
    Ok(())
}





