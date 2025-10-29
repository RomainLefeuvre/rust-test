use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use swh_graph::properties::{self};
use swh_graph::{graph::*, NodeType };
use crate::utils::filter_by_node_type;
use crate::origin::{Origin, OriginData};
use serde_json;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use rand::seq::{IndexedRandom, SliceRandom};

#[derive(Clone, Copy, Debug)]
pub enum SerializationFormat {
    Json,
    Bincode,
}


pub struct Graph<G>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync,
{
    graph: Arc<G>,
    #[allow(dead_code)]
    base_path: PathBuf,
    origins_cache_file: PathBuf,
    origins: Option<Vec<Origin<G>>>,
    serialization_format: SerializationFormat,
} 

impl <G> Graph<G>
where
    G: SwhLabeledForwardGraph 
    + SwhGraphWithProperties<
        Maps: properties::Maps,
        Timestamps: properties::Timestamps,
        Persons: properties::Persons,
        Contents: properties::Contents,
        Strings: properties::Strings,
        LabelNames: properties::LabelNames,
    > + Send + Sync,
{  
    pub fn new<P: Into<PathBuf>>(graph_path: P, graph: G) -> Self {
        Self::with_serialization_format(graph_path, graph, SerializationFormat::Json)
    }
    
    pub fn with_serialization_format<P: Into<PathBuf>>(
        graph_path: P, 
        graph: G, 
        format: SerializationFormat
    ) -> Self {
        let base_path: PathBuf = graph_path.into();

        let mut origins_cache_file = base_path.clone();
        let extension = match format {
            SerializationFormat::Json => "origins.json",
            SerializationFormat::Bincode => "origins.bin",
        };
        origins_cache_file.set_file_name(extension);

        Graph {
            graph: Arc::new(graph),
            base_path,
            origins_cache_file,
            origins: None,
            serialization_format: format,
        }
    }
    
    /// Get graph statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.graph.num_nodes(), self.graph.num_arcs().try_into().unwrap())
    }
    
    
    /// Get origins, automatically loading if not already loaded
    /// Returns a reference to the Vec of Origin objects
    #[allow(dead_code)]
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
            println!("Loading origins from cache ({:?}): {:?}", 
                     self.serialization_format, self.origins_cache_file);
            match self.load_origins_from_file() {
                Ok(()) => {
                    println!("Successfully loaded {} origins from cache", 
                             self.origins.as_ref().map_or(0, |o| o.len()));
                }
                Err(e) => {
                    eprintln!("Failed to load origins from cache: {}. Recomputing...", e);
                    // Delete the corrupted cache file
                    let _ = fs::remove_file(&self.origins_cache_file);
                    // Recompute origins
                    self.origins = Some(self.compute_origins());
                     if let Err(e) = self.save_origins_to_file() {
                eprintln!("Failed to save origins to cache: {}", e);
            }
                }
            }
        } else {
            println!("Computing origins and caching to ({:?}): {:?}", 
                     self.serialization_format, self.origins_cache_file);
            self.origins= Some(self.compute_origins());
            if let Err(e) = self.save_origins_to_file() {
                eprintln!("Failed to save origins to cache: {}", e);
            }
        }
    }
    
    fn load_origins_from_file(&mut self) -> Result<(), std::io::Error> {
        let file = File::open(&self.origins_cache_file)?;
        let reader = BufReader::new(file);
        
        // Deserialize the Origin objects (without graph reference)
        let origins_data: Vec<OriginData> = match self.serialization_format {
            SerializationFormat::Json => {
                serde_json::from_reader(reader)
                    .map_err(|e| {
                        eprintln!("Error deserializing JSON: {}", e);
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                    })?
            }
            SerializationFormat::Bincode => {
                bincode::deserialize_from(reader)
                    .map_err(|e| {
                        eprintln!("Error deserializing Bincode: {}", e);
                        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Bincode error: {}", e))
                    })?
            }
        };
        
        //map to Origin<G> by setting the graph reference
        let origins: Vec<Origin<G>> = origins_data.into_iter()
            .map(|data| Origin::from_data(data, self.graph.clone()))
            .collect();
        self.origins = Some(origins);
        Ok(())
    }

 pub fn filter_n_first_origins(&mut self, max_size: usize) {
    if let Some(origins) = &mut self.origins {
        if origins.len() > max_size {
            origins.truncate(max_size);
        }
    }
}

    
    pub fn save_origins_to_file(&self) -> Result<(), std::io::Error> {
        let file = File::create(&self.origins_cache_file)?;
        let writer = BufWriter::new(file);
        
        // Convert Origins to OriginData for serialization
        if let Some(origins) = &self.origins {
            let origins_data: Vec<OriginData> = origins.iter()
                .map(|origin| OriginData {
                    id: origin.id,
                    url: origin.url.clone(),
                    latest_commit_date: origin.latest_commit_date,
                    number_of_commits: origin.number_of_commits,
                    number_of_commiters: origin.number_of_commiters,
                })
                .collect();
            
            // Serialize the origins data using the chosen format
            match self.serialization_format {
                SerializationFormat::Json => {
                    serde_json::to_writer_pretty(writer, &origins_data)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                }
                SerializationFormat::Bincode => {
                    bincode::serialize_into(writer, &origins_data)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                }
            }
            
        } else {
            Ok(())
        }
    }
    
    /// Save n random origins to file instead of all origins
    /// Useful for testing and reducing file sizes
    pub fn save_n_random_origins_to_file(&self, n: usize) -> Result<(), std::io::Error> {
        let mut cache_file = self.origins_cache_file.clone();
        
        // Modify filename to include the number of origins
        let base_name = cache_file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("origins");
        let extension = cache_file.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");
        
        let new_filename = format!("{}_random_{}.{}", base_name, n, extension);
        cache_file.set_file_name(new_filename);
        
        let file = File::create(&cache_file)?;
        let writer = BufWriter::new(file);
        
        // Convert Origins to OriginData for serialization
        if let Some(origins) = &self.origins {
            // Select n random origins
            let mut rng = rand::thread_rng();
            let selected_origins: Vec<&Origin<G>> = origins
                .choose_multiple(&mut rng, n.min(origins.len()))
                .collect();
            
            let origins_data: Vec<OriginData> = selected_origins.iter()
                .map(|origin| OriginData {
                    id: origin.id,
                    url: origin.url.clone(),
                    latest_commit_date: origin.latest_commit_date,
                    number_of_commits: origin.number_of_commits,
                    number_of_commiters: origin.number_of_commiters,
                })
                .collect();
            
            println!("Saving {} random origins out of {} total to: {:?}", 
                     origins_data.len(), origins.len(), cache_file);
            
            // Serialize the origins data using the chosen format
            match self.serialization_format {
                SerializationFormat::Json => {
                    serde_json::to_writer_pretty(writer, &origins_data)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                }
                SerializationFormat::Bincode => {
                    bincode::serialize_into(writer, &origins_data)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                }
            }
            
        } else {
            Ok(())
        }
    }
    
    fn compute_origins(&self) -> Vec<Origin<G>> {
        let origin_ids = filter_by_node_type(&self.graph, NodeType::Origin);
        
        // Create progress bar
        let pb = Arc::new(ProgressBar::new(origin_ids.len() as u64));
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        pb.set_message("Computing origins");
        
        let origins: Vec<Origin<G>> = origin_ids.par_iter()
            .filter_map(|&id| {
                let mut origin = Origin::new(id, self.graph.clone());
                pb.inc(1);
                
                // Filter out origins that don't have a latest snapshot
                if origin.get_latest_snapshot().is_some() {
                    Some(origin)
                } else {
                    None
                }
            })
            .collect();
            
        pb.finish_with_message("Origins computed! Check logs for count with snapshots");
        println!("Found {} origins with snapshots out of {} total", origins.len(), origin_ids.len());
        origins
    }


}


