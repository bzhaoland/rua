use std::fs;
use std::path;

use anyhow::{Context, Result};
use crossterm::terminal;
use ratatui::style::Stylize;

use crate::submods::compdb::{CompDB, CompDBRecord};

/// Find corresponding compile command from compilation database for the given filename.
pub fn find_compile_command(filename: &str, compdb: &path::Path) -> Result<Vec<CompDBRecord>> {
    let compdb_str = fs::read_to_string(compdb)
        .context(format!(r#"Can't read file "{}""#, compdb.display()))?;
    let compdb: CompDB = serde_json::from_str(&compdb_str)
        .context(format!(r#"Failed to parse "{}"!"#, compdb.display()))?;

    let commands: Vec<CompDBRecord> = compdb
        .into_iter()
        .filter_map(|x| {
            let file = path::Path::new(x.file.as_str());
            if file.file_name().unwrap().to_str().unwrap() == filename {
                Some(x)
            } else {
                None
            }
        })
        .collect();

    Ok(commands)
}

pub fn print_records(records: &[CompDBRecord]) -> Result<()> {
    if records.is_empty() {
        println!("No matched record.");
        return Ok(());
    }

    let width = terminal::window_size()?.columns;
    let head_decor = "=".repeat(width as usize).green().to_string();
    let data_decor = "-".repeat(width as usize).green().to_string();

    let mut out = String::new();
    out.push_str(&format!(
        "{} matched record{}:\n",
        records.len(),
        if records.len() > 1 { "s" } else { "" }
    ));

    out.push_str(&head_decor);
    for (idx, item) in records.iter().enumerate() {
        out.push_str(&format!(
            "File      : {}\nDirectory : {}\nCommand   : {}\n",
            item.file, item.directory, item.command
        ));

        if idx < records.len() - 1 {
            out.push_str(&data_decor);
        }
    }
    out.push_str(&head_decor);
    out.push_str(&"Run the compile command under the corresponding directory.".yellow().to_string());

    println!("{}", out);

    Ok(())
}
