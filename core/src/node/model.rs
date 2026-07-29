use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum NodeType {
    Workspace,
    Group,
    Task,
    Note,
    Attachment,
    Comment,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeType::Workspace => "Workspace",
            NodeType::Group => "Group",
            NodeType::Task => "Task",
            NodeType::Note => "Note",
            NodeType::Attachment => "Attachment",
            NodeType::Comment => "Comment",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Workspace" => Some(NodeType::Workspace),
            "Group" => Some(NodeType::Group),
            "Task" => Some(NodeType::Task),
            "Note" => Some(NodeType::Note),
            "Attachment" => Some(NodeType::Attachment),
            "Comment" => Some(NodeType::Comment),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    P1 = 1,
    P2 = 2,
    P3 = 3,
    P4 = 4,
}

impl Priority {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            1 => Some(Priority::P1),
            2 => Some(Priority::P2),
            3 => Some(Priority::P3),
            4 => Some(Priority::P4),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reminders: Option<Vec<ReminderConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<RecurrenceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderConfig {
    pub time: Option<DateTime<Utc>>,
    pub offset_minutes: Option<i32>,
    #[serde(rename = "type")]
    pub reminder_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceConfig {
    pub pattern: String,
    pub interval: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    pub position: f64,
    pub node_type: NodeType,
    pub title: String,
    pub body: String,
    pub properties: NodeProperties,
    pub is_collapsed: bool,
    pub is_completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub version: i32,
    pub clock: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeViewModel {
    pub id: String,
    pub parent_id: Option<String>,
    pub node_type: NodeType,
    pub title: String,
    pub depth: u32,
    pub has_children: bool,
    pub is_expanded: bool,
    pub is_completed: bool,
    pub priority: Option<i32>,
    pub due_date: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub child_count: u32,
    pub completed_child_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNodeInput {
    pub parent_id: Option<String>,
    pub node_type: NodeType,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub properties: NodeProperties,
    pub position: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateNodeInput {
    pub title: Option<String>,
    pub body: Option<String>,
    pub node_type: Option<NodeType>,
    pub is_collapsed: Option<bool>,
    pub properties: Option<NodeProperties>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchFilters {
    pub node_types: Option<Vec<NodeType>>,
    pub tags: Option<Vec<String>>,
    pub priority: Option<i32>,
    pub due_before: Option<DateTime<Utc>>,
    pub due_after: Option<DateTime<Utc>>,
    pub is_completed: Option<bool>,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub node: Node,
    pub snippet: String,
    pub breadcrumb: Vec<String>,
    pub rank: f64,
}
