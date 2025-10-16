use std::rc::Rc;
use swh_graph::NodeType;
use swh_graph::graph::SwhGraphWithProperties;
use crate::graph::GraphType;

/// Represents an origin node in the Software Heritage graph
pub struct Origin {
    /// Internal node ID of the origin
    id: usize,
    /// Reference-counted pointer to the graph containing this origin
    graph: Rc<GraphType>,
}

impl Origin {
    /// Create a new Origin from a node ID and graph reference
    pub fn new(id: usize, graph: Rc<GraphType>) -> Self {
        Origin { id, graph }
    }
    
    /// Get the internal node ID of this origin
    pub fn id(&self) -> usize {
        self.id
    }
    
    /// Get the URL of this origin from the graph properties
    pub fn get_url(&self) -> Option<String> {
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
