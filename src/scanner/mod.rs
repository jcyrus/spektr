pub mod strategy;

use rayon::prelude::*;
pub use strategy::{CleaningStrategy, RiskLevel};
use anyhow::Result;
use jwalk::WalkDir;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

/// Represents a discovered project that can be cleaned
#[derive(Debug, Clone)]
pub struct CleanableProject {
    pub root_path: PathBuf,
    pub strategy_name: String,
    pub targets: Vec<PathBuf>,
    pub total_size: u64,
    #[allow(dead_code)]
    pub risk_level: RiskLevel,
}

/// Scanner that uses multiple cleaning strategies to find cleanable artifacts
pub struct Scanner {
    strategies: Vec<Box<dyn CleaningStrategy>>,
}

impl Scanner {
    pub fn new(strategies: Vec<Box<dyn CleaningStrategy>>) -> Self {
        Self { strategies }
    }

    /// Scans a directory tree for cleanable projects
    /// Sends updates via the provided channel
    /// Scans a directory tree for cleanable projects
    /// Sends updates via the provided channel
    pub fn scan(&self, root: &Path, tx: Sender<ScanEvent>) -> Result<Vec<CleanableProject>> {
        struct Candidate {
            root: PathBuf,
            strategy_idx: usize,
        }

        let mut candidates = Vec::new();

        // 1. Discovery Phase: specific project detection
        // Use jwalk for parallel directory traversal
        let tx_progress = tx.clone();
        for entry in WalkDir::new(root)
            .skip_hidden(false)
            .process_read_dir(move |_depth, path, _read_dir_state, _children| {
                // Emit scanning event (best effort)
                let _ = tx_progress.send(ScanEvent::Scanning(path.display().to_string()));
            })
            .parallelism(jwalk::Parallelism::RayonNewPool(num_cpus::get()))
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                for (idx, strategy) in self.strategies.iter().enumerate() {
                    if strategy.detect(&path) {
                        candidates.push(Candidate {
                            root: path.clone(),
                            strategy_idx: idx,
                        });
                        // Once a strategy matches, stop checking others for this dir
                        // (Assuming one dir isn't multiple project types simultaneously, or if so, first wins)
                        break; 
                    }
                }
            }
        }

        // 2. Deduplication Phase: Filter out nested projects
        // Sort by path length (shortest first) to ensure parents are processed before children
        candidates.sort_by(|a, b| a.root.components().count().cmp(&b.root.components().count()));

        let mut valid_projects = Vec::new();
        let mut ignored_prefixes = Vec::new();

        for candidate in candidates {
            // Check if this project is inside a directory marked for deletion
            let mut skip = false;
            for prefix in &ignored_prefixes {
                if candidate.root.starts_with(prefix) { 
                    skip = true; 
                    break; 
                }
            }

            if skip { continue; }

            // It's a valid project
            let strategy = &self.strategies[candidate.strategy_idx];
            
            // Mark its targets as ignored zones for future candidates
            for target_name in strategy.targets() {
                ignored_prefixes.push(candidate.root.join(target_name));
            }

            valid_projects.push(candidate);
        }

        // 3. Calculation Phase: Compute sizes and notify
        let projects: Vec<CleanableProject> = valid_projects
            .into_par_iter()
            .map(|candidate| {
                let strategy = &self.strategies[candidate.strategy_idx];
                
                // Emit scanning event for this project
                // Clone tx for this thread
                let _ = tx.send(ScanEvent::Scanning(format!("Analyzing: {}", candidate.root.display())));

                let targets = self.find_targets(&candidate.root, strategy.as_ref());
                
                // Calculate size (using jwalk internally for parallelism)
                let total_size = self.calculate_size(&targets).unwrap_or(0);

                let project = CleanableProject {
                    root_path: candidate.root,
                    strategy_name: strategy.name().to_string(),
                    targets,
                    total_size,
                    risk_level: strategy.risk_level(),
                };

                // Send progress update
                let _ = tx.send(ScanEvent::ProjectFound(project.clone()));

                project
            })
            .collect();

        tx.send(ScanEvent::Complete)?;
        Ok(projects)
    }

    /// Finds all target directories within a project
    fn find_targets(&self, root: &Path, strategy: &dyn CleaningStrategy) -> Vec<PathBuf> {
        let mut targets = Vec::new();

        for target_name in strategy.targets() {
            let target_path = root.join(target_name);
            if target_path.exists() {
                targets.push(target_path);
            }
        }

        targets
    }

    /// Calculates the total size of all targets
    fn calculate_size(&self, targets: &[PathBuf]) -> Result<u64> {
        let mut total = 0u64;

        for target in targets {
            for entry in WalkDir::new(target).skip_hidden(false) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    total += entry.metadata()?.len();
                }
            }
        }

        Ok(total)
    }
}

/// Events sent during scanning
#[derive(Debug, Clone)]
pub enum ScanEvent {
    Scanning(String), // New variant for progress updates
    ProjectFound(CleanableProject),
    Complete,
}
