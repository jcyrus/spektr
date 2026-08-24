pub mod strategy;

use anyhow::Result;
use jwalk::WalkDir;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
pub use strategy::{CleaningStrategy, RiskLevel};

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
        candidates.sort_by_key(|c| c.root.components().count());

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

            if skip {
                continue;
            }

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
                let _ = tx.send(ScanEvent::Scanning(format!(
                    "Analyzing: {}",
                    candidate.root.display()
                )));

                let targets = self.find_targets(&candidate.root, strategy.as_ref());

                let total_size = self.calculate_size(&targets);

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

    /// Calculates the total size of all targets.
    ///
    /// The walk is deliberately serial: this runs inside the `into_par_iter` in
    /// `scan`, so the surrounding Rayon pool already provides the parallelism.
    /// Letting jwalk claim the default pool from a worker thread fails with
    /// `ThreadpoolBusy` and loses the whole subtree.
    ///
    /// Unreadable entries are skipped rather than aborting the target, so a
    /// single permission error can't zero out an entire project's size.
    fn calculate_size(&self, targets: &[PathBuf]) -> u64 {
        let mut total = 0u64;

        for target in targets {
            let walker = WalkDir::new(target)
                .skip_hidden(false)
                .parallelism(jwalk::Parallelism::Serial);

            for entry in walker.into_iter().flatten() {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        total += metadata.len();
                    }
                }
            }
        }

        total
    }
}

/// Events sent during scanning
#[derive(Debug, Clone)]
pub enum ScanEvent {
    Scanning(String), // New variant for progress updates
    ProjectFound(CleanableProject),
    Complete,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc;

    /// Creates `count` Node projects, each holding one file of `file_size`
    /// bytes inside `node_modules`. Returns the total reclaimable byte count.
    fn node_projects(dir: &Path, count: usize, file_size: usize) -> u64 {
        for i in 0..count {
            let root = dir.join(format!("app{i}"));
            let modules = root.join("node_modules");
            fs::create_dir_all(&modules).unwrap();
            fs::write(root.join("package.json"), "{}").unwrap();
            fs::write(modules.join("blob.bin"), vec![b'x'; file_size]).unwrap();
        }
        (count * file_size) as u64
    }

    fn scan(root: &Path) -> Vec<CleanableProject> {
        let (tx, rx) = mpsc::channel();
        let projects = Scanner::new(strategy::default_strategies())
            .scan(root, tx)
            .unwrap();
        drop(rx);
        projects
    }

    /// `calculate_size` runs inside the scan's Rayon pool. If its inner walk
    /// tries to claim jwalk's default pool from a worker thread it fails with
    /// `ThreadpoolBusy`, and the subtree is silently counted as zero bytes --
    /// so the reported total lands far under the real size and differs between
    /// runs. Enough projects to force contention is what makes this show up.
    #[test]
    fn reports_exact_total_size_under_pool_contention() {
        let tmp = tempfile::tempdir().unwrap();
        let expected = node_projects(tmp.path(), 128, 1024);

        let projects = scan(tmp.path());

        assert_eq!(projects.len(), 128, "every project should be discovered");
        let total: u64 = projects.iter().map(|p| p.total_size).sum();
        assert_eq!(total, expected, "reported total must match bytes on disk");
    }

    /// A project inside another project's target directory is an artifact, not
    /// a project -- counting it separately would double-count its bytes and
    /// offer the user a deletion nested inside another deletion.
    #[test]
    fn skips_projects_nested_inside_a_target_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("app");
        let dep = root.join("node_modules").join("dep");
        fs::create_dir_all(&dep).unwrap();
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::write(dep.join("package.json"), "{}").unwrap();

        let projects = scan(tmp.path());

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].root_path, root);
    }

    /// An unreadable directory must not discard the bytes already counted for
    /// that target -- the previous implementation propagated the error and the
    /// caller turned the whole project into a size of zero.
    #[cfg(unix)]
    #[test]
    fn unreadable_entries_do_not_zero_the_target() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let expected = node_projects(tmp.path(), 1, 4096);

        let locked = tmp.path().join("app0").join("node_modules").join("locked");
        fs::create_dir_all(&locked).unwrap();
        fs::write(locked.join("hidden.bin"), vec![b'x'; 512]).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

        let projects = scan(tmp.path());
        // Restore permissions so the tempdir can clean itself up.
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(projects.len(), 1);
        assert!(
            projects[0].total_size >= expected,
            "readable bytes must survive an unreadable sibling: got {}, expected at least {expected}",
            projects[0].total_size
        );
    }
}
