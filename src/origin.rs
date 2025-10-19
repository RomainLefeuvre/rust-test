use std::rc::Rc;
use swh_graph::{NodeType};
use swh_graph::graph::{NodeId, SwhFullGraph, SwhGraphWithProperties};

/// Represents an origin node in the Software Heritage graph
pub struct Origin <G>
where
    G: SwhFullGraph {
    /// Internal node ID of the origin
    id: usize,
    /// Reference-counted pointer to the graph containing this origin
     graph: Rc<G> ,
}

impl <G> Origin<G>
where
    G: SwhFullGraph {
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


    pub fn get_latest_commit_date(&self) -> Option<u64>{
        //iterate over get_all_latest_snapshots_revisions and get the max commit date
        let revisions = self.get_all_latest_snapshots_revisions();
        let mut max_date:Option<u64> = None;
        for rev in revisions {
            let props = self.graph.properties();
            let commit_date = props.committer_timestamp(rev);
            if let Some(date) = commit_date {
                if let Some(max) = max_date {
                    if date > max.try_into().unwrap() {
                        max_date = Some(date.try_into().unwrap());
                    }
                } 
            }
    }
                return max_date;

}

    //Get all head revision of the latest snapshots
    pub fn get_all_latest_snapshots_revisions(&self) -> Vec<NodeId>{
        let latest_snapshots:(NodeId, u64) = self.get_latest_snapshot().unwrap();
        let mut revisions:Vec<NodeId> = Vec::new();
        for succ in self.graph.successors(latest_snapshots.0) {
             let node_type = self.graph.properties().node_type(succ);
                if node_type == NodeType::Revision {
                    revisions.push(succ );
                }else if node_type == NodeType::Release {
                    //get all revisions linked to this release
                    for rel_succ in self.graph.successors(succ) {
                        let rel_node_type = self.graph.properties().node_type(rel_succ);
                        if rel_node_type == NodeType::Revision {
                            revisions.push(rel_succ );
                        }
                    }
                }
                else{
                    //print the type for debugging
                    println!("Successor {} is of type {:?}", succ, node_type);
                }
                
        }
        return revisions;
        
    }

    
}

impl <G> std::fmt::Debug for Origin<G> where
    G: SwhFullGraph,
 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Origin")
            .field("id", &self.id)
            .field("url", &self.get_url())
            .finish()
    }
}

impl <G> std::fmt::Display for Origin<G> where
    G: SwhFullGraph,
 {    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_url() {
            Some(url) => write!(f, "Origin({})", url),
            None => write!(f, "Origin(id={})", self.id),
        }
    }
}


