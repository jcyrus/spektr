# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- Self-update capability (`spektr --update`)
- Configuration file support for custom strategies
- Python support (.venv, **pycache**)
- Go support (vendor, bin)
- Docker/container artifact cleaning
- Statistics dashboard (space saved over time)

## [0.2.0] - 2026-09-07

### Added

- **Storage Explorer (`spektr analyze [path]`)**
  - Interactive drill-down into any directory, largest first at every level
  - One parallel pass sizes the whole tree up front, so descending is instant
  - Percentages rebase to the current folder as you navigate
  - `Other (N items)` collapses the long tail instead of flooding the view
- **Project drill-down** — press `→` on any project in the main list to break
  its size down folder by folder with the same bars, `←` to back out
- **Synthwave theme** applied across the whole TUI (`src/theme.rs`): magenta
  accents, amber highlights, a single reserved danger color for destructive
  actions, and size-magnitude-colored figures throughout

### Fixed

- Directory and project sizes now report **space allocated on disk** (matching
  `du`/Finder) instead of logical byte length, which understated the size of
  trees with many small files
- Byte formatting now uses **decimal SI units** (1 kB = 1000 B), matching
  Finder; the previous binary-divisor math under-reported by ~7% at MB and
  ~10% at GB against what users saw elsewhere
- The project list didn't track `ListState`, so scanning a folder with more
  projects than fit on screen made rows below the fold unreachable

## [0.1.1] - 2026-01-08

### Added

- **Tree View Mode**
  - Visual hierarchy with guide characters (`│`, `├─`, `└─`)
  - Press `Tab` to toggle between List and Tree views
  - Nested project display for monorepos/workspaces
  - Parent-child selection propagation (selecting parent selects all children)
  - Expand/collapse nodes with `→` or `l` key

### Fixed

- **Security Hardening**
  - Replaced panic-prone `.unwrap()` / `.expect()` with proper `Result` handling
  - Expanded `.gitignore` to prevent accidental commit of IDE/OS files

### Changed

- **Code Quality**
  - Resolved all `cargo clippy` warnings
  - Refactored conversational comments to professional documentation
  - Added `homepage` field to `Cargo.toml` metadata

## [0.1.0] - 2026-01-07

### Added

- **Core Scanner Engine**

  - Multi-threaded directory scanning using `jwalk`
  - Trait-based `CleaningStrategy` architecture for extensibility
  - Channel-based progress reporting
  - Parallel size calculation

- **Cleaning Strategies**

  - Node.js: `node_modules`, `.next`, `dist`, `build`
  - Rust: `target`
  - Flutter: `build`, `.dart_tool`
  - Android: `app/build`, `build`, `.gradle`

- **Interactive TUI Dashboard**

  - 3-pane layout (project tree, details, actions)
  - Multi-selection with spacebar
  - Keyboard-driven navigation (↑/↓, j/k)
  - Sorting modes (size ↑↓, name ↑↓)
  - Filtering by project type
  - Pagination (top 100 results)
  - Emoji icons for project types (🦀 📦 💙 🤖)
  - Safe deletion with confirmation modal

- **Distribution & Installation**

  - Single-line install script for Linux/macOS (Bash)
  - Single-line install script for Windows (PowerShell)
  - Uninstall script with PATH cleanup
  - GitHub Actions workflow for automated cross-platform builds
  - Homebrew formula (macOS/Linux)
  - Scoop manifest (Windows)
  - Binary optimization (LTO, size optimization, symbol stripping)

- **CLI Features**

  - `--mode` flag: `scan` (stdout) or `tui` (interactive)
  - `--version` / `-v`: Display version information
  - `--help`: Show usage information
  - Positional path argument (defaults to current directory)

- **Documentation**
  - Comprehensive README with installation instructions
  - Keyboard shortcuts reference
  - Performance metrics (48,699 projects, 69.78 GB tested)
  - Release checklist
  - Project walkthrough

### Technical Details

- Rust 2021 edition
- Dependencies: `ratatui`, `crossterm`, `jwalk`, `tokio`, `anyhow`, `clap`
- Minimum Rust version: 1.70+
- Supported platforms: Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), Windows (x86_64)

[Unreleased]: https://github.com/jcyrus/spektr/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/jcyrus/spektr/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/jcyrus/spektr/releases/tag/v0.1.0
