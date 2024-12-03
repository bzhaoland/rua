use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use anstyle::{AnsiColor, Color, Style};
use anyhow::{anyhow, bail, Context};
use std::time::Instant;

use crate::utils::SvnInfo;

const COLOR_ANSI_YLW: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
const COLOR_ANSI_GRN: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));

fn trucate_string(s: &str, l: usize) -> String {
    s.chars().skip(s.chars().count() - l).collect()
}

pub fn clean_build(
    dirs: Option<Vec<OsString>>,
    ignores: Option<&Vec<OsString>>,
    debug: bool,
) -> anyhow::Result<()> {
    let svn_info = SvnInfo::new()?;
    let proj_dir = svn_info.working_copy_root_path();
    let curr_dir = env::current_dir()?;

    // Must run under the project root
    if curr_dir.as_path() != proj_dir {
        bail!(
            r#"Location error! Please run this command under the project root, i.e. "{}"."#,
            proj_dir.display()
        );
    }

    let ignores = ignores
        .map(|x| {
            x.iter()
                .filter_map(|p| fs::canonicalize(p).ok())
                .collect::<Vec<PathBuf>>()
        })
        .unwrap_or_default();

    const REFRESH_INTERVAL: u128 = 200; // In milliseconds
    const DISPLAY_PATH_LEN: usize = 32;

    let num_steps = 3;
    let mut step: usize = 0;

    // Cleaning the objects generated in building process
    step += 1;
    eprint!("[{}/{}] CLEANING TARGET OBJS...", step, num_steps);
    io::stderr().flush()?;

    let target_dir = fs::canonicalize("target");
    if target_dir.is_ok() {
        let mut prev_time = Instant::now();
        for (i, x) in walkdir::WalkDir::new(target_dir.unwrap())
            .contents_first(true)
            .into_iter()
            .filter(|x| {
                if x.is_err() {
                    return false;
                }
                let x = x.as_ref().unwrap();
                let entry = x.path();

                ignores.iter().all(|x| x.as_path() != entry)
            })
            .enumerate()
        {
            let entry = x?;
            let path_ = entry.path();

            if debug {
                eprintln!("REMOVING {}", path_.display());
            } else {
                let curr_time = Instant::now();
                if debug || (curr_time - prev_time).as_millis() >= REFRESH_INTERVAL {
                    let file_indicator = trucate_string(&path_.to_string_lossy(), DISPLAY_PATH_LEN);
                    eprint!(
                        "\r[{}/{}] CLEANING TARGET OBJS...{}{}{:#}: {}...{}{:#}\x1B[0K",
                        step,
                        num_steps,
                        COLOR_ANSI_YLW,
                        i + 1,
                        COLOR_ANSI_YLW,
                        COLOR_ANSI_GRN,
                        file_indicator,
                        COLOR_ANSI_GRN
                    );
                    io::stderr().flush()?;
                    prev_time = curr_time;
                }
            }

            if path_.is_file() || path_.is_symlink() {
                fs::remove_file(path_)
                    .context(format!("Error removing file {}", path_.display()))?;
            } else if path_.is_dir() {
                fs::remove_dir_all(path_)
                    .context(format!("Error removing directory {}", path_.display()))?;
            }
        }
    }

    eprintln!(
        "\r[{}/{}] CLEANING TARGET OBJS...{}DONE{:#}\x1B[0K",
        step, num_steps, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    );

    // Clean unversioned entries
    step += 1;
    eprint!("[{}/{}] LISTING UNVERSIONEDS...", step, num_steps);
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

    let pattern_for_unversioneds =
        regex::Regex::new(r#"(?m)^\?[[:blank:]]+([^[:blank:]].*?)[[:space:]]*$"#)
            .context("Error creating regex pattern")?; // Pattern for out-of-control files
    let output_str =
        String::from_utf8(output.stdout).context(anyhow!("Error converting to `String` type"))?;
    let unversioned_files = pattern_for_unversioneds
        .captures_iter(&output_str)
        .filter_map(|c| fs::canonicalize((c.extract::<1>().1)[0]).ok())
        .filter(|x| ignores.iter().all(|i| x != i))
        .collect::<Vec<PathBuf>>();

    let mut prev_time = Instant::now();
    for (idx, item) in unversioned_files.iter().enumerate() {
        if debug {
            eprintln!("REMOVING {}", item.display());
        } else {
            let curr_time = Instant::now();
            if (curr_time - prev_time).as_millis() >= REFRESH_INTERVAL {
                eprint!(
                    "\r[{}/{}] CLEANING UNVERSIONEDS...{}{}{:#}/{}{}{:#}: {}...{}{:#}\x1B[0K",
                    step,
                    num_steps,
                    COLOR_ANSI_GRN,
                    idx,
                    COLOR_ANSI_GRN,
                    COLOR_ANSI_YLW,
                    unversioned_files.len(),
                    COLOR_ANSI_YLW,
                    COLOR_ANSI_GRN,
                    trucate_string(&item.to_string_lossy(), DISPLAY_PATH_LEN),
                    COLOR_ANSI_GRN
                );
                io::stderr().flush()?;
                prev_time = curr_time;
            }
        }

        if item.is_file() || item.is_symlink() {
            fs::remove_file(item).context(format!("Error removing file {}", item.display()))?;
        } else if item.is_dir() {
            fs::remove_dir_all(item)
                .context(format!("Error removing directory {}", item.display()))?;
        }
    }

    eprintln!(
        "\r[{}/{}] CLEANING UNVERSIONEDS...{}DONE{:#}\x1B[0K",
        step, num_steps, COLOR_ANSI_GRN, COLOR_ANSI_GRN,
    );

    // Clean UI files
    step += 1;
    eprint!("[{}/{}] CLEANING WEBUI OBJS...", step, num_steps);
    io::stderr().flush()?;

    let webui_dir = fs::canonicalize(svn_info.branch_name()); // UI directory name is the same as the branch name
    if webui_dir.is_ok() {
        let mut prev_time = Instant::now();
        for (idx, x) in walkdir::WalkDir::new(webui_dir.unwrap())
            .contents_first(true)
            .into_iter()
            .filter(|x| {
                if x.is_err() {
                    return false;
                }

                let x = x.as_ref().unwrap();
                let entry = x.path();

                ignores.iter().all(|x| x.as_path() != entry)
            })
            .enumerate()
        {
            let entry = x?;
            let path_ = entry.path();

            if debug {
                eprintln!("REMOVING {}", path_.display());
            } else {
                let curr_time = Instant::now();
                if (curr_time - prev_time).as_millis() >= REFRESH_INTERVAL {
                    eprint!(
                        "\r[{}/{}] CLEANING WEBUI OBJS...{}{}{:#}: {}...{}{:#}\x1B[0K",
                        step,
                        num_steps,
                        COLOR_ANSI_YLW,
                        idx + 1,
                        COLOR_ANSI_YLW,
                        COLOR_ANSI_GRN,
                        trucate_string(&path_.to_string_lossy(), DISPLAY_PATH_LEN),
                        COLOR_ANSI_GRN
                    );
                    io::stderr().flush()?;
                    prev_time = curr_time;
                }
            }

            if path_.is_file() || path_.is_symlink() {
                fs::remove_file(path_)
                    .context(format!("Error removing file {}", path_.display()))?;
            } else if path_.is_dir() {
                fs::remove_dir_all(path_)
                    .context(format!("Error removing directory {}", path_.display()))?;
            }
        }
    }

    eprintln!(
        "\r[{}/{}] CLEANING UI OBJS...{}DONE{:#}\x1B[0K",
        step, num_steps, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    );

    Ok(())
}
