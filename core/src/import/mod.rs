use crate::db::Database;
use crate::error::Result;
use crate::node::model::CreateNodeInput;
use crate::node::repository::NodeRepository;
use std::sync::Arc;

pub struct ImportService {
    db: Arc<Database>,
}

impl ImportService {
    pub fn new(db: Arc<Database>) -> Self { Self { db } }

    pub fn import_json(&self, json: &str, parent_id: Option<&str>) -> Result<Vec<String>> {
        let repo = NodeRepository::new(self.db.clone());
        let data: serde_json::Value = serde_json::from_str(json)?;
        let nodes = data.as_array()
            .ok_or_else(|| crate::error::Error::Validation("Expected JSON array".into()))?;

        let mut imported = Vec::new();
        let mut id_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for node in nodes {
            let original_id = node["id"].as_str().unwrap_or("").to_string();
            let node_type_str = node["type"].as_str().unwrap_or("Task");
            let node_type = crate::node::model::NodeType::from_str(node_type_str)
                .unwrap_or(crate::node::model::NodeType::Task);

            let new_parent_id = match node["parentId"].as_str() {
                Some(pid) => id_map.get(pid).cloned().or_else(|| parent_id.map(String::from)),
                None => parent_id.map(String::from),
            };

            let input = CreateNodeInput {
                parent_id: new_parent_id,
                node_type,
                title: node["title"].as_str().unwrap_or("Untitled").to_string(),
                body: node["body"].as_str().unwrap_or("").to_string(),
                properties: serde_json::from_str(node["properties"].as_str().unwrap_or("{}")).unwrap_or_default(),
                position: None,
            };

            match repo.create(&input) {
                Ok(new_node) => {
                    id_map.insert(original_id, new_node.id.clone());
                    imported.push(new_node.id);
                }
                Err(e) => {
                    log::warn!("Failed to import node: {}", e);
                }
            }
        }

        Ok(imported)
    }
}
