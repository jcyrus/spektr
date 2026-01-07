<div align="center">

# SPEKTR

### The Dev Cleaner 🧹

**Mission Control for your Disk Space.** _A blazing-fast, TUI-based artifact cleaner for developers._

[![License](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/jcyrus/spektr/actions/workflows/release.yml/badge.svg)](https://github.com/jcyrus/spektr/actions/workflows/release.yml)
[![Version](https://img.shields.io/github/v/release/jcyrus/spektr)](https://github.com/jcyrus/spektr/releases)
![Platform](https://img.shields.io/badge/platform-macos%20|%20linux%20|%20windows-lightgrey)

[Installation](#installation) • [Usage](#usage) • [Features](#features) • [Contributing](#contributing)

</div>

---

## ⚡ What is SPEKTR?

SPEKTR is a precision instrument for your terminal. It recursively scans your development workspaces to identify heavy build artifacts (`node_modules`, `target`, `build`, etc.) and lets you "jettison" gigabytes of digital waste in seconds.

Built in **Rust** 🦀, it is multi-threaded, memory-safe, and designed to handle monorepos with 50,000+ folders without breaking a sweat.

## 🚀 Features

- **Blazing Fast Scans:** Powered by `jwalk` for parallel directory traversal. Scans gigabytes in milliseconds.
- **Mission Control TUI:** A beautiful 3-pane interface built with `ratatui`.
- **Smart Detection:** Context-aware scanning (only deletes `node_modules` if `package.json` exists).
- **Safety First:** "Dry Run" by default. Confirmation modals prevent accidental nukes.

> [!CAUTION] > **Use responsibly.** Running this tool on your root directory (`/` or `C:\`) is NOT recommended. While SPEKTR detects projects safely, accidental deletion of critical system files is always a risk with any cleaning tool. Stick to your development workspaces (e.g., `~/code`, `~/projects`).

- **Developer Focused:** Filter by project type (Node, Rust, Flutter, Android).
- **Deep Clean:** Handles nested monorepos and workspaces with ease.

## 📦 Installation

### ⚡ Quick Install (Recommended)

**macOS / Linux:**

```bash
curl -sL https://raw.githubusercontent.com/jcyrus/spektr/main/install.sh | bash

```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/jcyrus/spektr/main/install.ps1 | iex

```

### Package Managers

**Homebrew:**

```bash
brew install jcyrus/tap/spektr

```

**Scoop (Windows):**

```bash
scoop bucket add jcyrus https://github.com/jcyrus/scoop-bucket
scoop install spektr

```

### From Source (Rust)

```bash
git clone https://github.com/jcyrus/spektr
cd spektr
cargo install --path .

```

## 🛠 Usage

### Interactive Mode (The Dashboard)

This scans the current directory recursively.

```bash
spektr

```

Scan a specific workspace:

```bash
spektr ~/code/work

```

### Scan-Only Mode (Headless)

Good for quick checks or CI environments.

```bash
spektr --mode scan ~/code/work

```

## ⌨️ Keyboard Shortcuts

| Key         | Action                                          |
| ----------- | ----------------------------------------------- |
| `↑` / `↓`   | Navigate project list                           |
| `Space`     | Toggle selection for deletion                   |
| `Enter`     | **Trigger Cleanup** (Opens Confirmation)        |
| `f`         | **Filter** (Cycle: All → Node → Rust → Flutter) |
| `s`         | **Sort** (Cycle: Path → Size)                   |
| `q` / `Esc` | Quit Application                                |

## 🎯 Supported Stacks

SPEKTR currently supports detection and cleaning for:

| Stack          | Marker File    | Targets Cleaned                          |
| -------------- | -------------- | ---------------------------------------- |
| **Node.js** 📦 | `package.json` | `node_modules`, `.next`, `dist`, `build` |
| **Rust** 🦀    | `Cargo.toml`   | `target/`                                |
| **Flutter** 💙 | `pubspec.yaml` | `build/`, `.dart_tool/`                  |
| **Android** 🤖 | `build.gradle` | `app/build/`, `.gradle/`                 |

_> More stacks (Python, Docker, Go) coming in v0.2.0_

## 📊 Performance Benchmarks

Tested on a MacBook Pro (M3 Max) scanning `~/code`:

- **Projects Scanned:** 48,699
- **Time to Scan:** < 800ms
- **Space Reclaimed:** 69.78 GB
- **UI Responsiveness:** 60 FPS (Pagination enabled for large lists)

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:

- Code of Conduct
- Development setup
- Pull request process
- Adding new cleaning strategies
- Coding standards

To add a new language strategy (e.g., Python), see the "Adding New Cleaning Strategies" section in the guide.

## 📜 Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed history of changes and releases.

## 📄 License

MIT © [JCyrus](https://github.com/jcyrus)

See [LICENSE](LICENSE) for full details.
