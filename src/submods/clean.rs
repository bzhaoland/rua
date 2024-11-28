use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context};
use std::time::Instant;

use crate::utils::SvnInfo;

const COLOR_ANSI_YLW: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));
const COLOR_ANSI_GRN: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));

pub fn clean_build(
    dirs: Option<Vec<OsString>>,
    ignores: Option<Vec<OsString>>,
    debug: bool,
) -> anyhow::Result<()> {
    let svninfo = SvnInfo::new()?;
    let proj_root = svninfo.working_copy_root_path();
    let current_dir = env::current_dir()?;

    // Must run under the project root
    if current_dir.as_path() != proj_root {
        anyhow::bail!(
            r#"Wrong location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    let ignores: Vec<PathBuf> = ignores
        .as_deref()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|x| {
            let entry = Path::new(x);
            fs::canonicalize(entry).ok()
        })
        .collect();

    let nsteps: usize = if Path::new(svninfo.branch_name()).is_dir() {
        3
    } else {
        2
    };
    let mut step: usize = 1;

    // Cleaning the objects generated in building process
    eprint!("[{}/{}] CLEANING TARGET OBJS...", step, nsteps);
    io::stderr().flush()?;

    let mut prev_time = Instant::now();
    let target_dir = Path::new("target");
    if target_dir.is_dir() {
        for (i, x) in walkdir::WalkDir::new(target_dir)
            .contents_first(true)
            .into_iter()
            .filter(|x| {
                if x.is_err() {
                    return false;
                }
                let x = x.as_ref().unwrap();
                let entry = x.path();
                !ignores.iter().any(|x| x.as_path() == entry)
            })
            .enumerate()
        {
            let path_ = x.as_ref().unwrap().path();

            if debug {
                eprintln!("REMOVING {}", path_.display());
            }

            let curr_time = Instant::now();
            let delta = curr_time - prev_time;
            prev_time = curr_time;
            if delta.as_millis() >= 200 {
                let mut file_indicator = path_.to_string_lossy().to_string();
                file_indicator.truncate(32);
                eprint!(
                    "\r[{}/{}] CLEANING TARGET OBJS...{}{}{:#}: {}{}...{:#}...\x1B[0K",
                    step,
                    nsteps,
                    COLOR_ANSI_YLW,
                    i + 1,
                    COLOR_ANSI_YLW,
                    COLOR_ANSI_GRN,
                    file_indicator,
                    COLOR_ANSI_GRN
                );
                io::stderr().flush().unwrap();
            }

            if path_.is_file() || path_.is_symlink() {
                fs::remove_file(path_)
                    .context(format!("Error removing file {}", path_.display()))
                    .unwrap();
            } else if path_.is_dir() {
                fs::remove_dir_all(path_)
                    .context(format!("Error removing directory {}", path_.display()))
                    .unwrap();
            }
        }
    }
    eprintln!(
        "\r[{}/{}] CLEANING TARGET OBJS...{}DONE{:#}\x1B[0K",
        step, nsteps, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    );

    // Clean unversioned entries
    step += 1;
    eprint!("[{}/{}] LISTING UNVERSIONEDS...", step, nsteps);
    io::stderr().flush()?;

    let dirs: Vec<OsString> = dirs.unwrap_or_default();
    let output = Command::new("svn")
        .arg("status")
        .args(dirs.iter())
        .output()
        .context(format!("Command `svn status {:?}` failed", dirs))?;

    if !output.status.success() {
        bail!(
            "Error invoking `svn status {:?}`",
            dirs.iter()
                .map(|x| x.to_string_lossy().to_string().to_owned())
                .collect::<Vec<String>>()
                .join(" ")
        );
    }

    let pattern_file = regex::Regex::new(r#"(?m)^\?[[:blank:]]+(.+?)[[:space:]]*$"#)
        .context("Error creating regex pattern")?;
    let output_str = String::from_utf8(output.stdout)
        .context(anyhow::anyhow!("Error converting to `String` type"))?;
    let mut filelist = Vec::new();
    for (_, [file]) in pattern_file.captures_iter(&output_str).map(|c| c.extract()) {
        filelist.push(file.to_string());
    }

    let mut prev_time = Instant::now();
    for (idx, item) in filelist.iter().enumerate() {
        let curr_time = Instant::now();
        let delta = curr_time - prev_time;
        prev_time = curr_time;

        if delta.as_millis() >= 200 {
            eprint!(
                "\r[{}/{}] CLEANING UNVERSIONEDS...{}{}{:#}/{}{}{:#}: {}{}{:#}\x1B[0K",
                step,
                nsteps,
                COLOR_ANSI_GRN,
                idx,
                COLOR_ANSI_GRN,
                COLOR_ANSI_YLW,
                filelist.len(),
                COLOR_ANSI_YLW,
                COLOR_ANSI_GRN,
                item,
                COLOR_ANSI_GRN
            );
            io::stderr().flush()?;
        }

        let path_ = Path::new(item);

        if debug {
            eprintln!("REMOVING {}", path_.display());
        }

        if path_.is_file() || path_.is_symlink() {
            fs::remove_file(path_).context(format!("Error removing file {}", path_.display()))?;
        } else if path_.is_dir() {
            fs::remove_dir_all(path_)
                .context(format!("Error removing directory {}", path_.display()))?;
        }
    }
    eprintln!(
        "\r[{}/{}] CLEANING UNVERSIONEDS...{}DONE{:#}\x1B[0K",
        step, nsteps, COLOR_ANSI_GRN, COLOR_ANSI_GRN,
    );

    // Clean UI files
    let ui_dir = Path::new(svninfo.branch_name()); // UI directory name is the same as the branch name
    if ui_dir.is_dir() {
        step += 1;

        eprint!("[{}/{}] CLEANING WEBUI OBJS...", step, nsteps);
        io::stderr().flush()?;

        let mut prev_time = Instant::now();
        for (idx, x) in walkdir::WalkDir::new(ui_dir)
            .contents_first(true)
            .into_iter()
            .enumerate()
        {
            let entry = x?;
            let path_ = entry.path();

            if debug {
                eprintln!("REMOVING {}", path_.display());
            }

            let curr_time = Instant::now();
            let delta = curr_time - prev_time;
            prev_time = curr_time;
            if delta.as_millis() >= 200 {
                eprint!(
                    "\r[{}/{}] CLEANING WEBUI OBJS...{}{}{:#}: {}{}{:#}\x1B[0K",
                    step,
                    nsteps,
                    COLOR_ANSI_YLW,
                    idx + 1,
                    COLOR_ANSI_YLW,
                    COLOR_ANSI_GRN,
                    path_.display(),
                    COLOR_ANSI_GRN
                );
                io::stderr().flush()?;
            }

            if path_.is_file() || path_.is_symlink() {
                fs::remove_file(path_)
                    .context(format!("Error removing file {}", path_.display()))?;
            } else if path_.is_dir() {
                fs::remove_dir_all(path_)
                    .context(format!("Error removing directory {}", path_.display()))?;
            }
        }

        eprintln!(
            "\r[{}/{}] CLEANING UI OBJS...{}DONE{:#}\x1B[0K",
            step, nsteps, COLOR_ANSI_GRN, COLOR_ANSI_GRN
        );
    }

    Ok(())
}
