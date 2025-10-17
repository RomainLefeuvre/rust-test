use std::rc::Rc;
use swh_graph::mph::DynMphf;
use swh_graph::properties::{MappedContents, MappedLabelNames, MappedMaps, MappedPersons, MappedStrings, MappedTimestamps};
use swh_graph::{NodeType, SwhGraphProperties};
use swh_graph::graph::{NodeId, SwhBidirectionalGraph, SwhForwardGraph, SwhGraphWithProperties};

/// Represents an origin node in the Software Heritage graph
pub struct Origin <G>
where
    G: SwhForwardGraph {
    /// Internal node ID of the origin
    id: usize,
    /// Reference-counted pointer to the graph containing this origin
     graph: Rc<G> ,
}

impl <G> Origin<G>
where
    G: SwhForwardGraph {
    /// Create a new Origin from a node ID and graph reference
    pub fn new(id: usize, graph:Rc<G>    ) -> Self {
        Origin { id, graph }
    }
    
    /// Get the internal node ID of this origin
    pub fn id(&self) -> usize {
        self.id
    }
    
    /// Get the URL of this origin from the graph properties
    pub fn get_url(&self) -> Option<String> {
        self.graph.
        let props = self.graph.properties();
        
        // Verify this is actually an origin node
        if props.node_type(self.id) != NodeType::Origin {
            return None;
        }
        
        // For origin nodes, the URL is stored in the message field
        props.message(self.id)
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    }
    
    /// Get the SWHID string for this origin
    pub fn swhid(&self) -> String {
        let props = self.graph.properties();
        props.swhid(self.id).to_string()
    }

    pub fn get_latest_snapshot(&self) -> Option<(NodeId, u64)>{
        let props = self.graph.properties();
        if props.node_type(self.id) != NodeType::Origin {
            return None;
        }
        return swh_graph_stdlib::find_latest_snp(self.graph.as_ref(), self.id).ok().flatten();
    }

    
}

impl std::fmt::Debug for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Origin")
            .field("id", &self.id)
            .field("url", &self.get_url())
            .finish()
    }
}

impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_url() {
            Some(url) => write!(f, "Origin({})", url),
            None => write!(f, "Origin(id={})", self.id),
        }
    }
}
