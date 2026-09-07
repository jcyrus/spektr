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

use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// Live scan progress, polled by the UI thread rather than pushed through a
/// channel -- `mpsc::Sender` is not `Sync` and cannot cross a Rayon closure.
#[derive(Default)]
pub struct Progress {
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub done: AtomicBool,
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

/// Space a file occupies on disk, which is its block allocation rather than
/// its logical length. Windows has no cheap equivalent in `std`, so the
/// logical size stands in there.
#[cfg(unix)]
fn allocated_size(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    // `blocks()` is always in 512-byte units, whatever the filesystem uses.
    metadata.blocks().saturating_mul(512)
}

#[cfg(not(unix))]
fn allocated_size(metadata: &std::fs::Metadata) -> u64 {
    metadata.len()
}

/// Recursive size of every directory in the tree, keyed by path.
pub type DirSizes = HashMap<PathBuf, u64>;

/// Walks `root`, returning the recursive byte size of every directory found.
pub fn scan(root: &Path, progress: &Progress) -> DirSizes {
    let sizes = Mutex::new(HashMap::new());
    walk(root, &sizes, progress, 0);
    progress.done.store(true, Ordering::Release);
    sizes.into_inner().unwrap_or_default()
}

/// Guards against pathological trees; real filesystems stay far below this.
const MAX_DEPTH: u32 = 128;

fn walk(dir: &Path, sizes: &Mutex<DirSizes>, progress: &Progress, depth: u32) -> u64 {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        // Unreadable directory: record zero rather than aborting the parent.
        Err(_) => {
            record(sizes, dir, 0);
            return 0;
        }
    };

    let mut files_total = 0u64;
    let mut subdirs = Vec::new();

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

    progress.visit(dir, files_total);

    let subdir_total: u64 = if depth >= MAX_DEPTH {
        0
    } else {
        subdirs
            .par_iter()
            .map(|path| walk(path, sizes, progress, depth + 1))
            .sum()
    };

    let total = files_total + subdir_total;
    record(sizes, dir, total);
    total
}

fn record(sizes: &Mutex<DirSizes>, dir: &Path, total: u64) {
    if let Ok(mut map) = sizes.lock() {
        map.insert(dir.to_path_buf(), total);
    }
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
}

impl Browser {
    pub fn new(root: PathBuf, sizes: DirSizes) -> Self {
        let mut browser = Self {
            cwd: root,
            entries: Vec::new(),
            cursor: 0,
            visible_limit: 16,
            expand_other: false,
            sizes,
            stack: Vec::new(),
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
}
