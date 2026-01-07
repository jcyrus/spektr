use crate::scanner::CleanableProject;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub path: PathBuf,
    pub children: Vec<TreeNode>,
    pub project: Option<CleanableProject>,
    pub collapsed: bool,
    pub checked: bool, // Simplified tri-state logic: true if ALL children checked or self checked
}

#[derive(Debug, Clone)]
pub struct TreeFlatNode<'a> {
    pub node: &'a TreeNode,
    #[allow(dead_code)]
    pub depth: usize,
    /// Pre-computed guide prefix (e.g., "│  └─ ")
    pub guide_prefix: String,
}

impl TreeNode {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            children: Vec::new(),
            project: None,
            collapsed: false,
            checked: false,
        }
    }

    pub fn label(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }
    
    pub fn total_size(&self) -> u64 {
        let self_size = self.project.as_ref().map_or(0, |p| p.total_size);
        let children_size: u64 = self.children.iter().map(|c| c.total_size()).sum();
        self_size + children_size
    }

    /// Recursively set checked state
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
        for child in &mut self.children {
            child.set_checked(checked);
        }
    }


}

/// Builds a forest (list of root nodes) from a list of projects, relative to scan_root
pub fn build_tree(projects: &[CleanableProject], scan_root: &Path) -> Vec<TreeNode> {
    let mut roots: Vec<TreeNode> = Vec::new();

    // Sort projects to ensure we process in deterministic order
    let mut projects_sorted = projects.to_vec();
    projects_sorted.sort_by(|a, b| a.root_path.cmp(&b.root_path));

    for project in projects_sorted {
        // Calculate path relative to scan_root
        // If project path is not under scan_root (shouldn't happen), we default to just checking if we can insert it at all
        // Or handle it as a separate root.
        
        let relative = match project.root_path.strip_prefix(scan_root) {
            Ok(r) => r,
            Err(_) => {
                // Fallback for paths not relative to scan_root
    if let Some(_name) = project.root_path.file_name() {
                     let mut node = TreeNode::new(project.root_path.clone());
                     node.project = Some(project.clone());
                     roots.push(node);
                }
                continue;
            }
        };

        let components: Vec<&str> = relative
            .to_str()
            .unwrap_or("")
            .split(std::path::MAIN_SEPARATOR)
            .filter(|s| !s.is_empty())
            .collect();
            
        if components.is_empty() {
             // Handle case where scan_root itself is the project (detected as empty components).
             // Create a logical root node "." for display.
             let mut node = TreeNode::new(scan_root.to_path_buf());
             node.project = Some(project.clone());
             
             // Check if "." root node already exists
             if let Some(existing) = roots.iter_mut().find(|r| r.path == scan_root) {
                 existing.project = Some(project.clone());
             } else {
                 roots.push(node);
             }
        } else {
             insert_path(&mut roots, &components, &project, scan_root);
        }
    }
    
    // Sort tree recursively
    sort_tree(&mut roots);
    
    roots
}

fn insert_path(nodes: &mut Vec<TreeNode>, components: &[&str], project: &CleanableProject, current_base: &Path) {
    if components.is_empty() {
        return;
    }

    let current_name = components[0];
    let is_last = components.len() == 1;

    // We build the full path for this node
    let node_path = current_base.join(current_name);

    // Find if this node already exists
    let idx = if let Some(i) = nodes.iter().position(|n| n.label() == current_name) {
        i
    } else {
        // Create new node
        let new_node = TreeNode::new(node_path.clone());
        nodes.push(new_node);
        nodes.len() - 1
    };

    if is_last {
        // This is the leaf (project) node
        nodes[idx].project = Some(project.clone());
    } else {
        // Continue recursion
        insert_path(&mut nodes[idx].children, &components[1..], project, &node_path);
    }
}



fn sort_tree(nodes: &mut Vec<TreeNode>) {
    // Sorting strategy: Alphabetical by label
    nodes.sort_by_key(|a| a.label().to_lowercase());
    for node in nodes {
        sort_tree(&mut node.children);
    }
}

/// Flatten the tree for rendering (List view of open nodes)
pub fn flatten_tree<'a>(roots: &'a [TreeNode]) -> Vec<TreeFlatNode<'a>> {
    let mut flat = Vec::new();
    let root_count = roots.len();
    for (i, root) in roots.iter().enumerate() {
        let is_last = i == root_count - 1;
        flatten_recursive(root, 0, is_last, &[], &mut flat);
    }
    flat
}

/// Recursively flatten tree nodes with proper guide prefix generation.
/// `ancestors_are_last` tracks whether each ancestor was the last child at its level.
fn flatten_recursive<'a>(
    node: &'a TreeNode, 
    depth: usize, 
    is_last_child: bool,
    ancestors_are_last: &[bool],
    out: &mut Vec<TreeFlatNode<'a>>
) {
    // Build guide prefix based on ancestry
    let guide_prefix = build_guide_prefix(depth, is_last_child, ancestors_are_last);
    
    out.push(TreeFlatNode {
        node,
        depth,
        guide_prefix,
    });

    if !node.collapsed {
        let child_count = node.children.len();
        // Build new ancestry for children
        let mut new_ancestors: Vec<bool> = ancestors_are_last.to_vec();
        if depth > 0 {
            new_ancestors.push(is_last_child);
        }
        
        for (i, child) in node.children.iter().enumerate() {
            let child_is_last = i == child_count - 1;
            flatten_recursive(child, depth + 1, child_is_last, &new_ancestors, out);
        }
    }
}

/// Builds the visual guide prefix string for a tree node.
/// Example outputs: "", "├─ ", "└─ ", "│  ├─ ", "│  └─ ", "   └─ "
fn build_guide_prefix(depth: usize, is_last: bool, ancestors_are_last: &[bool]) -> String {
    if depth == 0 {
        return String::new();
    }
    
    let mut prefix = String::new();
    
    // Add continuation lines for ancestors
    for &ancestor_was_last in ancestors_are_last {
        if ancestor_was_last {
            prefix.push_str("   "); // Space (no more siblings at that level)
        } else {
            prefix.push_str("│  "); // Vertical line (more siblings at that level)
        }
    }
    
    // Add connector for current node
    if is_last {
        prefix.push_str("└─ ");
    } else {
        prefix.push_str("├─ ");
    }
    
    prefix
}
