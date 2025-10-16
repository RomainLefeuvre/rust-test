use std::fs::File;
use std::fs::read_to_string;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use swh_graph::graph::NodeId;
use swh_graph::graph::SwhGraphWithProperties;
use swh_graph::properties;
use swh_graph::NodeType;


pub fn write_node_ids(path: &PathBuf, node_ids: &Vec<NodeId>) -> Result<(), io::Error> {
    let mut file = File::create(path)?;
    for node_id in node_ids {
        writeln!(file, "{}", node_id)?;
    }
    Ok(())
}

pub fn read_node_ids(path: &PathBuf) -> Result<Vec<NodeId>, io::Error> {
    let node_ids = read_to_string(path)?
        .lines()
        .map(|x| {
            x.parse::<usize>()
                .expect(&format!("Failed to parse NodeId '{}' from origin file", x))
        })
        .collect();
    Ok(node_ids)
}

pub fn filter_by_node_type<G>(graph: &G, node_type: NodeType) -> Vec<NodeId>
where
    G: SwhGraphWithProperties<Maps: properties::Maps>,
{
   let props = graph.properties();
   
     return (0..graph.num_nodes())
             .filter(|&node| props.node_type(node) == node_type)
             .collect()
}