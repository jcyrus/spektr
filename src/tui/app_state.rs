use crate::scanner::CleanableProject;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    SizeDesc,
    SizeAsc,
    NameAsc,
    NameDesc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    NodeJs,
    Rust,
    Flutter,
    Android,
}

impl FilterMode {
    pub fn next(&self) -> Self {
        match self {
            Self::All => Self::NodeJs,
            Self::NodeJs => Self::Rust,
            Self::Rust => Self::Flutter,
            Self::Flutter => Self::Android,
            Self::Android => Self::All,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::All => "All",
            Self::NodeJs => "Node.js",
            Self::Rust => "Rust",
            Self::Flutter => "Flutter",
            Self::Android => "Android",
        }
    }
}

use crate::tui::tree::{TreeNode, build_tree, flatten_tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Tree,
}

use std::path::PathBuf;

pub struct AppState {
    /// The root path of the scan
    pub scan_path: PathBuf,

    /// All discovered projects
    all_projects: Vec<CleanableProject>,
    
    /// Filtered and sorted projects (displayed)
    visible_projects: Vec<CleanableProject>,
    
    /// Currently selected index in visible_projects (or flattened tree)
    pub selected_index: usize,
    
    /// Set of selected project indices (for multi-selection in List mode)
    /// In Tree mode, the TreeNode itself holds Checked state
    selected_projects: HashSet<usize>,
    
    /// Current sort mode
    pub sort_mode: SortMode,
    
    /// Current filter mode
    pub filter_mode: FilterMode,

    /// Current view mode (List vs Tree)
    pub view_mode: ViewMode,

    /// Root nodes of the project tree
    pub tree_roots: Vec<TreeNode>,
    
    /// Show confirmation modal
    pub show_confirmation: bool,
    
    /// User confirmed deletion (set when 'y' is pressed)
    pub deletion_confirmed: bool,
    
    /// Scan is still running
    pub scanning: bool,

    /// Current path being scanned
    pub scanning_path: String,
    
    /// Spinner animation index
    pub spinner_index: usize,
}

impl AppState {
    pub fn new(scan_path: PathBuf) -> Self {
        Self {
            scan_path,
            all_projects: Vec::new(),
            visible_projects: Vec::new(),
            selected_index: 0,
            selected_projects: HashSet::new(),
            sort_mode: SortMode::SizeDesc,
            filter_mode: FilterMode::All,
            view_mode: ViewMode::List,
            tree_roots: Vec::new(),
            show_confirmation: false,
            deletion_confirmed: false,
            scanning: true,
            scanning_path: String::new(),
            spinner_index: 0,
        }
    }

    pub fn add_project(&mut self, project: CleanableProject) {
        self.all_projects.push(project);
        self.refresh_visible();
    }

    pub fn finish_scan(&mut self) {
        self.scanning = false;
        self.refresh_visible();
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::List => ViewMode::Tree,
            ViewMode::Tree => ViewMode::List,
        };
        self.selected_index = 0;
        self.refresh_visible();
    }

    /// Toggle expand/collapse of current node in Tree mode
    pub fn toggle_expand(&mut self) {
        if self.view_mode == ViewMode::List {
            return;
        }

        if let Some(node) = self.get_node_at_mut(self.selected_index) {
            if !node.children.is_empty() {
                node.collapsed = !node.collapsed;
            }
        }
    }

    pub fn get_flat_tree(&self) -> Vec<crate::tui::tree::TreeFlatNode<'_>> {
        flatten_tree(&self.tree_roots)
    }

    fn get_node_at_mut(&mut self, index: usize) -> Option<&mut TreeNode> {
        let mut current_idx = 0;
        for root in &mut self.tree_roots {
            if let Some(node) = find_node_at_mut(root, &mut current_idx, index) {
                return Some(node);
            }
        }
        None
    }

    pub fn visible_projects(&self) -> &[CleanableProject] {
        &self.visible_projects
    }



    pub fn visible_count(&self) -> usize {
        match self.view_mode {
            ViewMode::List => self.visible_projects.len(),
            ViewMode::Tree => self.get_flat_tree().len(),
        }
    }

    /// Toggle selection of the current project
    pub fn toggle_selection(&mut self) {
        match self.view_mode {
            ViewMode::List => {
                if self.visible_projects.is_empty() {
                    return;
                }
                if self.selected_projects.contains(&self.selected_index) {
                    self.selected_projects.remove(&self.selected_index);
                } else {
                    self.selected_projects.insert(self.selected_index);
                }
            }
            ViewMode::Tree => {
                if let Some(node) = self.get_node_at_mut(self.selected_index) {
                    let new_state = !node.checked;
                    node.set_checked(new_state);
                }
            }
        }
    }

    pub fn is_selected(&self, index: usize) -> bool {
        match self.view_mode {
            ViewMode::List => self.selected_projects.contains(&index),
            ViewMode::Tree => {
                // For rendering tree, we need to know if the Nth visible node is checked.
                // This is a bit inefficient to traverse O(N) for every line render.
                // In render loop we should traverse once.
                // But for random access:
                let flat = self.get_flat_tree();
                flat.get(index).map(|n| n.node.checked).unwrap_or(false)
            }
        }
    }

    pub fn selected_count(&self) -> usize {
        match self.view_mode {
            ViewMode::List => self.selected_projects.len(),
            ViewMode::Tree => {
                // Count all checked projects in the tree (recursively)
                count_checked_projects(&self.tree_roots)
            }
        }
    }

    pub fn total_selected_size(&self) -> u64 {
        match self.view_mode {
            ViewMode::List => self.selected_projects
                .iter()
                .filter_map(|&idx| self.visible_projects.get(idx))
                .map(|p| p.total_size)
                .sum(),
            ViewMode::Tree => {
                // Sum size of all checked projects in tree
                sum_checked_size(&self.tree_roots)
            }
        }
    }

    pub fn get_selected_projects(&self) -> Vec<CleanableProject> {
        match self.view_mode {
            ViewMode::List => self.selected_projects
                .iter()
                .filter_map(|&idx| self.visible_projects.get(idx))
                .cloned()
                .collect(),
            ViewMode::Tree => {
                let mut projects = Vec::new();
                collect_checked_projects(&self.tree_roots, &mut projects);
                projects
            }
        }
    }

    pub fn confirm_deletion(&mut self) {
        self.deletion_confirmed = true;
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index + 1 < self.visible_count() {
            self.selected_index += 1;
        }
    }

    pub fn toggle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::SizeDesc => SortMode::SizeAsc,
            SortMode::SizeAsc => SortMode::NameAsc,
            SortMode::NameAsc => SortMode::NameDesc,
            SortMode::NameDesc => SortMode::SizeDesc,
        };
        self.refresh_visible();
    }

    pub fn cycle_filter(&mut self) {
        self.filter_mode = self.filter_mode.next();
        self.selected_index = 0;
        self.selected_projects.clear();
        self.refresh_visible();
    }

    /// Refresh visible projects based on current filter and sort
    fn refresh_visible(&mut self) {
        // 1. Filter all projects
        let mut filtered: Vec<CleanableProject> = self
            .all_projects
            .iter()
            .filter(|p| match self.filter_mode {
                FilterMode::All => true,
                FilterMode::NodeJs => p.strategy_name == "Node.js",
                FilterMode::Rust => p.strategy_name == "Rust",
                FilterMode::Flutter => p.strategy_name == "Flutter",
                FilterMode::Android => p.strategy_name == "Android",
            })
            .cloned()
            .collect();

        // 2. Sort or Build Tree
        match self.view_mode {
            ViewMode::List => {
                // Sort
                match self.sort_mode {
                    SortMode::SizeDesc => filtered.sort_by_key(|p| std::cmp::Reverse(p.total_size)),
                    SortMode::SizeAsc => filtered.sort_by_key(|p| p.total_size),
                    SortMode::NameAsc => {
                        filtered.sort_by(|a, b| a.root_path.cmp(&b.root_path));
                    }
                    SortMode::NameDesc => {
                        filtered.sort_by(|a, b| b.root_path.cmp(&a.root_path));
                    }
                }
                // Take top 100 for performance (list only)
                // filtered.truncate(100); 
                
                self.visible_projects = filtered;
            }
            ViewMode::Tree => {
                // For Tree, we sort by Path primarily to structure it correctly,
                // or we rely on build_tree to separate them.
                // build_tree handles sorting.
                self.tree_roots = build_tree(&filtered, &self.scan_path);
                // Tree roots are re-built, so expanded state is lost...
                // Ideally we should preserve state, but for MVP re-collapse is acceptable on filter change.
            }
        }

        // Clamp selected index
        let count = self.visible_count();
        if self.selected_index >= count && count > 0 {
            self.selected_index = count - 1;
        }
    }

    pub fn current_project(&self) -> Option<&CleanableProject> {
        match self.view_mode {
            ViewMode::List => self.visible_projects.get(self.selected_index),
            ViewMode::Tree => {
                 let flat = self.get_flat_tree();
                 flat.get(self.selected_index).and_then(|node| node.node.project.as_ref())
            }
        }
    }
}

// Helper functions for tree traversal

fn find_node_at_mut<'a>(node: &'a mut TreeNode, current_idx: &mut usize, target_idx: usize) -> Option<&'a mut TreeNode> {
    if *current_idx == target_idx {
        return Some(node);
    }
    *current_idx += 1;

    if !node.collapsed {
        for child in &mut node.children {
            if let Some(found) = find_node_at_mut(child, current_idx, target_idx) {
                return Some(found);
            }
        }
    }
    None
}

fn count_checked_projects(nodes: &[TreeNode]) -> usize {
    let mut count = 0;
    for node in nodes {
        if node.project.is_some() && node.checked {
            count += 1;
        }
        count += count_checked_projects(&node.children);
    }
    count
}

fn sum_checked_size(nodes: &[TreeNode]) -> u64 {
    let mut total = 0;
    for node in nodes {
        if node.project.is_some() && node.checked {
             // Sum size only for checked projects (folders have None project)
             if let Some(p) = &node.project {
                 total += p.total_size;
             }
        }
        total += sum_checked_size(&node.children);
    }
    total
}

fn collect_checked_projects(nodes: &[TreeNode], out: &mut Vec<CleanableProject>) {
    for node in nodes {
        if node.checked {
            if let Some(p) = &node.project {
                out.push(p.clone());
            }
        }
        collect_checked_projects(&node.children, out);
    }
}
