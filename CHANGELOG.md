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

[Unreleased]: https://github.com/jcyrus/spektr/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jcyrus/spektr/releases/tag/v0.1.0
