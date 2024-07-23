use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use anyhow::{Context, Error, Result};
use crossterm::style::Stylize;
use regex::Regex;
use walkdir::WalkDir;

pub fn clean_build() -> Result<()> {
    const NSTEPS: usize = 2;
    let mut step: usize = 1;

    print!("[{}/{}] FINDING TARGET OBJECTS...", step, NSTEPS);
    io::stdout().flush()?;
    let mut num_entries = 0;
    for (idx, _) in WalkDir::new("target")
        .contents_first(true)
        .into_iter()
        .enumerate()
    {
        num_entries += 1;
        print!(
            "\r[{}/{}] FINDING TARGET OBJECTS...{}\x1B[0K",
            step,
            NSTEPS,
            idx + 1
        );
        io::stdout().flush()?;
    }
    print!(
        "\r[{}/{}] FINDING TARGET OBJECTS...{}\x1B[0K",
        step,
        NSTEPS,
        num_entries.to_string().yellow()
    );
    io::stdout().flush()?;

    // Remove the whole target directory
    print!(
        "\r[{}/{}] REMOVING TARGET OBJECTS...{}/{}\x1B[0K",
        step,
        NSTEPS,
        "0".green(),
        num_entries.to_string().yellow()
    );
    io::stdout().flush()?;
    for (idx, entry) in WalkDir::new("target")
        .contents_first(true)
        .into_iter()
        .enumerate()
    {
        let entry = entry.unwrap();
        let entry = entry.into_path();
        if entry.is_file() || entry.is_symlink() {
            fs::remove_file(entry)?;
        } else if entry.is_dir() {
            fs::remove_dir_all(entry)?;
        }
        print!(
            "\r[{}/{}] REMOVING TARGET OBJECTS...{}/{}\x1B[0K",
            step,
            NSTEPS,
            (idx + 1).to_string().green(),
            num_entries.to_string().yellow()
        );
        io::stdout().flush()?;
    }
    println!(
        "\r[{}/{}] REMOVING TARGET OBJECTS...{}\x1B[0K",
        step,
        NSTEPS,
        "DONE".green()
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
        "\r[{}/{}] CLEANING UNVERSIONEDS...{}/{}\x1B[0K",
        step,
        NSTEPS,
        "0".green(),
        filelist.len().to_string().yellow()
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
            "\r[{}/{}] CLEANING UNVERSIONEDS...{}/{}\x1B[0K",
            step,
            NSTEPS,
            (idx + 1).to_string().green(),
            filelist.len().to_string().yellow()
        );
        io::stdout().flush()?;
    }
    println!(
        "\r[{}/{}] CLEANING UNVERSIONEDS...{}\x1B[0K",
        step,
        NSTEPS,
        "DONE".green()
    );

    Ok(())
}
