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

pub struct AppState {
    /// All discovered projects
    all_projects: Vec<CleanableProject>,
    
    /// Filtered and sorted projects (displayed)
    visible_projects: Vec<CleanableProject>,
    
    /// Currently selected index in visible_projects
    pub selected_index: usize,
    
    /// Set of selected project indices (for multi-selection)
    selected_projects: HashSet<usize>,
    
    /// Current sort mode
    pub sort_mode: SortMode,
    
    /// Current filter mode
    pub filter_mode: FilterMode,
    
    /// Show confirmation modal
    pub show_confirmation: bool,
    
    /// User confirmed deletion (set when 'y' is pressed)
    pub deletion_confirmed: bool,
    
    /// Scan is still running
    pub scanning: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            all_projects: Vec::new(),
            visible_projects: Vec::new(),
            selected_index: 0,
            selected_projects: HashSet::new(),
            sort_mode: SortMode::SizeDesc,
            filter_mode: FilterMode::All,
            show_confirmation: false,
            deletion_confirmed: false,
            scanning: true,
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

    pub fn visible_projects(&self) -> &[CleanableProject] {
        &self.visible_projects
    }

    pub fn total_projects(&self) -> usize {
        self.all_projects.len()
    }

    pub fn visible_count(&self) -> usize {
        self.visible_projects.len()
    }

    /// Toggle selection of the current project
    pub fn toggle_selection(&mut self) {
        if self.visible_projects.is_empty() {
            return;
        }

        if self.selected_projects.contains(&self.selected_index) {
            self.selected_projects.remove(&self.selected_index);
        } else {
            self.selected_projects.insert(self.selected_index);
        }
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_projects.contains(&index)
    }

    pub fn selected_count(&self) -> usize {
        self.selected_projects.len()
    }

    pub fn total_selected_size(&self) -> u64 {
        self.selected_projects
            .iter()
            .filter_map(|&idx| self.visible_projects.get(idx))
            .map(|p| p.total_size)
            .sum()
    }

    pub fn get_selected_projects(&self) -> Vec<&CleanableProject> {
        self.selected_projects
            .iter()
            .filter_map(|&idx| self.visible_projects.get(idx))
            .collect()
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
        if self.selected_index + 1 < self.visible_projects.len() {
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
        // Filter
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

        // Take top 100 for performance
        filtered.truncate(100);

        self.visible_projects = filtered;

        // Clamp selected index
        if self.selected_index >= self.visible_projects.len() && !self.visible_projects.is_empty()
        {
            self.selected_index = self.visible_projects.len() - 1;
        }
    }

    pub fn current_project(&self) -> Option<&CleanableProject> {
        self.visible_projects.get(self.selected_index)
    }
}
