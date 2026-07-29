use crate::error::Result;
use crate::node::model::Node;
use crate::tag::model::Tag;
use crate::tag::repository::TagRepository;

pub struct TagService {
    repo: TagRepository,
}

impl TagService {
    pub fn new(repo: TagRepository) -> Self {
        Self { repo }
    }

    pub fn add_tag(&self, node_id: &str, tag_name: &str) -> Result<Tag> {
        let tag = self.repo.get_or_create(tag_name)?;
        self.repo.add_to_node(node_id, &tag.id)?;
        Ok(tag)
    }

    pub fn create_tag(&self, name: &str, color: Option<&str>) -> Result<Tag> {
        self.repo.create(name, color)
    }

    pub fn rename_tag(&self, id: &str, name: &str, color: Option<&str>) -> Result<Tag> {
        self.repo.rename(id, name, color)
    }

    pub fn assign_tag(&self, node_id: &str, tag_id: &str) -> Result<()> {
        self.repo.add_to_node(node_id, tag_id)
    }

    pub fn remove_tag(&self, node_id: &str, tag_name: &str) -> Result<()> {
        let tag = self.repo.find_by_name(tag_name)?;
        self.repo.remove_from_node(node_id, &tag.id)
    }

    pub fn remove_tag_id(&self, node_id: &str, tag_id: &str) -> Result<()> {
        self.repo.remove_from_node(node_id, tag_id)
    }

    pub fn get_tags_for_node(&self, node_id: &str) -> Result<Vec<Tag>> {
        self.repo.get_tags_for_node(node_id)
    }

    pub fn get_tag_names_for_node(&self, node_id: &str) -> Result<Vec<String>> {
        self.repo.get_tag_names_for_node(node_id)
    }

    pub fn list_all(&self) -> Result<Vec<Tag>> {
        self.repo.list_all()
    }

    pub fn delete_tag(&self, id: &str) -> Result<()> {
        self.repo.delete(id)
    }

    pub fn query_tasks_by_tag(&self, id: &str) -> Result<Vec<Node>> {
        self.repo.query_tasks_by_tag(id)
    }

    pub fn sync_legacy_custom_tags(&self) -> Result<u32> {
        self.repo.sync_legacy_custom_tags()
    }
}
