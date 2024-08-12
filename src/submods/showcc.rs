use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use crossterm::{style::Stylize, terminal};

use crate::submods::compdb::{CompDB, CompDBRecord};

/// Fetch the corresponding compile command from compilation database for the given filename.
pub fn fetch_compile_command(filename: &str) -> Result<Vec<CompDBRecord>> {
    let compdb_str = fs::read_to_string("./compile_commands.json")?;
    let compdb: CompDB = serde_json::from_str(&compdb_str).context("CompDB parsed error!")?;
    let mut commands = vec![];

    for item in compdb.into_iter() {
        let file = Path::new(item.file.as_str());
        let basename = file
            .file_name()
            .context("Error path format")?
            .to_str()
            .context("Invalid Unicode")?;
        if basename == filename {
            commands.push(item);
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

    out.push_str(&format!("{}\n", head_decor.as_str().dark_green()));

    for (idx, item) in records.iter().enumerate() {
        out.push_str(&format!(
            "File      : {}\nDirectory : {}\nCommand   : {}\n",
            item.file, item.directory, item.command
        ));

        if idx < records.len() - 1 {
            out.push_str(&format!("{}\n", data_decor.as_str().dark_green()));
        }
    }

    out.push_str(&format!("{}\n", head_decor.as_str().dark_green()));

    out.push_str(&format!(
        "{}",
        "Run the compile command under the corresponding directory.".dark_yellow()
    ));

    println!("{}", out);

    Ok(())
}
