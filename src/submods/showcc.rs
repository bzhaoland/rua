use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::{style::Stylize, terminal};
use regex::Regex;

use crate::submods::compdb::{CompDB, CompDBRecord};

/// Fetch the corresponding compile command from compilation database for the given filename.
pub fn fetch_compile_command(filename: &str) -> Result<Vec<CompDBRecord>> {
    let compdb_str = fs::read_to_string("./compile_commands.json")?;
    let compdb: CompDB = serde_json::from_str(&compdb_str).context("CompDB parsed error!")?;
    let mut commands = vec![];

    for item in compdb.iter() {
        let file = PathBuf::from(item.file.as_str());
        let tmp = file
            .file_name()
            .context("Record format error!")?
            .to_str()
            .context("Not valid Unicode")?;
        if tmp == filename {
            let pattern = Regex::new(r"-M(?:[DGMP]|MD|no-modules|[FTQ]\s*\S+)?\s+")?;
            let new_command = pattern.replace(&item.command, "").to_string();
            commands.push(CompDBRecord {
                command: new_command,
                directory: item.directory.clone(),
                file: item.file.clone(),
            });
        }
    }

    Ok(commands)
}

pub fn print_records(records: &[CompDBRecord]) -> Result<()> {
    if records.is_empty() {
        println!("No matched record.");
        return Ok(());
    }

    let width = terminal::window_size()?.columns;
    let head_decor = "=".repeat(width as usize);
    let data_decor = "-".repeat(width as usize);

    let mut out = String::new();
    out.push_str(&format!(
        "{} matched record{}:\n",
        records.len(),
        if records.len() > 1 { "s" } else { "" }
    ));

    out.push_str(&format!("{}\n", head_decor.as_str().green()));

    for (idx, item) in records.iter().enumerate() {
        out.push_str(&format!(
            "File      : {}\nDirectory : {}\nCommand   : {}\n",
            item.file, item.directory, item.command
        ));

        if idx < records.len() - 1 {
            out.push_str(&format!("{}\n", data_decor.as_str().green()));
        }
    }

    out.push_str(&format!("{}\n", head_decor.as_str().green()));

    out.push_str(&format!(
        "{}\n",
        "Run compile command under corresponding directory.".yellow()
    ));

    print!("{}", out);

    Ok(())
}
