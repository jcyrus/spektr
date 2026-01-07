# Contributing to SPEKTR

First off, thank you for considering contributing to SPEKTR! 🎉

It's people like you that make SPEKTR such a great tool for developers.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Your First Contribution](#your-first-contribution)
- [Development Setup](#development-setup)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Adding New Cleaning Strategies](#adding-new-cleaning-strategies)
- [Commit Message Guidelines](#commit-message-guidelines)

## Code of Conduct

This project and everyone participating in it is governed by our commitment to fostering an open and welcoming environment. We pledge to make participation in our project a harassment-free experience for everyone.

### Our Standards

- **Be respectful** of differing viewpoints and experiences
- **Give and accept constructive feedback** gracefully
- **Focus on what is best** for the community
- **Show empathy** towards other community members

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the [existing issues](https://github.com/jcyrus/spektr/issues) to avoid duplicates.

When creating a bug report, include:

- **A clear title and description**
- **Steps to reproduce** the issue
- **Expected behavior** vs. actual behavior
- **System information** (OS, Rust version)
- **Error messages or screenshots** if applicable

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion:

- **Use a clear and descriptive title**
- **Provide a detailed description** of the proposed functionality
- **Explain why this enhancement would be useful** to most users
- **List similar features** in other tools if applicable

### Your First Code Contribution

Unsure where to begin? Look for issues tagged with:

- `good first issue` - Small, straightforward tasks
- `help wanted` - Issues that need community support

## Development Setup

### Prerequisites

- **Rust 1.70+** (install via [rustup](https://rustup.rs/))
- **Git**
- A terminal with TrueColor support
- (Optional) NerdFonts for better icon display

### Setup Instructions

1. **Fork and clone the repository:**

   ```bash
   git clone https://github.com/YOUR-USERNAME/spektr.git
   cd spektr
   ```

2. **Build the project:**

   ```bash
   cargo build
   ```

3. **Run tests (when added):**

   ```bash
   cargo test
   ```

4. **Run the TUI:**

   ```bash
   cargo run
   ```

5. **Run in scan-only mode:**
   ```bash
   cargo run -- --mode scan
   ```

## Pull Request Process

1. **Create a new branch** from `main`:

   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following our [coding standards](#coding-standards)

3. **Test your changes** thoroughly:

   - Ensure the project builds: `cargo build --release`
   - Test the TUI manually
   - Verify your changes don't break existing functionality

4. **Update documentation**:

   - Update `README.md` if you added features
   - Update `CHANGELOG.md` under `[Unreleased]`
   - Add inline documentation for new functions

5. **Commit your changes** using [conventional commits](#commit-message-guidelines)

6. **Push to your fork**:

   ```bash
   git push origin feature/your-feature-name
   ```

7. **Open a Pull Request**:
   - Use a clear title describing the change
   - Reference any related issues
   - Describe what changed and why
   - Add screenshots for UI changes

### PR Review Process

- A maintainer will review your PR within 1-2 weeks
- Address any requested changes
- Once approved, a maintainer will merge your PR

## Coding Standards

### Rust Style Guide

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- No `any` types - use strict typing
- Add doc comments (`///`) for public APIs

### Code Organization

```
src/
├── scanner/          # Scanning engine
│   ├── mod.rs       # Scanner implementation
│   └── strategy.rs  # Cleaning strategies
├── tui/             # Terminal UI
│   ├── mod.rs       # Event loop
│   ├── app_state.rs # State management
│   ├── events.rs    # Input handling
│   ├── layout.rs    # Layout definition
│   └── widgets.rs   # Custom widgets
└── main.rs          # CLI entry point
```

### Performance Considerations

- Use `jwalk` for parallel directory traversal
- Avoid blocking the TUI event loop
- Use channels for progress updates
- Implement pagination for large result sets

## Adding New Cleaning Strategies

Want to add support for a new language/framework? Here's how:

### 1. Implement the `CleaningStrategy` trait

```rust
// In src/scanner/strategy.rs

pub struct YourStrategy;

impl CleaningStrategy for YourStrategy {
    fn name(&self) -> &str {
        "Your Language"
    }

    fn detect(&self, path: &Path) -> bool {
        // Check for marker file (e.g., package.json, Cargo.toml)
        path.join("your-marker-file").exists()
    }

    fn targets(&self) -> Vec<&str> {
        // Directories to clean
        vec!["build", "cache"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low  // or Medium/High
    }

    fn rebuild_estimate(&self) -> &str {
        "~2-4 mins (your build command)"
    }
}
```

### 2. Add to default strategies

```rust
// In src/scanner/strategy.rs

pub fn default_strategies() -> Vec<Box<dyn CleaningStrategy>> {
    vec![
        // ... existing strategies
        Box::new(YourStrategy),
    ]
}
```

### 3. Add emoji icon

```rust
// In src/tui/widgets.rs, in render_project_tree function

let emoji = match project.strategy_name.as_str() {
    // ... existing emojis
    "Your Language" => "🎯",  // Choose appropriate emoji
    _ => "📁",
};
```

### 4. Update documentation

- Add to README.md "Supported Project Types" table
- Add to CHANGELOG.md under `[Unreleased]`

## Commit Message Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/):

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic change)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process, dependencies, tooling

### Examples

```
feat(scanner): add Python virtual environment support

Implements CleaningStrategy for Python projects, detecting .venv
directories and __pycache__ folders.

Closes #42
```

```
fix(tui): prevent crash when no projects found

Added check for empty project list before rendering to avoid
index out of bounds error.

Fixes #15
```

```
docs(readme): update installation instructions

Added instructions for ARM64 Linux and updated Homebrew
installation steps.
```

## Questions?

Feel free to open an issue with the `question` label, or reach out to [@jcyrus](https://github.com/jcyrus).

---

**Thank you for contributing to SPEKTR!** 🧹✨
