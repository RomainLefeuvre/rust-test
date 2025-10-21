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

    // Print the first 10 origins with their struct values
    println!("\nFirst 10 origins:");
    for (i, origin) in origins.iter_mut().take(10).enumerate() {
        println!("Origin #{} {{", i + 1);
        println!("  id: {},", origin.id);
        println!("  latest_commit_date: {:?},", origin.latest_commit_date);
        println!("  number_of_commits: {:?},", origin.number_of_commits);
        println!("  number_of_commiters: {:?},", origin.number_of_commiters);
        println!("}}");
        println!(); // Empty line for better readability
    }

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
    //         let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
    //             .expect("Invalid timestamp");
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
