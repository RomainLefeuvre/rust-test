use crate::graph::{Graph, SerializationFormat};
use rayon::prelude::*;
use swh_graph::{graph::SwhUnidirectionalGraph, mph::DynMphf};
use std::path::PathBuf;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use chrono;

mod graph;
mod origin;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph_path = "/mnt/graph_temp/graph";
    let _base_path: PathBuf = graph_path.into();
      // Get origins (will automatically load if not cached)

    //#swh_graph::graph::load_full::<swh_graph::mph::DynMphf>(&base_path).unwrap()
    let internal_graph = SwhUnidirectionalGraph::new(graph_path)?.load_all_properties::<DynMphf>()?.load_labels()?;
    

    // Option 2: Use Bincode serialization (faster, more compact)
    let mut graph = Graph::with_serialization_format(
        "/home/rlefeuvr/swh/rust-test/data",
        internal_graph,
        SerializationFormat::Bincode,
    );

    // Print graph statistics
    let (num_nodes, num_arcs) = graph.stats();
        
    // Get origins
    let origins = graph.get_origins_mut()?;
    println!("Found {} origins", origins.len());
    println!(
        "Graph loaded with {} nodes and {} arcs",
        num_nodes, num_arcs
    );

    // Limit to first 1000 origins for testing
    graph.filter_n_first_origins(1000);
    graph.save_origins_to_file()?;

    let origins = graph.get_origins_mut()?;
    println!("Found {} origins", origins.len());
    println!(
        "Graph loaded with {} nodes and {} arcs",
        num_nodes, num_arcs
    );

    // Print the first 10 origins with their struct values
    println!("\nFirst 100 origins:");
    for (i, origin) in origins.iter_mut().take(100).enumerate() {
        println!("Origin #{} {{", i + 1);
        println!("  id: {},", origin.id);
        println!("  latest_commit_date: {:?},", origin.latest_commit_date);
        println!("  number_of_commits: {:?},", origin.number_of_commits);
        println!("  number_of_commiters: {:?},", origin.number_of_commiters);
        println!("  url: {:?},", origin.url);
        println!("}}");
        println!(); // Empty line for better readability
    }

    //try to get the latest snapshot for all the origins and count missing ones over total
    let mut missing_count = 0;
    for origin in origins.iter_mut() {
        let latest_snapshot = origin.get_latest_snapshot();
        if latest_snapshot.is_none() {
            missing_count += 1;
        }
    }
    println!("Origins with no latest snapshot: {}/{}", missing_count, origins.len());
  



    // Compute all data with progress bar in parallel with rayon
    println!("\nComputing origin statistics in parallel...");
   
    let pb = Arc::new(ProgressBar::new(origins.len() as u64));
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({percent}%) | ETA: {eta_precise} | Rate: {per_sec}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  ")
    );
    pb.set_message("Processing origins");
    
    //origins.par_iter_mut().take(1000).for_each(|o| {
    origins.par_iter_mut().for_each(|o| {
        o.compute_data();
        pb.inc(1);
    });
    
    pb.finish_with_message("✅ All origin statistics computed successfully!");

    graph.save_origins_to_file()?;

    

    Ok(())
}
