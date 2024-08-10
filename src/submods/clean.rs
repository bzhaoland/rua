use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{self, bail, Context};
use crossterm::style::Stylize;
use regex::Regex;
use walkdir::WalkDir;

use crate::utils;

pub fn clean_build() -> anyhow::Result<()> {
    // Must run under the project root
    if !utils::is_at_proj_root()? {
        anyhow::bail!("Location error! Please run command under the project root.");
    }

    let branch = utils::get_svn_branch()?.unwrap();

    let nsteps: usize = if Path::new(&branch).is_dir() { 3 } else { 2 };
    let mut step: usize = 1;

    // Cleaning the objects generated in building process
    println!("[{}/{}] LISTING TARGET OBJS...", step, nsteps);
    let target_dir = Path::new("target");
    if target_dir.is_dir() {
        let mut num_entries = 0;
        for (idx, _) in WalkDir::new(target_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            println!(
                "\x1B[1A\x1B[2K\r[{}/{}] LISTING TARGET OBJS...{}",
                step,
                nsteps,
                (idx + 1).to_string().dark_yellow()
            );
        }

        // Remove the whole target directory
        println!(
            r#"\x1B[1A\x1B[2K\r[{}/{}] CLEANING TARGET OBJS...{}/{}"#,
            step,
            nsteps,
            "0".dark_green(),
            num_entries.to_string().dark_yellow()
        );
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
            println!(
                r#"\x1B[1A\x1B[0K\r[{}/{}] CLEANING TARGET OBJS...{}/{}"#,
                step,
                nsteps,
                (idx + 1).to_string().dark_green(),
                num_entries.to_string().dark_yellow()
            );
        }
    }
    println!(
        r#"\x1B[1A\x1B[2K\r[{}/{}] CLEANING TARGET OBJS...{}"#,
        step,
        nsteps,
        "DONE".dark_green()
    );

    // Clean unversioned entries
    step += 1;
    println!("[{}/{}] LISTING UNVERSIONEDS...", step, nsteps);
    let output = Command::new("svn")
        .args(["status", "src", "bin", "lib"])
        .output()
        .context("Command `svn status src` failed")?;

    if !output.status.success() {
        bail!("Command `svn status src` failed");
    }
    let file_pattern = Regex::new(r#"\?\s+(\S+)\n"#).context("Error regex pattern")?;
    let output_str = String::from_utf8(output.stdout)
        .context(anyhow::anyhow!("Error converting to `String` type"))?;
    let mut filelist = Vec::new();
    for (_, [file]) in file_pattern.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
    }
    println!(
        r#"\x1B[1A\x1B[2K\r[{}/{}] LISTING UNVERSIONEDS...{}"#,
        step,
        nsteps,
        filelist.len().to_string().dark_yellow()
    );

    println!(
        "\x1B[1A\x1B[2K\r[{}/{}] CLEANING UNVERSIONEDS...{}/{}",
        step,
        nsteps,
        "0".dark_green(),
        filelist.len().to_string().dark_yellow()
    );
    for (idx, item) in filelist.iter().enumerate() {
        let entry = Path::new(item);
        if entry.is_file() || entry.is_symlink() {
            fs::remove_file(item)?;
        } else if entry.is_dir() {
            fs::remove_dir_all(entry)?;
        }
        println!(
            r#"\x1B[1A\x1B[2K\r[{}/{}] CLEANING UNVERSIONEDS...{}/{}"#,
            step,
            nsteps,
            (idx + 1).to_string().dark_green(),
            filelist.len().to_string().dark_yellow()
        );
    }
    println!(
        r#"\x1B[1A\x1B[2K\r[{}/{}] CLEANING UNVERSIONEDS...{}"#,
        step,
        nsteps,
        "DONE".dark_green()
    );

    // Clean UI files
    let ui_dir = Path::new(&branch); // UI directory name is the same as the branch name
    if ui_dir.is_dir() {
        step += 1;

        println!(r#"[{}/{}] LISTING UI OBJS..."#, step, nsteps);

        let mut num_entries = 0;
        for (idx, _) in WalkDir::new(ui_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            println!(
                r#"\x1B[1A\x1B[2K\r[{}/{}] LISTING UI OBJS...{}"#,
                step,
                nsteps,
                (idx + 1).to_string().dark_yellow()
            );
        }

        // Cleaning UI files
        println!(
            r#"\x1B[1A\x1B[2K\r[{}/{}] CLEANING UI OBJS...{}/{}"#,
            step,
            nsteps,
            "0".dark_green(),
            num_entries.to_string().dark_yellow()
        );
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
            println!(
                r#"\x1B[1A\x1B[2K\r[{}/{}] CLEANING UI OBJS...{}/{}"#,
                step,
                nsteps,
                (idx + 1).to_string().dark_green(),
                num_entries.to_string().dark_yellow()
            );
        }

        println!(
            "\x1B[1A\x1B[2K\r[{}/{}] CLEANING UI OBJS...{}",
            step,
            nsteps,
            "DONE".dark_green()
        );
    }

    Ok(())
}
