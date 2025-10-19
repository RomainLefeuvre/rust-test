use std::path::PathBuf;
use std::fs;
use swh_graph::{graph::{self, *}, properties, NodeType, };
use swh_graph::mph::DynMphf;
use crate::utils::{filter_by_node_type, write_node_ids, read_node_ids};

// Type alias for the graph type with properties only (no labels)
type GraphType = SwhBidirectionalGraph<
    properties::SwhGraphProperties<
        properties::MappedMaps<DynMphf>,
        properties::MappedTimestamps,
        properties::MappedPersons,
        properties::MappedContents,
        properties::MappedStrings,
        properties::MappedLabelNames,
    >,
>;

pub struct Graph {
    graph: GraphType,
    base_path: PathBuf,
    origins_cache_file: PathBuf,
    origins: Option<Vec<usize>>,
} 

impl Graph {
    /// Create a new Graph object from a graph path
    pub fn new<P: Into<PathBuf>>(graph_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let base_path = graph_path.into();
        // Load the graph with all properties but without labels
        let graph = SwhBidirectionalGraph::new(&base_path)?
            .load_all_properties::<DynMphf>()?;


        let mut origins_cache_file = base_path.clone();
        origins_cache_file.set_file_name("origins.bin");
        Ok(Graph {
            graph,
            base_path,
            origins_cache_file,
            origins: None,
        })
    }
    
    /// Get graph statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.graph.num_nodes(), self.graph.num_arcs().try_into().unwrap())
    }
    
    /// Load origins, using cache if available
    pub fn load_origins(&mut self) -> Result<&Vec<usize>, Box<dyn std::error::Error>> {
        if self.origins.is_none() {
            self.origins = Some(self.get_or_compute_origins()?);
        }
        Ok(self.origins.as_ref().unwrap())
    }
     /// Get origins, automatically loading if not already loaded
    pub fn get_origins(&mut self) -> Result<&Vec<usize>, Box<dyn std::error::Error>> {
        if self.origins.is_none() {
            self.origins = Some(self.get_or_compute_origins()?);
        }
        Ok(self.origins.as_ref().unwrap())
    }
    
    /// Get origins (cached) without loading if not already loaded
    pub fn get_origins_cached(&self) -> Option<&Vec<usize>> {
        self.origins.as_ref()
    }

    
    /// Get the underlying graph reference
    pub fn inner_graph(&self) -> &GraphType {
        &self.graph
    }
    
    // Private helper methods
    fn get_or_compute_origins(&self) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        if fs::exists(&self.origins_cache_file).unwrap_or(false) {
            println!("Loading origins from cache: {:?}", self.origins_cache_file);
            Ok(read_node_ids(&self.origins_cache_file)?)
        } else {
            println!("Computing origins and caching to: {:?}", self.origins_cache_file);
            let origins = self.compute_origins();
            write_node_ids(&self.origins_cache_file, &origins)?;
            Ok(origins)
        }
    }
    
    fn compute_origins(&self) -> Vec<usize> {
        filter_by_node_type(&self.graph, NodeType::Origin)
    }
}


