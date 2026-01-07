mod scanner;
mod tui;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use scanner::{Scanner, ScanEvent};
use scanner::strategy::default_strategies;
use std::env;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

#[derive(Parser)]
#[command(name = "spektr")]
#[command(about = "A blazing-fast TUI utility for cleaning development artifacts", long_about = None)]
struct Cli {
    /// Directory to scan (defaults to current directory)
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Run mode: scan output or interactive TUI
    #[arg(short, long, value_enum, default_value = "tui")]
    mode: Mode,

    /// Dry run (scan only, no deletion)
    #[arg(long)]
    dry_run: bool,

    /// Show version information
    #[arg(short = 'v', long)]
    version: bool,
}

#[derive(Clone, ValueEnum)]
enum Mode {
    /// Simple scan mode (prints to stdout)
    Scan,
    /// Interactive TUI mode
    Tui,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Show version and exit
    if cli.version {
        println!("spektr {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let scan_path = cli
        .path
        .unwrap_or_else(|| env::current_dir().expect("Failed to get current directory"));

    match cli.mode {
        Mode::Scan => run_scan_mode(&scan_path),
        Mode::Tui => run_tui_mode(&scan_path, cli.dry_run),
    }
}

fn run_scan_mode(scan_path: &PathBuf) -> Result<()> {
    println!("🔍 SPEKTR - Scanning: {}", scan_path.display());
    println!();

    let (tx, rx) = mpsc::channel();
    let tx_clone = tx.clone();
    let scan_path_clone = scan_path.clone();

    let handle = thread::spawn(move || {
        let scanner = Scanner::new(default_strategies());
        scanner.scan(&scan_path_clone, tx_clone)
    });

    let mut total_size = 0u64;
    let mut project_count = 0;

    for event in rx {
        match event {
            ScanEvent::ProjectFound(project) => {
                project_count += 1;
                total_size += project.total_size;

                let emoji = match project.strategy_name.as_str() {
                    "Rust" => "🦀",
                    "Node.js" => "📦",
                    "Flutter" => "💙",
                    "Android" => "🤖",
                    _ => "📁",
                };

                println!(
                    "{} {} | {} | {}",
                    emoji,
                    project.strategy_name,
                    project.root_path.display(),
                    format_size(project.total_size)
                );
            }
            ScanEvent::Complete => break,
        }
    }

    handle.join().unwrap()?;

    println!();
    println!("✅ Scan Complete!");
    println!("   Projects Found: {}", project_count);
    println!("   Total Reclaimable: {}", format_size(total_size));

    Ok(())
}

fn run_tui_mode(scan_path: &PathBuf, _dry_run: bool) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let scan_path_clone = scan_path.clone();

    // Spawn scanner in background thread
    thread::spawn(move || {
        let scanner = Scanner::new(default_strategies());
        let _ = scanner.scan(&scan_path_clone, tx);
    });

    // Run TUI (blocks until user quits)
    let final_state = tui::run_tui(rx)?;

    // Handle deletion if user confirmed
    if final_state.deletion_confirmed {
        let selected = final_state.get_selected_projects();
        println!("\n🗑️  Deleting {} projects...", selected.len());

        for project in selected {
            println!("   Deleting: {}", project.root_path.display());
            for target in &project.targets {
                if target.exists() {
                    std::fs::remove_dir_all(target)?;
                }
            }
        }

        println!("✅ Cleanup complete!");
    } else {
        println!("\n👋 Exited without making changes.");
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
