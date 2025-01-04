use std::fs;
use std::path;

use anstyle::{AnsiColor, Color, Style};
use anyhow::{Context, Result};
use crossterm::terminal;
use serde::{Deserialize, Serialize};

const STYLE_GREEN: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Green)))
    .bold();
const STYLE_YELLOW: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
    .bold();

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum CommandOrArguments {
    Command { command: String },
    Arguments { arguments: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompdbEntry {
    #[serde(flatten)]
    mixed_compile_command: CommandOrArguments,
    directory: String,
    file: String,
}

pub(crate) type CompDb = Vec<CompdbEntry>;

/// Find corresponding compile command from compilation database for the given filename.
pub(crate) fn show_compile_command(filename: &str, compdb: &path::Path) -> Result<()> {
    let compdb_str =
        fs::read_to_string(compdb).context(format!(r#"Can't read file "{}""#, compdb.display()))?;
    let compdb: CompDb = serde_json::from_str(&compdb_str)
        .context(format!(r#"Failed to parse "{}"!"#, compdb.display()))?;

    let entries: Vec<CompdbEntry> = compdb
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

    if entries.is_empty() {
        println!("No matched record.");
        return Ok(());
    }

    let width = terminal::window_size()?.columns;
    let head_decor = format!("{STYLE_GREEN}{}{STYLE_GREEN:#}", "=".repeat(width as usize));
    let data_decor = format!("{STYLE_GREEN}{}{STYLE_GREEN:#}", "-".repeat(width as usize));

    let mut out = String::new();
    out.push_str(&format!(
        "{} matched record{}:\n",
        entries.len(),
        if entries.len() > 1 { "s" } else { "" }
    ));

    out.push_str(&head_decor);
    for (idx, item) in entries.iter().enumerate() {
        out.push_str(&format!(
            "File      : {}\nDirectory : {}\nCommand   : {}\n",
            item.file,
            item.directory,
            match &item.mixed_compile_command {
                CommandOrArguments::Command { command } => command.to_owned(),
                CommandOrArguments::Arguments { arguments } => arguments.join(" "),
            }
        ));

        if idx < entries.len() - 1 {
            out.push_str(&data_decor);
        }
    }
    out.push_str(&head_decor);
    out.push_str(&format!(
        "{STYLE_YELLOW}Run compile command under corresponding directory.{STYLE_YELLOW:#}"
    ));

    println!("{}", out);

    Ok(())
}
