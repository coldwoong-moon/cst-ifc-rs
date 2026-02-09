//! IFC spatial hierarchy (Project -> Site -> Building -> Storey).

/// A node in the IFC spatial hierarchy tree.
#[derive(Debug, Clone)]
pub struct SpatialNode {
    pub entity_id: u64,
    pub kind: SpatialKind,
    pub name: String,
    pub description: Option<String>,
    pub children: Vec<SpatialNode>,
}

/// The kind of spatial element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpatialKind {
    Project,
    Site,
    Building,
    BuildingStorey,
    Space,
}

impl SpatialNode {
    /// Create a new spatial node.
    pub fn new(entity_id: u64, kind: SpatialKind, name: impl Into<String>) -> Self {
        Self {
            entity_id,
            kind,
            name: name.into(),
            description: None,
            children: Vec::new(),
        }
    }

    /// Add a child node.
    pub fn add_child(&mut self, child: SpatialNode) {
        self.children.push(child);
    }

    /// Find a node by entity id (depth-first search).
    pub fn find_by_id(&self, id: u64) -> Option<&SpatialNode> {
        if self.entity_id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// Collect all nodes of a given kind.
    pub fn find_by_kind(&self, kind: &SpatialKind) -> Vec<&SpatialNode> {
        let mut result = Vec::new();
        if &self.kind == kind {
            result.push(self);
        }
        for child in &self.children {
            result.extend(child.find_by_kind(kind));
        }
        result
    }

    /// Count total nodes in the subtree (including self).
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> SpatialNode {
        let mut project = SpatialNode::new(1, SpatialKind::Project, "My Project");
        let mut site = SpatialNode::new(2, SpatialKind::Site, "Main Site");
        let mut building = SpatialNode::new(3, SpatialKind::Building, "Building A");
        let storey1 = SpatialNode::new(4, SpatialKind::BuildingStorey, "Ground Floor");
        let storey2 = SpatialNode::new(5, SpatialKind::BuildingStorey, "First Floor");
        building.add_child(storey1);
        building.add_child(storey2);
        site.add_child(building);
        project.add_child(site);
        project
    }

    #[test]
    fn test_tree_count() {
        let tree = sample_tree();
        assert_eq!(tree.count(), 5);
    }

    #[test]
    fn test_find_by_id() {
        let tree = sample_tree();
        let found = tree.find_by_id(3).unwrap();
        assert_eq!(found.name, "Building A");
        assert_eq!(found.kind, SpatialKind::Building);
    }

    #[test]
    fn test_find_by_id_not_found() {
        let tree = sample_tree();
        assert!(tree.find_by_id(999).is_none());
    }

    #[test]
    fn test_find_by_kind() {
        let tree = sample_tree();
        let storeys = tree.find_by_kind(&SpatialKind::BuildingStorey);
        assert_eq!(storeys.len(), 2);
    }

    #[test]
    fn test_hierarchy_structure() {
        let tree = sample_tree();
        assert_eq!(tree.kind, SpatialKind::Project);
        assert_eq!(tree.children.len(), 1); // one site
        assert_eq!(tree.children[0].children.len(), 1); // one building
        assert_eq!(tree.children[0].children[0].children.len(), 2); // two storeys
    }
}
