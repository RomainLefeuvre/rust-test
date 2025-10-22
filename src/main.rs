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
    let graph_path = "/home/sandbox/graph/partial/graph";
    let base_path: PathBuf = graph_path.into();
      // Get origins (will automatically load if not cached)

    //#swh_graph::graph::load_full::<swh_graph::mph::DynMphf>(&base_path).unwrap()
    let internal_graph = SwhUnidirectionalGraph::new(graph_path)?.load_all_properties::<DynMphf>()?.load_labels()?;
    

    // Option 2: Use Bincode serialization (faster, more compact)
    let mut graph = Graph::with_serialization_format(
        "./data",
        internal_graph,
        SerializationFormat::Bincode,
    );

        
    // Get origins
    let origins = graph.get_origins_mut()?;

    // Print the first 10 origins with their struct values
    println!("\nFirst 100 origins:");
    for (i, origin) in origins.iter_mut().take(10).enumerate() {
        println!("Origin #{} {{", i + 1);
        println!("  id: {},", origin.id);
        println!("  latest_commit_date: {:?},", origin.latest_commit_date);
        println!("  number_of_commits: {:?},", origin.number_of_commits);
        println!("  number_of_commiters: {:?},", origin.number_of_commiters);
        println!("  url: {:?},", origin.url);
        println!("}}");
        println!(); // Empty line for better readability
    }


    // Compute all data with progress bar in parallel with rayon
    println!("\nComputing origin attribute in parallel...");
   
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
