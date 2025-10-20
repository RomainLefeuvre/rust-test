use serde::{Deserialize, Serialize};
use std::sync::Arc;
use swh_graph::NodeType;
use swh_graph::graph::{NodeId, SwhFullGraph, SwhGraphWithProperties};

/// Serializable data for Origin (without graph reference)
#[derive(Serialize, Deserialize)]
pub struct OriginData {
    pub id: usize,
    pub latest_commit_date: Option<usize>,
    pub number_of_commits: Option<usize>,
    pub number_of_commiters: Option<usize>,
}

/// Represents an origin node in the Software Heritage graph
#[derive(Serialize, Deserialize)]
pub struct Origin<G>
where
    G: SwhFullGraph + Send + Sync,
{
    /// Internal node ID of the origin
    id: usize,
    /// Reference-counted pointer to the graph containing this origin
    #[serde(skip)]
    graph: Option<Arc<G>>,

    latest_commit_date: Option<usize>,
    number_of_commits: Option<usize>,
    number_of_commiters: Option<usize>,
}

impl<G> Origin<G>
where
    G: SwhFullGraph + Send + Sync,
{
    /// Create a new Origin from a node ID and graph reference
    pub fn new(id: usize, graph: Arc<G>) -> Self {
        Origin {
            id: id,
            graph: Some(graph),
            latest_commit_date: None,
            number_of_commits: None,
            number_of_commiters: None,
        }
    }

    /// Set the graph reference (used after deserialization)
    #[allow(dead_code)]
    pub fn set_graph(&mut self, graph: Arc<G>) {
        self.graph = Some(graph);
    }

    pub fn get_graph(&self) -> Arc<G> {
        return self.graph.as_ref().unwrap().clone();
    }

    /// Convert Origin to OriginData (without graph reference)
    #[allow(dead_code)]
    pub fn to_data(&self) -> OriginData {
        OriginData {
            id: self.id,
            latest_commit_date: self.latest_commit_date,
            number_of_commits: self.number_of_commits,
            number_of_commiters: self.number_of_commiters,
        }
    }

    /// Create Origin from OriginData and graph reference
    pub fn from_data(data: OriginData, graph: Arc<G>) -> Self {
        Origin {
            id: data.id,
            graph: Some(graph),
            latest_commit_date: data.latest_commit_date,
            number_of_commits: data.number_of_commits,
            number_of_commiters: data.number_of_commiters,
        }
    }

    pub fn compute_data(&mut self) {
        // Compute latest commit date
        self.get_latest_commit_date();
        // Compute total number of commits
        self.total_commit_latest_snp();
        // Compute total number of commiters
        self.total_commiter_latest_snp();
    }
    /// Get the internal node ID of this origin
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the URL of this origin from the graph properties
    pub fn get_url(&self) -> Option<String> {
        let binding = self.get_graph();
        let props = binding.properties();

        // Verify this is actually an origin node
        if props.node_type(self.id) != NodeType::Origin {
            return None;
        }

        // For origin nodes, the URL is stored in the message field
        props
            .message(self.id)
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    }

    /// Get the SWHID string for this origin
    pub fn swhid(&self) -> String {
        let graph = self.get_graph();
        let props = graph.properties();
        props.swhid(self.id).to_string()
    }

    pub fn get_latest_snapshot(&self) -> Option<(NodeId, u64)> {
        let graph = self.get_graph();
        let props = graph.properties();
        if props.node_type(self.id) != NodeType::Origin {
            return None;
        }
        return swh_graph_stdlib::find_latest_snp(graph.as_ref(), self.id)
            .ok()
            .flatten();
    }

    pub fn total_commit_latest_snp(&mut self) -> Option<usize> {
        if self.number_of_commits.is_none() {
            let snapshot = self.get_latest_snapshot()?;
            let snapshot_id = snapshot.0;
            let graph = self.get_graph();
            let count = swh_graph_stdlib::iter_nodes(&graph, &[snapshot_id])
                .filter(|&node| graph.properties().node_type(node) == NodeType::Revision)
                .count();

            self.number_of_commits = count.into()
        }
        return self.number_of_commits;
    }

    pub fn total_commiter_latest_snp(&mut self) -> Option<usize> {
        //Check wether the value is not computed yet
        let graph = self.get_graph();
        if self.number_of_commiters.is_none() {
            let snapshot = self.get_latest_snapshot()?;

            let snapshot_id = snapshot.0;
            let count = swh_graph_stdlib::iter_nodes(&graph, &[snapshot_id])
                .filter(|&node| graph.properties().node_type(node) == NodeType::Revision)
                .filter_map(|rev| graph.properties().committer_id(rev).map(|ts| ts as u64))
                .collect::<std::collections::HashSet<u64>>()
                .len();

            self.number_of_commiters = count.into();
        }
        return self.number_of_commiters;
    }

    pub fn get_latest_commit_date(&mut self) -> Option<usize> {
        let graph = self.get_graph();
        if self.latest_commit_date.is_none() {
            let revisions = self.get_all_latest_snapshots_revisions();
            let mut max_date: Option<usize> = None;
            for rev in revisions {
                let props = graph.properties();
                let commit_date = props.committer_timestamp(rev);
                if let Some(date) = commit_date {
                    if let Some(max) = max_date {
                        if date > max.try_into().unwrap() {
                            max_date = Some(date.try_into().unwrap());
                        }
                    } else {
                        max_date = Some(date.try_into().unwrap());
                    }
                }
                self.latest_commit_date = max_date;
            }
        }
        //iterate over get_all_latest_snapshots_revisions and get the max commit date
        return self.latest_commit_date;
    }

    //Get all head revision of the latest snapshots
    pub fn get_all_latest_snapshots_revisions(&self) -> Vec<NodeId> {
        
        
        let latest_snapshots: (NodeId, u64) = self.get_latest_snapshot().unwrap();
        let graph = self.get_graph();
        let mut revisions: Vec<NodeId> = Vec::new();
        for succ in graph.successors(latest_snapshots.0) {
            let node_type = graph.properties().node_type(succ);
            if node_type == NodeType::Revision {
                revisions.push(succ);
            } else if node_type == NodeType::Release {
                //get all revisions linked to this release
                for rel_succ in graph.successors(succ) {
                    let rel_node_type = graph.properties().node_type(rel_succ);
                    if rel_node_type == NodeType::Revision {
                        revisions.push(rel_succ);
                    }
                }
            } else {
                //print the type for debugging
                println!("Successor {} is of type {:?}", succ, node_type);
            }
        }
        return revisions;
    }
}

impl<G> std::fmt::Debug for Origin<G>
where
    G: SwhFullGraph + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Origin")
            .field("id", &self.id)
            .field("url", &self.get_url())
            .finish()
    }
}
