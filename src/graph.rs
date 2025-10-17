use std::path::PathBuf;
use std::fs;
use std::rc::Rc;
use swh_graph::{graph::*, NodeType };
use crate::utils::{filter_by_node_type, write_node_ids, read_node_ids};
use crate::origin::Origin;


pub struct Graph<G>
where
    G: SwhForwardGraph, {
    graph: Rc<G>,
    base_path: PathBuf,
    origins_cache_file: PathBuf,
    origins: Option<Vec<Origin<G>>>,
} 

impl <G> Graph<G>
where
    G: SwhForwardGraph {
    /// Crée un nouveau Graph à partir du chemin du graphe
    pub fn new<P: Into<PathBuf>>(graph_path: P) -> Graph<impl SwhForwardGraph> {
        let base_path = graph_path.into();

        let mut origins_cache_file = base_path.clone();
        origins_cache_file.set_file_name("origins.bin");

        // Ici on utilise le type concret SwhUnidirectionalGraph
        let graph = swh_graph::graph::SwhUnidirectionalGraph::new(&base_path).unwrap();
        
        Graph {
            graph: Rc::new(graph),
            base_path,
            origins_cache_file,
            origins: None,
        }
    }
    
    /// Get graph statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.graph.num_nodes(), self.graph.num_arcs().try_into().unwrap())
    }
    
    
    /// Get origins, automatically loading if not already loaded
    /// Returns a reference to the Vec of Origin objects
    pub fn get_origins(&mut self) -> Result<&Vec<Origin<G>, Box<dyn std::error::Error>> {
        if self.origins.is_none() {
            let origin_ids = self.get_or_compute_origin_ids()?;
            let origins = origin_ids.iter()
                .map(|&id| Origin::new(id, self.graph.clone()))
                .collect();
            self.origins = Some(origins);
        }
        Ok(self.origins.as_ref().unwrap())
    }
    
    
    // Private helper methods
    fn get_or_compute_origin_ids(&self) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        if fs::exists(&self.origins_cache_file).unwrap_or(false) {
            println!("Loading origins from cache: {:?}", self.origins_cache_file);
            Ok(read_node_ids(&self.origins_cache_file)?)
        } else {
            println!("Computing origins and caching to: {:?}", self.origins_cache_file);
            let origins = self.compute_origin_ids();
            write_node_ids(&self.origins_cache_file, &origins)?;
            Ok(origins)
        }
    }
    
    fn compute_origin_ids(&self) -> Vec<usize> {
        filter_by_node_type(&self.graph, NodeType::Origin)
    }
}


