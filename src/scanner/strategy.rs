use std::path::Path;

/// Risk level for deletion operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// Safe to delete, can be rebuilt easily (e.g., node_modules, target)
    Low,
    /// Cache directories, may slow down next build
    #[allow(dead_code)]
    Medium,
    /// Configuration or state files, requires caution
    #[allow(dead_code)]
    High,
}

/// Trait for cleaning strategies targeting specific project types
pub trait CleaningStrategy: Send + Sync {
    /// Name of the strategy (e.g., "Node.js", "Rust")
    fn name(&self) -> &str;

    /// Detects if a given path represents a project of this type
    /// Usually checks for marker files like package.json, Cargo.toml
    fn detect(&self, path: &Path) -> bool;

    /// Returns the list of target directories to clean
    fn targets(&self) -> Vec<&str>;

    /// Risk level for deleting this project's artifacts
    fn risk_level(&self) -> RiskLevel;

    /// Optional: estimate rebuild time as a string
    #[allow(dead_code)]
    fn rebuild_estimate(&self) -> &str {
        "~1-3 mins"
    }
}

// === Node.js Strategy ===

pub struct NodeStrategy;

impl CleaningStrategy for NodeStrategy {
    fn name(&self) -> &str {
        "Node.js"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("package.json").exists()
    }

    fn targets(&self) -> Vec<&str> {
        vec!["node_modules", ".next", "dist", "build"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn rebuild_estimate(&self) -> &str {
        "~1-2 mins (npm install)"
    }
}

// === Rust Strategy ===

pub struct RustStrategy;

impl CleaningStrategy for RustStrategy {
    fn name(&self) -> &str {
        "Rust"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("Cargo.toml").exists()
    }

    fn targets(&self) -> Vec<&str> {
        vec!["target"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn rebuild_estimate(&self) -> &str {
        "~2-5 mins (cargo build)"
    }
}

// === Flutter Strategy ===

pub struct FlutterStrategy;

impl CleaningStrategy for FlutterStrategy {
    fn name(&self) -> &str {
        "Flutter"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("pubspec.yaml").exists()
    }

    fn targets(&self) -> Vec<&str> {
        vec!["build", ".dart_tool"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn rebuild_estimate(&self) -> &str {
        "~1-3 mins (flutter pub get + build)"
    }
}

// === Android Strategy ===

pub struct AndroidStrategy;

impl CleaningStrategy for AndroidStrategy {
    fn name(&self) -> &str {
        "Android"
    }

    fn detect(&self, path: &Path) -> bool {
        path.join("build.gradle").exists() || path.join("build.gradle.kts").exists()
    }

    fn targets(&self) -> Vec<&str> {
        vec!["app/build", "build", ".gradle"]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn rebuild_estimate(&self) -> &str {
        "~3-10 mins (gradle build)"
    }
}

/// Factory function to create all built-in strategies
pub fn default_strategies() -> Vec<Box<dyn CleaningStrategy>> {
    vec![
        Box::new(NodeStrategy),
        Box::new(RustStrategy),
        Box::new(FlutterStrategy),
        Box::new(AndroidStrategy),
    ]
}
