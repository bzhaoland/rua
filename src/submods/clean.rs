use std::io::{self, Write};
use std::path;
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context};

use crate::utils::SvnInfo;

const COLOR_ANSI_YLW: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
const COLOR_ANSI_GRN: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));

pub fn clean_build() -> anyhow::Result<()> {
    let svninfo = SvnInfo::new()?;
    let proj_root = svninfo.working_copy_root_path();

    // Must run under the project root
    if env::current_dir()?.as_path() != proj_root {
        anyhow::bail!(
            r#"Wrong location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    let nsteps: usize = if path::Path::new(svninfo.branch_name()).is_dir() {
        3
    } else {
        2
    };
    let mut step: usize = 1;
    let mut stderr = io::stderr();

    // Cleaning the objects generated in building process
    let mut stderr_lock = stderr.lock();
    write!(stderr_lock, "[{}/{}] LISTING TARGET OBJS...", step, nsteps)?;
    stderr_lock.flush()?;

    let target_dir = path::Path::new("target");
    if target_dir.is_dir() {
        let num_entries = walkdir::WalkDir::new(target_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
            .map(|(i, _)| -> anyhow::Result<()> {
                write!(
                    stderr_lock,
                    "\r[{}/{}] LISTING TARGET OBJS...{}{}{:#}\x1B[0K",
                    step,
                    nsteps,
                    COLOR_ANSI_YLW,
                    i + 1,
                    COLOR_ANSI_YLW
                )?;
                stderr_lock.flush()?;
                Ok(())
            })
            .count();
        let num_entries_suffix_str =
            format!("{}{}{:#}", num_entries, COLOR_ANSI_YLW, COLOR_ANSI_YLW);

        // Remove the whole target directory
        walkdir::WalkDir::new("target")
            .contents_first(true)
            .into_iter()
            .enumerate()
            .map(|(i, v)| -> anyhow::Result<()> {
                write!(
                    stderr_lock,
                    "\r[{}/{}] CLEANING TARGET OBJS...{}{}{:#}/{}{}{:#}\x1B[0K",
                    step,
                    nsteps,
                    COLOR_ANSI_GRN,
                    i,
                    COLOR_ANSI_GRN,
                    COLOR_ANSI_YLW,
                    num_entries_suffix_str,
                    COLOR_ANSI_YLW,
                )?;
                stderr_lock.flush()?;
                let entry = v?;
                let path_ = entry.path();
                if path_.is_file() || path_.is_symlink() {
                    fs::remove_file(path_)
                        .context(format!("Error removing file {}", path_.display()))?;
                } else if path_.is_dir() {
                    fs::remove_dir_all(path_)
                        .context(format!("Error removing directory {}", path_.display()))?;
                }
                Ok(())
            })
            .count();
    }
    writeln!(
        stderr_lock,
        "\r[{}/{}] CLEANING TARGET OBJS...{}DONE{:#}\x1B[0K",
        step, nsteps, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    )?;

    // Clean unversioned entries
    step += 1;
    write!(stderr_lock, "[{}/{}] LISTING UNVERSIONEDS...", step, nsteps)?;
    stderr_lock.flush()?;
    let output = Command::new("svn")
        .args(["status", "src", "bin", "lib"])
        .output()
        .context("Command `svn status src` failed")?;

    if !output.status.success() {
        bail!("Error invoking `svn status src bin lib`");
    }

    let pattern_file = regex::Regex::new(r#"(?m)^\?[[:blank:]]+(\S+)[[:space:]]*$"#)
        .context("Error creating regex pattern")?;
    let output_str = String::from_utf8(output.stdout)
        .context(anyhow::anyhow!("Error converting to `String` type"))?;
    let mut filelist = Vec::new();
    for (_, [file]) in pattern_file.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
    }

    filelist
        .iter()
        .enumerate()
        .map(|(idx, item)| -> anyhow::Result<()> {
            write!(
                stderr_lock,
                "\r[{}/{}] CLEANING UNVERSIONEDS...{}{}{:#}/{}{}{:#}\x1B[0K",
                step,
                nsteps,
                COLOR_ANSI_GRN,
                idx,
                COLOR_ANSI_GRN,
                COLOR_ANSI_YLW,
                filelist.len(),
                COLOR_ANSI_YLW
            )?;
            stderr.flush()?;
            let path_ = path::Path::new(item);
            if path_.is_file() || path_.is_symlink() {
                fs::remove_file(path_)
                    .context(format!("Error removing file {}", path_.display()))?;
            } else if path_.is_dir() {
                fs::remove_dir_all(path_)
                    .context(format!("Error removing directory {}", path_.display()))?;
            }
            Ok(())
        })
        .count();
    writeln!(
        stderr_lock,
        "\r[{}/{}] CLEANING UNVERSIONEDS...{}DONE{:#}\x1B[0K",
        step, nsteps, COLOR_ANSI_GRN, COLOR_ANSI_GRN,
    )?;

    // Clean UI files
    let ui_dir = path::Path::new(svninfo.branch_name()); // UI directory name is the same as the branch name
    if ui_dir.is_dir() {
        step += 1;

        write!(stderr_lock, "[{}/{}] LISTING UI OBJS...", step, nsteps)?;
        stderr.flush()?;

        let mut num_entries = 0;
        for (idx, _) in walkdir::WalkDir::new(ui_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            num_entries += 1;
            write!(
                stderr_lock,
                "\r[{}/{}] LISTING UI OBJS...{}{}{}\x1B[0K",
                step,
                nsteps,
                COLOR_ANSI_YLW,
                idx + 1,
                COLOR_ANSI_YLW
            )?;
            stderr.flush()?;
        }

        // Cleaning UI files
        stderr.flush()?;
        for (idx, entry) in walkdir::WalkDir::new(ui_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            write!(
                stderr_lock,
                "\r[{}/{}] CLEANING UI OBJS...{}{}{:#}/{}{}{:#}\x1B[0K",
                step,
                nsteps,
                COLOR_ANSI_GRN,
                idx,
                COLOR_ANSI_GRN,
                COLOR_ANSI_YLW,
                num_entries,
                COLOR_ANSI_YLW
            )?;
            stderr.flush()?;
            let entry = entry?;
            let path_ = entry.path();
            if path_.is_file() || path_.is_symlink() {
                fs::remove_file(path_)
                    .context(format!("Error removing file {}", path_.display()))?;
            } else if path_.is_dir() {
                fs::remove_dir_all(path_)
                    .context(format!("Error removing directory {}", path_.display()))?;
            }
        }

        writeln!(
            stderr_lock,
            "\r[{}/{}] CLEANING UI OBJS...{}DONE{:#}\x1B[0K",
            step, nsteps, COLOR_ANSI_GRN, COLOR_ANSI_GRN
        )?;
    }

    Ok(())
}
