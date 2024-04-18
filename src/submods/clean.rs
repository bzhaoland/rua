use std::io::{self, Write};
use std::process;

use anyhow::{Error, Result as AResult};
use colored::Colorize;
use regex;

pub fn clean_build() -> AResult<()> {
    let num_steps = 7usize;
    let mut curr_step;

    // Clean the target directory
    curr_step = 1usize;
    print!("[{}/{}] REMOVING TARGET DIRECTORY...", curr_step, num_steps);
    println!("[{}/{}] REMOVING TARGET DIRECTORY...{}\x1B[0K", curr_step, num_steps, "OK".green());

    // Clean the unversioned entries
    curr_step = 2usize;
    print!("[{}/{}] FINDING UNVERSIONED ENTRIES...", curr_step, num_steps);
    io::stdout().flush()?;
    let output = process::Command::new("svn").args(["status", "src"]).output().map_err(|e| { println!("\r[{}/{}] FINDING UNVERSIONED ENTRIES...{}\x1B[0K", curr_step, num_steps, "FAILED".red()); Error::msg(format!("Failed to execute `svn status src`: {}", e.to_string())) })?;
    if output.status.success() != true {
        println!("\r[{}/{}] FINDING UNVERSIONED ENTRIES...{}\x1B[0K", curr_step, num_steps, "FAILED".red());
        return Err(Error::msg("Error: Failed to execute `svn status src`"));
    }
    let file_pattern = regex::Regex::new(r#"\?\s+(\S+)\n"#)?;
    let output_str = String::from_utf8(output.stdout)?;
    let mut filelist = Vec::new();
    for (_, [file]) in file_pattern.captures_iter(&output_str).map(|c| c.extract()) {
       filelist.push(file.to_string());
    }
    println!("\r[{}/{}] FINDING UNVERSIONED ENTRIES...{}\x1B[0K", curr_step, num_steps, "OK".green());

    Ok(())
}
