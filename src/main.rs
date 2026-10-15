use clap::{Parser, ValueEnum};
use langstats::output::{Filter, Sort as ReportSort, build_report, terminal};
use langstats::scanner::{ScanOptions, scan};
use std::io::IsTerminal;
use std::path::PathBuf;

/// Fast, offline language statistics for local directories.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Directory to analyze
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Emit versioned JSON
    #[arg(long, conflicts_with = "format")]
    json: bool,
    /// Output format
    #[arg(long, value_enum, default_value_t = Format::Terminal)]
    format: Format,
    /// Show at most N languages
    #[arg(long)]
    top: Option<usize>,
    /// Worker count (1 to 256)
    #[arg(long)]
    threads: Option<usize>,
    /// Include files matched by ignore rules
    #[arg(long)]
    include_ignored: bool,
    /// Include hidden files and directories
    #[arg(long)]
    include_hidden: bool,
    /// Disable ANSI colors
    #[arg(long)]
    no_color: bool,
    /// Suppress terminal headers and warnings
    #[arg(long, conflicts_with = "verbose")]
    quiet: bool,
    /// Show scan diagnostics on stderr
    #[arg(long)]
    verbose: bool,
    /// Sort languages
    #[arg(long, value_enum, default_value_t = Sort::Bytes)]
    sort: Sort,
    /// Hide languages below this percent (0 to 100)
    #[arg(long, default_value_t = 0.0)]
    min_percent: f64,
    /// Hide languages below this many bytes
    #[arg(long, default_value_t = 0)]
    min_bytes: u64,
    /// Exit nonzero when no source bytes were counted
    #[arg(long)]
    fail_on_empty: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Terminal,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum Sort {
    Bytes,
    Percentage,
    Name,
    Files,
}

fn main() {
    let cli = Cli::parse();
    if cli.threads.is_some_and(|n| !(1..=256).contains(&n)) {
        eprintln!("langstats: --threads must be between 1 and 256");
        std::process::exit(2);
    }
    if !cli.min_percent.is_finite() || !(0.0..=100.0).contains(&cli.min_percent) {
        eprintln!("langstats: --min-percent must be between 0 and 100");
        std::process::exit(2);
    }
    let options = ScanOptions {
        threads: cli
            .threads
            .unwrap_or_else(|| ScanOptions::default().threads),
        include_ignored: cli.include_ignored,
        include_hidden: cli.include_hidden,
    };
    let result = match scan(&cli.path, &options) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("langstats: {error}");
            std::process::exit(2);
        }
    };
    let path = cli.path.to_string_lossy();
    let name = cli
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(".");
    let report = build_report(
        &result,
        &path,
        name,
        Filter {
            top: cli.top,
            min_percent: cli.min_percent,
            min_bytes: cli.min_bytes,
            sort: match cli.sort {
                Sort::Bytes | Sort::Percentage => ReportSort::Bytes,
                Sort::Name => ReportSort::Name,
                Sort::Files => ReportSort::Files,
            },
        },
    );
    if cli.json || matches!(cli.format, Format::Json) {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("langstats: JSON serialization failed: {error}");
                std::process::exit(1);
            }
        }
    } else {
        let color = !cli.no_color
            && std::env::var_os("NO_COLOR").is_none()
            && std::io::stdout().is_terminal();
        print!("{}", terminal(&report, color, cli.quiet));
    }
    if (!cli.quiet || cli.verbose) && !result.warnings.is_empty() {
        eprintln!(
            "langstats: {} scan warning(s); first: {}",
            result.warnings.len() + result.skipped_warnings as usize,
            result.warnings[0]
        );
    }
    if cli.fail_on_empty && report.summary.total_bytes == 0 {
        std::process::exit(1);
    }
}
