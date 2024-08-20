use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::{self, bail, Context};
use crossterm::style::Stylize;
use regex::Regex;
use walkdir::WalkDir;

use crate::utils::SvnInfo;

pub fn clean_build() -> anyhow::Result<()> {
    let svninfo = SvnInfo::new()?;
    let proj_root = Path::new(
        svninfo
            .working_copy_root_path()
            .context("Error fetching project root")?,
    );

    // Must run under the project root
    if env::current_dir()?.as_path() != proj_root {
        anyhow::bail!(
            r#"Location error! Please run command under the project root, i.e. "{}"."#,
            proj_root.to_string_lossy()
        );
    }

    let branch = svninfo
        .branch_name()
        .context("Error fetching branch name")?;

    let nsteps: usize = if Path::new(&branch).is_dir() { 3 } else { 2 };
    let mut step: usize = 1;
    let mut stdout = io::stdout();

    // Cleaning the objects generated in building process
    print!("[{}/{}] LISTING TARGET OBJS...", step, nsteps);
    stdout.flush()?;
    let target_dir = Path::new("target");
    if target_dir.is_dir() {
        let mut num_entries = 0;
        for (idx, _) in WalkDir::new(target_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            print!(
                "\x1B[2K\r[{}/{}] LISTING TARGET OBJS...{}",
                step,
                nsteps,
                (idx + 1).to_string().dark_yellow()
            );
            stdout.flush()?;
        }

        // Remove the whole target directory
        print!(
            "\x1B[2K\r[{}/{}] CLEANING TARGET OBJS...{}/{}",
            step,
            nsteps,
            "0".dark_green(),
            num_entries.to_string().dark_yellow()
        );
        stdout.flush()?;
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
                "\x1B[2K\r[{}/{}] CLEANING TARGET OBJS...{}/{}",
                step,
                nsteps,
                (idx + 1).to_string().dark_green(),
                num_entries.to_string().dark_yellow()
            );
            stdout.flush()?;
        }
    }
    println!(
        "\x1B[2K\r[{}/{}] CLEANING TARGET OBJS...{}",
        step,
        nsteps,
        "DONE".dark_green()
    );

    // Clean unversioned entries
    step += 1;
    print!("[{}/{}] LISTING UNVERSIONEDS...", step, nsteps);
    stdout.flush()?;
    let output = Command::new("svn")
        .args(["status", "src", "bin", "lib"])
        .output()
        .context("Command `svn status src` failed")?;

    if !output.status.success() {
        bail!("Command `svn status src` failed");
    }
    let pattern_file = Regex::new(r#"(?m)^\?[[:blank:]]+(\S+)[[:space:]]*$"#).context("Error creating regex pattern")?;
    let output_str = String::from_utf8(output.stdout)
        .context(anyhow::anyhow!("Error converting to `String` type"))?;
    let mut filelist = Vec::new();
    for (_, [file]) in pattern_file.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
    }

    print!(
        "\x1B[2K\r[{}/{}] CLEANING UNVERSIONEDS...{}/{}",
        step,
        nsteps,
        "0".dark_green(),
        filelist.len().to_string().dark_yellow()
    );
    stdout.flush()?;
    for (idx, item) in filelist.iter().enumerate() {
        let entry = Path::new(item);
        if entry.is_file() || entry.is_symlink() {
            fs::remove_file(item)?;
        } else if entry.is_dir() {
            fs::remove_dir_all(entry)?;
        }
        print!(
            "\x1B[2K\r[{}/{}] CLEANING UNVERSIONEDS...{}/{}",
            step,
            nsteps,
            (idx + 1).to_string().dark_green(),
            filelist.len().to_string().dark_yellow()
        );
        stdout.flush()?;
    }
    println!(
        "\x1B[2K\r[{}/{}] CLEANING UNVERSIONEDS...{}",
        step,
        nsteps,
        "DONE".dark_green()
    );

    // Clean UI files
    let ui_dir = Path::new(&branch); // UI directory name is the same as the branch name
    if ui_dir.is_dir() {
        step += 1;

        print!("[{}/{}] LISTING UI OBJS...", step, nsteps);
        stdout.flush()?;

        let mut num_entries = 0;
        for (idx, _) in WalkDir::new(ui_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            print!(
                "\x1B[2K\r[{}/{}] LISTING UI OBJS...{}",
                step,
                nsteps,
                (idx + 1).to_string().dark_yellow()
            );
            stdout.flush()?;
        }

        // Cleaning UI files
        print!(
            "\x1B[2K\r[{}/{}] CLEANING UI OBJS...{}/{}",
            step,
            nsteps,
            "0".dark_green(),
            num_entries.to_string().dark_yellow()
        );
        stdout.flush()?;
        for (idx, entry) in WalkDir::new(ui_dir)
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
                "\x1B[2K\r[{}/{}] CLEANING UI OBJS...{}/{}",
                step,
                nsteps,
                (idx + 1).to_string().dark_green(),
                num_entries.to_string().dark_yellow()
            );
            stdout.flush()?;
        }

        println!(
            "\x1B[2K\r[{}/{}] CLEANING UI OBJS...{}",
            step,
            nsteps,
            "DONE".dark_green()
        );
    }

    Ok(())
}
