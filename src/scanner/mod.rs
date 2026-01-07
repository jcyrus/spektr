pub mod strategy;

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
    pub fn scan(&self, root: &Path, tx: Sender<ScanEvent>) -> Result<Vec<CleanableProject>> {
        let mut projects = Vec::new();

        // Use jwalk for parallel directory traversal
        for entry in WalkDir::new(root)
            .skip_hidden(false)
            .parallelism(jwalk::Parallelism::RayonNewPool(num_cpus::get()))
        {
            let entry = entry?;
            let path = entry.path();

            // Check if this directory matches any strategy
            if path.is_dir() {
                for strategy in &self.strategies {
                    if strategy.detect(&path) {
                        // Found a project! Calculate its cleanable size
                        let targets = self.find_targets(&path, strategy.as_ref());
                        let total_size = self.calculate_size(&targets)?;

                        let project = CleanableProject {
                            root_path: path.clone(),
                            strategy_name: strategy.name().to_string(),
                            targets,
                            total_size,
                            risk_level: strategy.risk_level(),
                        };

                        // Send progress update
                        tx.send(ScanEvent::ProjectFound(project.clone()))?;

                        projects.push(project);
                    }
                }
            }
        }

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
    ProjectFound(CleanableProject),
    Complete,
}
