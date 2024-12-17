use std::fs;
use std::path;

use anstyle::{AnsiColor, Color, Style};
use anyhow::{Context, Result};
use crossterm::terminal;

use crate::submods::compdb::{CompDB, CompDBRecord};

const STYLE_GREEN: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Green)))
    .bold();
const STYLE_YELLOW: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
    .bold();

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
    let head_decor = format!("{STYLE_GREEN}{}{STYLE_GREEN:#}", "=".repeat(width as usize));
    let data_decor = format!("{STYLE_GREEN}{}{STYLE_GREEN:#}", "-".repeat(width as usize));

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
    out.push_str(&format!("{STYLE_YELLOW}Compile command should be run under the corresponding directory.{STYLE_YELLOW:#}"));

    println!("{}", out);

    Ok(())
}
