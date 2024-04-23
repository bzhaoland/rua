use std::fs;
use std::io::{self, Write};
use std::process;

use anyhow::Context;
use anyhow::Error as AnyError;
use anyhow::Result as AnyResult;
use colored::Colorize;
use regex;

pub fn clean_build() -> AnyResult<()> {
    let num_steps: usize = 2;
    let mut curr_step: usize;

    // Clean the target directory
    curr_step = 1;
    print!("[{}/{}] REMOVING TARGET DIRECTORY...", curr_step, num_steps);
    fs::remove_dir_all("target")?;
    println!("{}", "DONE".green());

    // Clean the unversioned entries
    curr_step = 2;
    print!(
        "[{}/{}] FINDING UNVERSIONED ENTRIES...",
        curr_step, num_steps
    );
    io::stdout().flush()?;
    let output = process::Command::new("svn")
        .args(["status", "src"])
        .output()
        .with_context(|| {
            println!("{}", "FAILED".red());
            format!("Failed to exec `svn status src`")
        })?;

    if !output.status.success() {
        println!("{}", "FAILED".red());
        return Err(AnyError::msg("Error: Failed to execute `svn status src`"));
    }
    let file_pattern =
        regex::Regex::new(r#"\?\s+(\S+)\n"#).with_context(|| format!("Error regex pattern"))?;
    let output_str = String::from_utf8(output.stdout).with_context(|| format!("Failed to convert output to String type"))?;
    let mut filelist = Vec::new();
    for (_, [file]) in file_pattern.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
    }
    println!("[{}/{}] FINDING UNVERSIONED ENTRIES...{}, {} FILES FOUND", curr_step, num_steps, "DONE".green(), filelist.len());

    Ok(())
}
