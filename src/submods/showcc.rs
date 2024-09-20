use std::fs;
use std::path;

use anyhow::{Context, Result};
use crossterm::terminal;

use crate::submods::compdb::{CompDB, CompDBRecord};

const COLOR_ANSI_GRN: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));
const COLOR_ANSI_YLW: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));

/// Fetch the corresponding compile command from compilation database for the given filename.
pub fn fetch_compile_command(filename: &str, compdb: &path::Path) -> Result<Vec<CompDBRecord>> {
    let compdb_str = fs::read_to_string(compdb)
        .context(format!(r#"Error reading file "{}""#, compdb.display()))?;
    let compdb: CompDB = serde_json::from_str(&compdb_str)
        .context(format!(r#"Error parsing "{}"!"#, compdb.display()))?;

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
    let head_decor = format!(
        "{}{}{:#}",
        COLOR_ANSI_GRN,
        "=".repeat(width as usize),
        COLOR_ANSI_GRN
    );
    let data_decor = format!(
        "{}{}{:#}",
        COLOR_ANSI_GRN,
        "-".repeat(width as usize),
        COLOR_ANSI_GRN
    );

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

    out.push_str(&format!(
        "{}{}{:#}",
        COLOR_ANSI_YLW,
        "Run the compile command under the corresponding directory.",
        COLOR_ANSI_YLW
    ));

    println!("{}", out);

    Ok(())
}
