use std::fs::File;
use std::fmt::Display;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct CompDBRecord {
    pub command: String,
    pub directory: String,
    pub file: String,
}

impl Display for CompDBRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r"{{ command: {}, directory: {}, file: {} }}", self.command, self.directory, self.file)
    }
}

type CompDB = Vec<CompDBRecord>;

/// Fetch compile command for a specific file from the compilation database.
pub fn fetch_compile_command(filename: &str) -> Result<Vec<String>> {
    let jcdb_file = File::open("./compile_commands.json").context("CompDB not found!")?;
    let compdb: CompDB = serde_json::from_reader(jcdb_file).context("CompDB parsed error!")?;
    let mut commands = vec![];

    for item in compdb.iter() {
        if item.file.as_str() == filename {
            commands.push(item.command.clone());
        }
    }

    Ok(commands)
}