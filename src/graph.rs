pub fn collect_origins<G: SwhFullGraph>(graph: &G) -> Vec<usize> {
     return filter_by_node_type(graph, NodeType::Origin);
             
 }
