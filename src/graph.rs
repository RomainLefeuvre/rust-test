use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::rc::Rc;
use swh_graph::{graph::*, NodeType };
use crate::utils::filter_by_node_type;
use crate::origin::{Origin, OriginData};
use serde_json;


pub struct Graph<G>
where
    G: SwhFullGraph, {
    graph: Rc<G>,
    base_path: PathBuf,
    origins_cache_file: PathBuf,
    origins: Option<Vec<Origin<G>>>,
} 

impl <G> Graph<G>
where
    G: SwhFullGraph {
    
    pub fn new<P: Into<PathBuf>>(graph_path: P, graph: G) -> Self {
        let base_path: PathBuf = graph_path.into();

        let mut origins_cache_file = base_path.clone();
        origins_cache_file.set_file_name("origins.json");

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
    pub fn get_origins(&mut self) -> Result<&Vec<Origin<G>>, std::io::Error> {
        if self.origins.is_none() {
            self.load_or_compute_origins();
        }
        Ok(self.origins.as_ref().unwrap())
    }
    
     pub fn get_origins_mut(&mut self) -> Result<&mut Vec<Origin<G>>, std::io::Error> {
        if self.origins.is_none() {
           self.load_or_compute_origins();
        }
        Ok(self.origins.as_mut().unwrap())
    }

    
    
    // Private helper methods
    fn load_or_compute_origins(&mut self)  {
        if fs::metadata(&self.origins_cache_file).is_ok() {
            println!("Loading origins from cache: {:?}", self.origins_cache_file);
            self.load_origins_from_file()
        } else {
            println!("Computing origins and caching to: {:?}", self.origins_cache_file);
            let origins = self.compute_origins();
            self.save_origins_to_file();
            
        }
        
    }
    
    fn load_origins_from_file(&mut self)  {
        let file = File::open(&self.origins_cache_file).unwrap();
        let reader = BufReader::new(file);
        
        // Deserialize the Origin objects (without graph reference)
        let origins_data: Vec<OriginData> = serde_json::from_reader(reader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            .unwrap();
        
        //map to Origin<G> by setting the graph reference
        let origins: Vec<Origin<G>> = origins_data.into_iter()
            .map(|data| Origin::from_data(data, self.graph.clone()))
            .collect();
        self.origins = Some(origins);
   
    }
    
    pub fn save_origins_to_file(&self) -> Result<(), std::io::Error> {
        let file = File::create(&self.origins_cache_file)?;
        let writer = BufWriter::new(file);
        
        // Serialize only the IDs (Origin implements Serialize which skips the graph field)
        serde_json::to_writer_pretty(writer, &self.origins)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
    
    fn compute_origins(&self) -> Vec<Origin<G>> {
        let origin_ids = filter_by_node_type(&self.graph, NodeType::Origin);
        origin_ids.iter()
            .map(|&id| Origin::new(id, self.graph.clone()))
            .collect()
    }


}


