use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use anyhow::{Context, Error, Result};
use crossterm::style::Stylize;
use regex::Regex;

pub fn clean_build() -> Result<()> {
    const NSTEPS: usize = 2;
    let mut step: usize = 1;

    // Remove the whole target directory
    print!("[{}/{}] REMOVING TARGET DIRECTORY...", step, NSTEPS);
    io::stdout().flush()?;
    fs::remove_dir_all("target").map_err(|e| {
        println!("{}", "FAILED".red());
        e
    })?;
    println!(
        "\r[{}/{}] REMOVING TARGET DIRECTORY...{}\x1B[0K",
        step,
        NSTEPS,
        "OK".green()
    );

    // Clean unversioned entries
    step = 2;
    print!("[{}/{}] FINDING UNVERSIONEDS...", step, NSTEPS);
    io::stdout().flush()?;
    let output = process::Command::new("svn")
        .args(["status", "src", "bin", "lib"])
        .output()
        .with_context(|| {
            println!("{}", "FAILED".red());
            "Failed to exec `svn status src`"
        })?;

    if !output.status.success() {
        println!("{}", "FAILED".red());
        return Err(Error::msg("Error: Failed to execute `svn status src`"));
    }
    let file_pattern = Regex::new(r#"\?\s+(\S+)\n"#).with_context(|| "Error regex pattern")?;
    let output_str = String::from_utf8(output.stdout)
        .with_context(|| "Failed to convert output to `String` type")?;
    let mut filelist = Vec::new();
    for (_, [file]) in file_pattern.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
    }
    print!(
        "\r[{}/{}] FINDING UNVERSIONEDS...{}\x1B[0K",
        step,
        NSTEPS,
        filelist.len().to_string().green()
    );
    io::stdout().flush()?;

    print!(
        "\r[{}/{}]CLEANING UNVERSIONEDS...0/{}\x1B[0K",
        step,
        NSTEPS,
        filelist.len().to_string().green()
    );
    io::stdout().flush()?;
    for (idx, item) in filelist.iter().enumerate() {
        let entry = PathBuf::from(item);
        if entry.is_file() || entry.is_symlink() {
            fs::remove_file(item)?;
        } else if entry.is_dir() {
            fs::remove_dir_all(entry)?;
        }
        print!(
            "\r[{}/{}]CLEANING UNVERSIONEDS...{}/{}\x1B[0K",
            step,
            NSTEPS,
            idx + 1,
            filelist.len().to_string().green()
        );
        io::stdout().flush()?;
    }
    println!(
        "\r[{}/{}] CLEANING UNVERSIONEDS...{}\x1B[0K",
        step,
        NSTEPS,
        "OK".green()
    );

    Ok(())
}
