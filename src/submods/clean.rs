use std::fs;
use std::io::{self, Write};
use std::path;
use std::process;

use anyhow::{self, Context};
use crossterm::style::Stylize;
use regex;
use walkdir;

use crate::utils;

pub fn clean_build() -> anyhow::Result<()> {
    // Must run under the project root
    if !utils::is_at_proj_root()? {
        anyhow::bail!("Location error! Please run command under the project root.");
    }

    let branch = utils::get_svn_branch()?.unwrap();

    let nsteps: usize = if path::Path::new(&branch).is_dir() {
        3
    } else {
        2
    };
    let mut step: usize = 1;

    // Cleaning the objects generated in building process
    print!(
        "[{}/{}] LISTING TARGET OBJS...{}",
        step,
        nsteps,
        "0".yellow()
    );
    io::stdout().flush()?;
    let target_dir = path::Path::new("target");
    if target_dir.is_dir() {
        let mut num_entries = 0;
        for (idx, _) in walkdir::WalkDir::new(target_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            print!(
                "\r[{}/{}] LISTING TARGET OBJS...{}\x1B[0K",
                step,
                nsteps,
                (idx + 1).to_string().yellow()
            );
            io::stdout().flush()?;
        }

        // Remove the whole target directory
        print!(
            "\r[{}/{}] CLEANING TARGET OBJS...{}/{}\x1B[0K",
            step,
            nsteps,
            "0".green(),
            num_entries.to_string().yellow()
        );
        io::stdout().flush()?;
        for (idx, entry) in walkdir::WalkDir::new("target")
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
    print!("[{}/{}] LISTING UNVERSIONEDS...", step, nsteps);
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
        anyhow::bail!("Failed to run `svn status src`");
    }
    let file_pattern =
        regex::Regex::new(r#"\?\s+(\S+)\n"#).with_context(|| "Error regex pattern")?;
    let output_str = String::from_utf8(output.stdout)
        .context(anyhow::anyhow!("Error converting output to `String` type"))?;
    let mut filelist = Vec::new();
    for (_, [file]) in file_pattern.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
    }
    print!(
        "\r[{}/{}] LISTING UNVERSIONEDS...{}\x1B[0K",
        step,
        nsteps,
        filelist.len().to_string().yellow()
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
        let entry = path::Path::new(item);
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
    let ui_dir = path::Path::new(&branch); // UI directory name is the same as the branch name
    if ui_dir.is_dir() {
        step += 1;

        print!("[{}/{}] LISTING UI OBJS...{}", step, nsteps, "0".yellow());
        io::stdout().flush()?;

        let mut num_entries = 0;
        for (idx, _) in walkdir::WalkDir::new(&ui_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            print!(
                "\r[{}/{}] LISTING UI OBJS...{}\x1B[0K",
                step,
                nsteps,
                (idx + 1).to_string().yellow()
            );
            io::stdout().flush()?;
        }

        // Cleaning UI files
        print!(
            "\r[{}/{}] CLEANING UI OBJS...{}/{}\x1B[0K",
            step,
            nsteps,
            "0".green(),
            num_entries.to_string().yellow()
        );
        io::stdout().flush()?;
        for (idx, entry) in walkdir::WalkDir::new(&ui_dir)
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
