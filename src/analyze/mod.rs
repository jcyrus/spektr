//! Interactive storage explorer.
//!
//! A single parallel pass records the recursive size of every directory under
//! the root. Browsing afterwards is pure lookup, so descending into a folder is
//! instant rather than re-walking the subtree.
//!
//! Sizes are the space actually allocated on disk, not the logical byte count.
//! A 5,214-byte file occupies two 4 KB blocks, and it is those 8,192 bytes that
//! deleting it gives back. For trees of many small files -- `node_modules`
//! being the obvious case -- the two figures diverge sharply, and the logical
//! one understates what a cleanup would reclaim.
//!
//! Symlinks are never followed and count as zero bytes: following them would
//! double-count and risks cycles. Hard-linked files (pnpm's store, for
//! instance) are counted once per link, so a tree sharing inodes with another
//! tree can read larger than the space its removal would actually free.

pub mod ui;

use crate::format::allocated_size;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// One directory's own file bytes, plus the subdirectories found inside it.
/// The unit of work for both traversal phases below.
struct Listing {
    path: PathBuf,
    files_total: u64,
    subdirs: Vec<PathBuf>,
}

/// Reads one directory: its immediate file bytes and its subdirectory paths.
/// Symlinks are skipped rather than followed, so the directory graph built
/// from repeated calls to this function is always acyclic.
fn list_dir(dir: PathBuf, progress: &Progress) -> Listing {
    let mut files_total = 0u64;
    let mut subdirs = Vec::new();

    // An unreadable directory contributes zero rather than aborting whatever
    // called it -- a single permission error shouldn't zero out its parent.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                subdirs.push(entry.path());
            } else if let Ok(metadata) = entry.metadata() {
                files_total += allocated_size(&metadata);
            }
        }
    }

    progress.visit(&dir, files_total);
    Listing {
        path: dir,
        files_total,
        subdirs,
    }
}

/// Live scan progress, polled by the UI thread rather than pushed through a
/// channel -- `mpsc::Sender` is not `Sync` and cannot cross a Rayon closure.
///
/// `cancel` lets the UI thread ask an in-flight walk to stop: dropping the
/// scan thread's `JoinHandle` without joining would only detach it, leaving
/// it to keep walking (and holding CPU/IO) in the background. `walk` checks
/// this flag on every directory it visits and returns early once it's set.
#[derive(Default)]
pub struct Progress {
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub done: AtomicBool,
    pub cancel: AtomicBool,
    pub current: Mutex<String>,
}

impl Progress {
    fn visit(&self, dir: &Path, bytes: u64) {
        self.dirs.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
        if let Ok(mut cur) = self.current.lock() {
            cur.clear();
            cur.push_str(&dir.display().to_string());
        }
    }
}

/// Recursive size of every directory in the tree, keyed by path.
pub type DirSizes = HashMap<PathBuf, u64>;

/// Walks `root`, returning the recursive byte size of every directory found.
pub fn scan(root: &Path, progress: &Progress) -> DirSizes {
    let sizes = walk(&[root.to_path_buf()], progress);
    progress.done.store(true, Ordering::Release);
    sizes
}

/// Walks each of `targets` independently and merges their sizes into one map.
/// Used for the project drill-down, which only cares about the bytes inside a
/// project's cleanup targets (`node_modules`, `target`, ...) -- not its whole
/// source tree -- so nothing outside them is ever read.
pub fn scan_targets(targets: &[PathBuf], progress: &Progress) -> DirSizes {
    let sizes = walk(targets, progress);
    progress.done.store(true, Ordering::Release);
    sizes
}

/// Sizes every directory reachable from `roots`, in two passes.
///
/// This is deliberately iterative, not recursive: a recursive walk's native
/// call stack grows with tree *depth*, and real trees (nested monorepo
/// packages, deeply generated build output) go far deeper than that stack
/// tolerates -- a directory just a few hundred levels down would overflow it
/// and abort the whole process. Depth here is instead bounded only by heap
/// memory, the same as the rest of the program's data.
///
/// Pass one (discovery) reads every directory breadth-first, one full level
/// at a time, each level's directories listed in parallel. Pass two (rollup)
/// walks that same list in reverse -- deepest level first -- so by the time a
/// directory's total is computed, every subdirectory's total already is.
fn walk(roots: &[PathBuf], progress: &Progress) -> DirSizes {
    let mut frontier: Vec<PathBuf> = roots.to_vec();
    let mut levels: Vec<Listing> = Vec::new();

    while !frontier.is_empty() {
        if progress.cancel.load(Ordering::Relaxed) {
            break;
        }
        let listed: Vec<Listing> = frontier
            .into_par_iter()
            .map(|dir| {
                if progress.cancel.load(Ordering::Relaxed) {
                    return Listing {
                        path: dir,
                        files_total: 0,
                        subdirs: Vec::new(),
                    };
                }
                list_dir(dir, progress)
            })
            .collect();

        frontier = listed.iter().flat_map(|l| l.subdirs.clone()).collect();
        levels.extend(listed);
    }

    // Deepest-discovered first, so every child total is already in `sizes`
    // by the time its parent is rolled up.
    let mut sizes = HashMap::with_capacity(levels.len());
    for listing in levels.into_iter().rev() {
        let subdir_total: u64 = listing
            .subdirs
            .iter()
            .filter_map(|path| sizes.get(path).copied())
            .sum();
        sizes.insert(listing.path, listing.files_total + subdir_total);
    }
    sizes
}

/// One row of the current directory listing.
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
}

/// A rendered row: either a real entry, or the aggregated tail.
pub enum Row<'a> {
    Item(&'a Entry),
    Other { count: usize, size: u64 },
}

/// Navigation state for the explorer.
pub struct Browser {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub cursor: usize,
    /// Rows the viewport can show; the UI updates this each frame.
    pub visible_limit: usize,
    /// Whether the aggregated tail has been expanded in place.
    pub expand_other: bool,
    sizes: DirSizes,
    stack: Vec<(PathBuf, usize)>,
    /// When set, the *top-level* listing (only) is restricted to exactly
    /// these paths instead of every entry `read_dir` finds in `cwd`. Used by
    /// the project drill-down: `cwd` is the project root so "back to
    /// projects" and breadcrumbs stay meaningful, but only its cleanup
    /// targets were ever sized -- listing the rest of the project alongside
    /// them would show paths with no data and imply they're reclaimable too.
    /// Once the user descends past the root, listing reverts to normal.
    root_allowlist: Option<Vec<PathBuf>>,
}

impl Browser {
    pub fn new(root: PathBuf, sizes: DirSizes) -> Self {
        Self::build(root, sizes, None)
    }

    /// Like [`Browser::new`], but the top-level listing shows only
    /// `allowlist` -- see [`Browser::root_allowlist`].
    pub fn new_scoped(root: PathBuf, sizes: DirSizes, allowlist: Vec<PathBuf>) -> Self {
        Self::build(root, sizes, Some(allowlist))
    }

    fn build(root: PathBuf, sizes: DirSizes, root_allowlist: Option<Vec<PathBuf>>) -> Self {
        let mut browser = Self {
            cwd: root,
            entries: Vec::new(),
            cursor: 0,
            visible_limit: 16,
            expand_other: false,
            sizes,
            stack: Vec::new(),
            root_allowlist,
        };
        browser.load();
        browser
    }

    /// Total bytes in the current directory.
    pub fn total(&self) -> u64 {
        self.sizes
            .get(&self.cwd)
            .copied()
            .unwrap_or_else(|| self.entries.iter().map(|e| e.size).sum())
    }

    pub fn at_root(&self) -> bool {
        self.stack.is_empty()
    }

    fn load(&mut self) {
        let mut entries = match &self.root_allowlist {
            // Only at the true root: descending clears `stack`'s emptiness,
            // so every deeper level falls through to the normal listing below.
            Some(allowlist) if self.stack.is_empty() => allowlist
                .iter()
                .filter_map(|path| {
                    let is_dir = path.is_dir();
                    if !is_dir && !path.is_file() {
                        // Target no longer exists (e.g. already cleaned
                        // elsewhere since the project was scanned); omit it
                        // rather than showing a phantom zero-byte row.
                        return None;
                    }
                    Some(Entry {
                        name: path.file_name()?.to_string_lossy().into_owned(),
                        size: self.sizes.get(path).copied().unwrap_or(0),
                        path: path.clone(),
                        is_dir,
                    })
                })
                .collect(),
            _ => {
                let mut entries = Vec::new();
                if let Ok(read_dir) = std::fs::read_dir(&self.cwd) {
                    for entry in read_dir.flatten() {
                        let Ok(file_type) = entry.file_type() else {
                            continue;
                        };
                        if file_type.is_symlink() {
                            continue;
                        }
                        let path = entry.path();
                        let is_dir = file_type.is_dir();
                        let size = if is_dir {
                            self.sizes.get(&path).copied().unwrap_or(0)
                        } else {
                            entry.metadata().map(|m| allocated_size(&m)).unwrap_or(0)
                        };
                        entries.push(Entry {
                            name: entry.file_name().to_string_lossy().into_owned(),
                            path,
                            size,
                            is_dir,
                        });
                    }
                }
                entries
            }
        };
        // Largest first; ties broken by name so equal-sized rows stay stable.
        entries.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
        self.entries = entries;
        self.cursor = 0;
        self.expand_other = false;
    }

    /// The rows to draw, collapsing the small tail into a single aggregate when
    /// the listing is taller than the viewport.
    pub fn rows(&self) -> Vec<Row<'_>> {
        let limit = self.visible_limit.max(1);
        if self.expand_other || self.entries.len() <= limit {
            return self.entries.iter().map(Row::Item).collect();
        }
        let shown = limit - 1;
        let mut rows: Vec<Row<'_>> = self.entries[..shown].iter().map(Row::Item).collect();
        let rest = &self.entries[shown..];
        rows.push(Row::Other {
            count: rest.len(),
            size: rest.iter().map(|e| e.size).sum(),
        });
        rows
    }

    pub fn row_count(&self) -> usize {
        self.rows().len()
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        let last = self.row_count().saturating_sub(1);
        self.cursor = (self.cursor + 1).min(last);
    }

    /// Descends into the highlighted directory, or expands the aggregate tail.
    pub fn descend(&mut self) {
        let target = match self.rows().get(self.cursor) {
            Some(Row::Item(entry)) if entry.is_dir => Some(entry.path.clone()),
            Some(Row::Other { .. }) => {
                self.expand_other = true;
                return;
            }
            _ => None,
        };
        if let Some(path) = target {
            self.stack.push((self.cwd.clone(), self.cursor));
            self.cwd = path;
            self.load();
        }
    }

    /// Returns to the parent directory, restoring its cursor position.
    pub fn ascend(&mut self) {
        if let Some((path, cursor)) = self.stack.pop() {
            self.cwd = path;
            self.load();
            self.cursor = cursor.min(self.row_count().saturating_sub(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tree() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("big/inner")).unwrap();
        fs::create_dir_all(root.join("small")).unwrap();
        fs::write(root.join("big/a.bin"), vec![b'x'; 4_000]).unwrap();
        fs::write(root.join("big/inner/b.bin"), vec![b'x'; 6_000]).unwrap();
        fs::write(root.join("small/c.bin"), vec![b'x'; 1_000]).unwrap();
        tmp
    }

    /// Directory sizes must roll up through every level, so a parent reports
    /// the bytes held by its whole subtree rather than just its own files.
    #[test]
    fn sizes_accumulate_up_the_tree() {
        let tmp = tree();
        let sizes = scan(tmp.path(), &Progress::default());

        let inner = sizes[&tmp.path().join("big/inner")];
        let big = sizes[&tmp.path().join("big")];
        let small = sizes[&tmp.path().join("small")];

        // Allocated size rounds up to whole blocks, so each is at least the
        // logical byte count that was written.
        assert!(inner >= 6_000, "inner holds 6 kB, got {inner}");
        assert!(small >= 1_000, "small holds 1 kB, got {small}");
        assert!(big > inner, "big holds inner's bytes plus its own file");
        assert_eq!(
            sizes[tmp.path()],
            big + small,
            "a directory is exactly the sum of its subtrees"
        );
    }

    /// Descending must rebase the listing on the child, and ascending must
    /// restore both the parent listing and the row the user came from.
    #[test]
    fn descend_and_ascend_restore_position() {
        let tmp = tree();
        let sizes = scan(tmp.path(), &Progress::default());
        let mut browser = Browser::new(tmp.path().to_path_buf(), sizes);

        // "big" (10 kB) sorts above "small" (1 kB).
        assert_eq!(browser.entries[0].name, "big");
        assert!(browser.at_root());

        browser.descend();
        assert_eq!(browser.cwd, tmp.path().join("big"));
        assert!(browser.total() >= 10_000);
        assert!(!browser.at_root());

        browser.move_down();
        let moved_to = browser.cursor;
        browser.ascend();
        assert_eq!(browser.cwd, tmp.path());
        assert_eq!(browser.cursor, 0, "returns to the row we descended from");
        assert_ne!(moved_to, 0, "cursor had actually moved inside the child");
    }

    /// A listing taller than the viewport collapses its tail into one row, and
    /// that row must account for every remaining byte -- otherwise the visible
    /// numbers stop summing to the directory total.
    #[test]
    fn tail_aggregates_into_other_without_losing_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..10 {
            fs::write(
                tmp.path().join(format!("f{i}.bin")),
                vec![b'x'; 100 * (i + 1)],
            )
            .unwrap();
        }
        let sizes = scan(tmp.path(), &Progress::default());
        let mut browser = Browser::new(tmp.path().to_path_buf(), sizes);
        browser.visible_limit = 4;

        let rows = browser.rows();
        assert_eq!(rows.len(), 4, "three entries plus the aggregate");
        let Some(Row::Other { count, size }) = rows.last() else {
            panic!("expected an aggregate tail row");
        };
        assert_eq!(*count, 7);

        let shown: u64 = rows
            .iter()
            .filter_map(|r| match r {
                Row::Item(e) => Some(e.size),
                Row::Other { .. } => None,
            })
            .sum();
        assert_eq!(shown + *size, browser.total(), "rows must sum to the total");
    }

    /// Expanding the tail reveals every entry.
    #[test]
    fn expanding_other_reveals_all_entries() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..10 {
            fs::write(tmp.path().join(format!("f{i}.bin")), vec![b'x'; 100]).unwrap();
        }
        let sizes = scan(tmp.path(), &Progress::default());
        let mut browser = Browser::new(tmp.path().to_path_buf(), sizes);
        browser.visible_limit = 4;

        browser.cursor = 3; // the aggregate row
        browser.descend();
        assert!(browser.expand_other);
        assert_eq!(browser.rows().len(), 10);
    }

    /// A subtree deeper than the old 128-level cutoff must still be sized,
    /// not silently zeroed. Symlinks are never followed, so a real directory
    /// graph has no cycles and there is nothing pathological to guard
    /// against by dropping data past a fixed depth.
    #[test]
    fn deeply_nested_directories_are_not_dropped() {
        let tmp = tempfile::tempdir().unwrap();
        let mut deep = tmp.path().to_path_buf();
        for i in 0..200 {
            deep = deep.join(format!("d{i}"));
        }
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("leaf.bin"), vec![b'x'; 2_000]).unwrap();

        let sizes = scan(tmp.path(), &Progress::default());

        assert!(
            sizes[tmp.path()] >= 2_000,
            "a 200-level-deep file must still be counted, got {}",
            sizes[tmp.path()]
        );
    }

    /// Setting `cancel` mid-walk must stop it quickly rather than let it run
    /// to completion in the background -- otherwise backing out of a
    /// drill-down and reopening it repeatedly piles up concurrent full-tree
    /// scans that were supposedly abandoned.
    #[test]
    fn cancel_stops_the_walk_without_completing_it() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..500 {
            let dir = tmp.path().join(format!("d{i}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("f.bin"), vec![b'x'; 4_096]).unwrap();
        }

        let progress = Progress::default();
        progress.cancel.store(true, Ordering::Relaxed);
        let sizes = scan(tmp.path(), &progress);

        assert!(
            progress.done.load(Ordering::Acquire),
            "scan must still finish and mark done"
        );
        assert!(
            sizes.len() < 500,
            "a walk cancelled before it starts should visit far fewer than all 500 subdirectories, visited {}",
            sizes.len()
        );
    }

    /// The drill-down only ever sizes a project's cleanup targets, not its
    /// whole source tree -- `new_scoped` must show exactly those targets at
    /// the root, with sizes matching what was actually scanned, and fall
    /// back to a normal full listing once the user descends into one.
    #[test]
    fn scoped_browser_lists_only_the_allowlisted_targets_at_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("node_modules/pkg/index.js"), vec![b'x'; 3_000]).unwrap();
        fs::create_dir_all(root.join("src")).unwrap(); // not a target; must not appear
        fs::write(root.join("README.md"), "hi").unwrap(); // not a target either

        let targets = vec![root.join("node_modules")];
        let sizes = scan_targets(&targets, &Progress::default());
        let mut browser = Browser::new_scoped(root.clone(), sizes, targets);

        assert_eq!(browser.entries.len(), 1, "only the target should be listed");
        assert_eq!(browser.entries[0].name, "node_modules");
        assert!(browser.entries[0].size >= 3_000);
        // The un-walked source files were never scanned, so the fallback sum
        // of entries (not a missing map lookup) must still equal the total.
        assert_eq!(browser.total(), browser.entries[0].size);

        browser.descend();
        assert_eq!(browser.cwd, root.join("node_modules"));
        assert_eq!(
            browser.entries.len(),
            1,
            "inside the target, listing is unrestricted again"
        );
        assert_eq!(browser.entries[0].name, "pkg");

        browser.ascend();
        assert_eq!(browser.cwd, root);
        assert_eq!(
            browser.entries.len(),
            1,
            "back at the root, the allowlist applies again"
        );
    }
}
