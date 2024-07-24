use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use anyhow::{anyhow, bail, Context, Result};
use crossterm::style::Stylize;
use regex::Regex;
use walkdir::WalkDir;

pub fn clean_build() -> Result<()> {
    // Check location
    let out = process::Command::new("svn")
        .arg("info")
        .output()
        .context(anyhow!("Failed to run `svn info`"))?;
    if !out.status.success() {
        bail!(anyhow!(String::from_utf8_lossy(&out.stderr).to_string())
            .context("Failed to run `svn info`"));
    }
    let output = String::from_utf8_lossy(&out.stdout);
    let pattern: Regex = Regex::new(r#"Relative URL: \^/branches/([\w-]+)\n"#).unwrap();
    let caps = pattern
        .captures(&output)
        .context("Error location! Run command under project root.")?;
    let branch = caps.get(1).context("Error fetching branch info")?.as_str();

    let nsteps: usize = if PathBuf::from(branch).is_dir() { 3 } else { 2 };
    let mut step: usize = 1;

    // Cleaning the objects generated in building process
    print!("[{}/{}] FINDING TARGET OBJS...", step, nsteps);
    io::stdout().flush()?;
    let target_dir = PathBuf::from("target");
    if target_dir.is_dir() {
        let mut num_entries = 0;
        for (idx, _) in WalkDir::new(target_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            print!(
                "\r[{}/{}] FINDING TARGET OBJS...{}\x1B[0K",
                step,
                nsteps,
                idx + 1
            );
            io::stdout().flush()?;
        }
        print!(
            "\r[{}/{}] FINDING TARGET OBJS...{}\x1B[0K",
            step,
            nsteps,
            num_entries.to_string().yellow()
        );
        io::stdout().flush()?;

        // Remove the whole target directory
        print!(
            "\r[{}/{}] CLEANING TARGET OBJS...{}/{}\x1B[0K",
            step,
            nsteps,
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
                "\r[{}/{}] CLEANING TARGET OBJS...{}/{}\x1B[0K",
                step,
                nsteps,
                (idx + 1).to_string().green(),
                num_entries.to_string().yellow()
            );
            io::stdout().flush()?;
        }
    }
    println!(
        "\r[{}/{}] CLEANING TARGET OBJS...{}\x1B[0K",
        step,
        nsteps,
        "DONE".green()
    );

    // Clean unversioned entries
    step += 1;
    print!("[{}/{}] FINDING UNVERSIONEDS...", step, nsteps);
    io::stdout().flush()?;
    let output = process::Command::new("svn")
        .args(["status", "src", "bin", "lib"])
        .output()
        .with_context(|| {
            println!("{}", "FAILED".red());
            "Failed to run `svn status src`"
        })?;

    if !output.status.success() {
        println!("{}", "FAILED".red());
        bail!("Failed to run `svn status src`");
    }
    let file_pattern = Regex::new(r#"\?\s+(\S+)\n"#).with_context(|| "Error regex pattern")?;
    let output_str = String::from_utf8(output.stdout)
        .context(anyhow!("Error converting output to `String` type"))?;
    let mut filelist = Vec::new();
    for (_, [file]) in file_pattern.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
    }
    print!(
        "\r[{}/{}] FINDING UNVERSIONEDS...{}\x1B[0K",
        step,
        nsteps,
        filelist.len().to_string().green()
    );
    io::stdout().flush()?;

    print!(
        "\r[{}/{}] CLEANING UNVERSIONEDS...{}/{}\x1B[0K",
        step,
        nsteps,
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
            nsteps,
            (idx + 1).to_string().green(),
            filelist.len().to_string().yellow()
        );
        io::stdout().flush()?;
    }
    println!(
        "\r[{}/{}] CLEANING UNVERSIONEDS...{}\x1B[0K",
        step,
        nsteps,
        "DONE".green()
    );

    // Clean UI files
    let ui_dir = PathBuf::from(branch); // UI directory name is the same as the branch name
    if ui_dir.is_dir() {
        step += 1;

        print!("[{}/{}] FINDING UI OBJS...", step, nsteps);
        io::stdout().flush()?;
        
        let mut num_entries = 0;
        for (idx, _) in WalkDir::new( &ui_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            print!(
                "\r[{}/{}] FINDING UI OBJS...{}\x1B[0K",
                step,
                nsteps,
                idx + 1
            );
            io::stdout().flush()?;
        }
        print!(
            "\r[{}/{}] FINDING UI OBJS...{}\x1B[0K",
            step,
            nsteps,
            num_entries.to_string().yellow()
        );
        io::stdout().flush()?;

        // Remove the whole target directory
        print!(
            "\r[{}/{}] CLEANING UI OBJS...{}/{}\x1B[0K",
            step,
            nsteps,
            "0".green(),
            num_entries.to_string().yellow()
        );
        io::stdout().flush()?;
        for (idx, entry) in WalkDir::new(&ui_dir)
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
                "\r[{}/{}] CLEANING UI OBJS...{}/{}\x1B[0K",
                step,
                nsteps,
                (idx + 1).to_string().green(),
                num_entries.to_string().yellow()
            );
            io::stdout().flush()?;
        }

        println!(
            "\r[{}/{}] CLEANING UI OBJS...{}\x1B[0K",
            step,
            nsteps,
            "DONE".green()
        );
    }

    Ok(())
}
