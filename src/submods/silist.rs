use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Error, Result};
use crossterm::style::Stylize;
use walkdir::WalkDir;

/// Generate filelist for SourceInsight editor.
///
/// This function just searches and collects all c/c++ source and header files over the project.
///
/// Parameter `prefix` means the repo's root path on WinBuilder.
///
/// Note: The method used by `compdb` command does not suit here for SourceInsight, bacause it is
/// time-consuming and does not collect header files. In order to collect header files, we have to
/// parse all the '#include' directives in all compilation units. This is also a very time-consuming
/// job.
pub fn gen_silist(prefix: &str) -> Result<()> {
    // Term control
    let mut stdout = io::stdout();

    // Generate FILELIST
    print!("GENERATING FILELIST...");
    stdout.flush()?;
    let extensions = [
        "c".to_string(),
        "cc".to_string(),
        "cxx".to_string(),
        "cpp".to_string(),
        "h".to_string(),
        "hh".to_string(),
        "hxx".to_string(),
        "hpp".to_string(),
    ];
    let mut files = Vec::new();
    let curr_dir = env::current_dir()?;
    let repo_root_on_winbuilder = PathBuf::from(prefix);

    // Search over src directory
    print!("GENERATING FILELIST...{} FILES FOUND", "0".yellow());
    stdout.flush()?;
    for entry in WalkDir::new("src") {
        if entry.is_err() {
            continue;
        }

        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            continue;
        }

        let result = entry.path().canonicalize();
        if result.is_err() {
            continue;
        }

        let entry = result.unwrap();
        let result = entry.extension();
        if result.is_none() {
            continue;
        }

        let extension = result.unwrap().to_string_lossy().to_lowercase();
        if !extensions.contains(&extension) {
            continue;
        }

        let entry_relative = entry.strip_prefix(&curr_dir).map_err(|e| {
            println!();
            Error::msg(format!("{}: {}", curr_dir.to_string_lossy(), e))
        })?;

        let entry_on_winbuilder = repo_root_on_winbuilder.join(entry_relative);
        files.push(entry_on_winbuilder.to_string_lossy().to_string());
        print!(
            "\rGENERATING FILELIST...{} FILES FOUND\x1B[K",
            files.len().to_string().yellow(),
        );
        stdout.flush()?;
    }

    // Search over gshare directory
    for entry in WalkDir::new("gshare") {
        if entry.is_err() {
            continue;
        }

        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            continue;
        }

        let result = entry.path().canonicalize();
        if result.is_err() {
            continue;
        }

        let entry = result.unwrap();
        let result = entry.extension();
        if result.is_none() {
            continue;
        }

        let extension = result.unwrap().to_string_lossy().to_lowercase();
        if !extensions.contains(&extension) {
            continue;
        }

        let entry_relative = entry.strip_prefix(&curr_dir).map_err(|e| {
            println!();
            Error::msg(format!("{}: {}", curr_dir.to_string_lossy(), e))
        })?;

        let entry_on_winbuilder = repo_root_on_winbuilder.join(entry_relative);
        files.push(entry_on_winbuilder.to_string_lossy().to_string());
        print!(
            "\rGENERATING FILELIST...{} FILES FOUND\x1B[K",
            files.len().to_string().yellow()
        );
        stdout.flush()?;
    }

    let filelist = files.join("\r\n");
    fs::write("filelist.txt", filelist)?;
    println!("\rGENERATING FILELIST...{}\x1B[K", "OK".green());

    Ok(())
}
