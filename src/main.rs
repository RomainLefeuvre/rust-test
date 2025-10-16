
use std::path::PathBuf;
mod utils;
mod graph;

use std::fs;
use swh_graph::graph::*;


fn main() {

    let base_name ="/home/sandbox/graph/2024-08-23-popular-500-python/graph";
    let graph = swh_graph::graph::load_full::<swh_graph::mph::DynMphf>(PathBuf::from(base_name))
    .expect("Could not load graph");
    println!("Graph loaded with {} nodes and {} arcs", graph.num_nodes(), graph.num_arcs());
    let origins_list_file = "origin.bin";


    if !fs::exists(&origins_list_file).unwrap() {
        let origins = graph::collect_origins(&graph);
        utils::write_node_ids(&PathBuf::from(&origins_list_file), &origins);
    }

    utils::read_node_ids(&PathBuf::from(origins_list_file));


}





