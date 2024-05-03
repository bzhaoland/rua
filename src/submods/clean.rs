use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use anyhow::Context;
use anyhow::Error as AnyError;
use anyhow::Result as AnyResult;
use console::{Style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

pub fn clean_build() -> AnyResult<()> {
    let num_steps: usize = 2;
    let mut curr_step: usize;

    // Term control
    let color_grn = Style::new().green();
    let color_red = Style::new().red();
    let term_stdout = Term::stdout();

    // Clean the target directory
    curr_step = 1;
    print!("[{}/{}] REMOVING TARGET DIRECTORY...", curr_step, num_steps);
    io::stdout().flush()?;
    fs::remove_dir_all("target").map_err(|e| {
        println!("{}", color_red.apply_to("FAILED"));
        e
    })?;
    term_stdout.clear_line()?;
    println!("[{}/{}] REMOVED TARGET DIRECTORY", curr_step, num_steps);

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
            println!("{}", color_red.apply_to("FAILED"));
            "Failed to exec `svn status src`"
        })?;

    if !output.status.success() {
        println!("{}", color_red.apply_to("FAILED"));
        return Err(AnyError::msg("Error: Failed to execute `svn status src`"));
    }
    let file_pattern = Regex::new(r#"\?\s+(\S+)\n"#).with_context(|| "Error regex pattern")?;
    let output_str = String::from_utf8(output.stdout)
        .with_context(|| "Failed to convert output to String type")?;
    let mut filelist = Vec::new();
    for (_, [file]) in file_pattern.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
        term_stdout.clear_line()?;
        print!(
            "[{}/{}] FINDING UNVERSIONED ENTRIES...{}",
            curr_step,
            num_steps,
            color_grn.apply_to(filelist.len().to_string())
        );
    }
    term_stdout.clear_line()?;
    print!(
        "[{}/{}] FOUND {} UNVERSIONED ENTRIES",
        curr_step,
        num_steps,
        color_grn.apply_to(filelist.len().to_string())
    );

    term_stdout.clear_line()?;
    let pb = ProgressBar::new(filelist.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} {prefix} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    pb.set_prefix(format!("{}/{} CLEANING", curr_step, num_steps));
    for item in filelist.iter() {
        let entry = PathBuf::from(item);
        if entry.is_file() || entry.is_symlink() {
            fs::remove_file(item).map_err(|e| {
                println!();
                e
            })?;
        } else if entry.is_dir() {
            fs::remove_dir_all(entry).map_err(|e| {
                println!();
                e
            })?;
        }
        pb.inc(1);
    }
    pb.finish_and_clear();
    term_stdout.clear_line()?;
    println!(
        "[{}/{}] CLEANED {} UNVERSIONED ENTRIES",
        curr_step,
        num_steps,
        color_grn.apply_to(filelist.len().to_string())
    );

    Ok(())
}
