use crate::error::{Error, Result};
use crate::node::model::*;

pub trait NodeHandler: Send + Sync {
    fn node_type(&self) -> NodeType;
    fn validate_properties(&self, props: &NodeProperties) -> Result<()>;
    fn default_properties(&self) -> NodeProperties;
}

pub struct TaskHandler;
impl NodeHandler for TaskHandler {
    fn node_type(&self) -> NodeType {
        NodeType::Task
    }
    fn validate_properties(&self, props: &NodeProperties) -> Result<()> {
        if let Some(priority) = props.priority {
            if !(1..=4).contains(&priority) {
                return Err(Error::Validation("Priority must be 1-4".into()));
            }
        }
        Ok(())
    }
    fn default_properties(&self) -> NodeProperties {
        NodeProperties {
            priority: Some(4),
            ..Default::default()
        }
    }
}

pub struct GroupHandler;
impl NodeHandler for GroupHandler {
    fn node_type(&self) -> NodeType {
        NodeType::Group
    }
    fn validate_properties(&self, _: &NodeProperties) -> Result<()> {
        Ok(())
    }
    fn default_properties(&self) -> NodeProperties {
        NodeProperties::default()
    }
}

pub struct WorkspaceHandler;
impl NodeHandler for WorkspaceHandler {
    fn node_type(&self) -> NodeType {
        NodeType::Workspace
    }
    fn validate_properties(&self, _: &NodeProperties) -> Result<()> {
        Ok(())
    }
    fn default_properties(&self) -> NodeProperties {
        NodeProperties::default()
    }
}

pub struct NoteHandler;
impl NodeHandler for NoteHandler {
    fn node_type(&self) -> NodeType {
        NodeType::Note
    }
    fn validate_properties(&self, _: &NodeProperties) -> Result<()> {
        Ok(())
    }
    fn default_properties(&self) -> NodeProperties {
        NodeProperties::default()
    }
}

pub fn get_handler(node_type: &NodeType) -> Box<dyn NodeHandler> {
    match node_type {
        NodeType::Task => Box::new(TaskHandler),
        NodeType::Group => Box::new(GroupHandler),
        NodeType::Workspace => Box::new(WorkspaceHandler),
        NodeType::Note => Box::new(NoteHandler),
        NodeType::Attachment => Box::new(NoteHandler),
        NodeType::Comment => Box::new(NoteHandler),
    }
}
