pub mod analysis;
pub mod cli;
pub mod duplicate;
pub mod model;
pub mod output;
pub mod platform;
pub mod scanner;
pub mod util;

pub mod report;

use std::path::Path;
use std::time::Instant;

use cli::Config;
use duplicate::DuplicateResult;
use scanner::{Progress, ScanResult, Scanner};

pub fn run(config: Config) -> Result<(), String> {
    let start = Instant::now();
    let mut progress = Progress::new(config.show_progress);
    let result = Scanner::new(&config).scan(&mut progress)?;
    progress.finish();

    let elapsed = start.elapsed();
    let duplicates = if config.find_duplicates {
        Some(duplicate::find_duplicates(&result.entries))
    } else {
        None
    };

    render_and_write(&config, &result, duplicates.as_ref(), elapsed)?;
    Ok(())
}

fn render_and_write(
    config: &Config,
    result: &ScanResult,
    duplicates: Option<&DuplicateResult>,
    elapsed: std::time::Duration,
) -> Result<(), String> {
    use report::{csv, file_list, json, markdown, summary, tree};

    if !config.quiet {
        print!("{}", tree::render(&result.tree, config));
        println!();
    }

    if config.show_full_table || config.output.is_some() {
        let detailed = summary::render_detailed(&config.root, &result.entries);
        if config.show_full_table {
            print!("{detailed}");
            println!();
        }
        if let Some(path) = &config.output {
            output::write_text_file(path, &detailed, config.utf8_bom)?;
            eprintln!("(full report written to {})", path.display());
        }
    }

    let dirs = analysis::top_dirs(&result.tree, &config.root, config.top_n);
    let summary_report = summary::render_summary(
        &config.root,
        &result.stats,
        &result.entries,
        &dirs,
        config.top_n,
        elapsed,
        duplicates,
    );
    print!("{summary_report}");

    if let Some(path) = &config.markdown {
        let markdown_report = markdown::render(
            config,
            &result.tree,
            &result.stats,
            &result.entries,
            &dirs,
            duplicates,
            elapsed,
        );
        output::write_text_file(path, &markdown_report, config.utf8_bom)?;
        eprintln!("(markdown report written to {})", path.display());
    }

    if let Some(path) = &config.json {
        let json_report = json::render(config, &result.stats, &result.entries, duplicates, elapsed);
        output::write_text_file(path, &json_report, false)?;
        eprintln!("(JSON report written to {})", path.display());
    }

    if let Some(path) = &config.csv {
        let csv_report = csv::render(&config.root, &result.entries);
        output::write_text_file(path, &csv_report, true)?;
        eprintln!("(CSV report written to {})", path.display());
    }

    if let Some(path) = &config.file_list {
        let list = file_list::render(&config.root, &result.entries);
        output::write_text_file(path, &list, true)?;
        eprintln!("(file list written to {})", path.display());
    }

    Ok(())
}

pub fn validate_root(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("path does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }
    Ok(())
}
