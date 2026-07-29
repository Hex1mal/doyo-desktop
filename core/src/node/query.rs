use crate::node::model::*;

pub fn build_node_viewmodel(
    node: &Node,
    child_count: u32,
    completed_child_count: u32,
    is_expanded: bool,
    depth: u32,
    tags: Vec<String>,
) -> NodeViewModel {
    NodeViewModel {
        id: node.id.clone(),
        parent_id: node.parent_id.clone(),
        node_type: node.node_type.clone(),
        title: node.title.clone(),
        depth,
        has_children: child_count > 0,
        is_expanded,
        is_completed: node.is_completed,
        priority: node.properties.priority,
        due_date: node.properties.due_date,
        tags,
        child_count,
        completed_child_count,
    }
}
