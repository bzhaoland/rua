use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Error, Result};
use colored::Colorize;
use regex::Regex;
use serde_json::{self, json};

#[derive(Debug)]
struct CompRecord {
    dirc: String,
    comm: String,
    file: String,
}

pub fn gen_compdb(product_dir: &str, make_target: &str) -> Result<()> {
    let lastrules_file = "./scripts/last-rules.mk";
    let rules_file = "./scripts/rules.mk";
    let num_steps = 6;
    let mut curr_step;

    // Checking running directory, should run under project root
    curr_step = 1;
    print!("[{}/{}] CHECKING LOCATION...", curr_step, num_steps);
    io::stdout().flush()?;
    let mut location_ok = true;
    let attr = fs::metadata(lastrules_file);
    if attr.is_err() || attr.unwrap().is_file() != true {
        location_ok = false;
    }
    let attr = fs::metadata(rules_file);
    if attr.is_err() || attr.unwrap().is_file() != true {
        location_ok = false;
    }
    if location_ok == false {
        eprintln!(
            "\r[{}/{}] CHECKING LOCATION...{}\x1B[0K",
            curr_step,
            num_steps,
            "FAILED".red()
        );
        return Result::Err(Error::msg("Error: Run this command under project root."));
    }
    println!(
        "\r[{}/{}] CHECKING LOCATION...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Inject hacked make rules
    curr_step = 2;
    print!("[{}/{}] INJECTING MAKEFILES...", curr_step, num_steps);
    io::stdout().flush()?;
    let recipe_pattern_c = Regex::new(r#"(\t\s*\$\(HS_CC\)\s+\$\(CFLAGS_GLOBAL_CP\)\s+\$\(CFLAGS_LOCAL_CP\)\s+-MMD\s+-c(\s+-E)?\s+-o\s+\$@\s+\$<\s*?\n?)"#).unwrap();
    let lastrules_orig = fs::read_to_string(lastrules_file)?;
    let lastrules_hack = recipe_pattern_c.replace_all(&lastrules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(CC) $(CFLAGS_GLOBAL_CP) $(CFLAGS_LOCAL_CP) -MMD -c$2 -o $$@ $$< >>:file:>> $$<\n${1}").to_string();
    fs::write(lastrules_file, lastrules_hack)?;
    let recipe_pattern_cc = Regex::new(r#"(\t\s*\$\(COMPILE_CXX_CP_E\)(\s+-E)?\s*?\n?)"#).unwrap();
    let rules_orig = fs::read_to_string(rules_file)?;
    let rules_hack = recipe_pattern_cc.replace_all(&rules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(COMPILE_CXX_CP)$2 >>:file:>> $$<\n${1}").to_string();
    fs::write(rules_file, rules_hack)?;
    println!(
        "\r[{}/{}] INJECTING MAKEFILES...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Build the target (pseudo)
    curr_step = 3;
    print!("[{}/{}] BUILDING TARGET...", curr_step, num_steps);
    io::stdout().flush()?;
    let output = Command::new("hsdocker7")
        .args([
            "make",
            "-C",
            product_dir,
            make_target,
            "-j16",
            "-inwB",
            "HS_BUILD_COVERITY=0",
            "ISBUILDRELEASE=1",
            "HS_BUILD_UNIWEBUI=0",
            "HS_SHELL_PASSWORD=0",
            "IMG_NAME=RUAIHA",
        ])
        .output().map_err(|e| {
            println!("\r[{}/{}] BUILDING TARGET...{}\x1B[0K", curr_step, num_steps, "FAILED".red()); Error::msg(format!("Failed to execute `hsdocker7 make ...`: {}", &e.to_string()))
        })?;
    let status = output.status;
    if status.success() != true {
        println!(
            "\r[{}/{}] BUILDING TARGET...{}\x1B[0K",
            curr_step,
            num_steps,
            "FAILED".red()
        );
        return Result::Err(Error::msg(format!("Error: Failed to build target: {}", status)));
    }
    println!(
        "\r[{}/{}] BUILDING TARGET...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Restore original makefiles
    curr_step = 4;
    print!("[{}/{} RESTORING MAKERULES...]", curr_step, num_steps);
    io::stdout().flush()?;
    fs::write(lastrules_file, lastrules_orig).map_err(|e| {
        println!("[{}/{} RESTORING MAKERULES...{}\x1B[0K", curr_step, num_steps, "FAILED".red());
        e
    })?;
    fs::write(rules_file, rules_orig).map_err(|e| {
        println!("[{}/{} RESTORING MAKERULES...{}\x1B[0K", curr_step, num_steps, "FAILED".red());
        e
    })?;
    println!(
        "\r[{}/{}] RESTORING MAKERULES...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Parse the build log
    curr_step = 5;
    print!("[{}/{}] PARSING BUILD LOG...", curr_step, num_steps);
    io::stdout().flush()?;
    let output_str = String::from_utf8(output.stdout).map_err(|e| {
        println!("\r[{}/{}] PARSING BUILD LOG...{}\x1B[0K", curr_step, num_steps, "FAILED".red());
        e
    })?;
    let hackrule_pattern = Regex::new(
        r#"##JCDB##\s+>>:directory:>>\s+([^\n]+?)\s+>>:command:>>\s+([^\n]+?)\s+>>:file:>>\s+([^\n]+)\s*\n?"#,
    )?;
    let mut records: Vec<CompRecord> = Vec::new();
    for (_, [dirc, comm, file]) in hackrule_pattern
        .captures_iter(&output_str)
        .map(|c| c.extract())
    {
        let dirc = dirc.to_string();
        let comm = comm.to_string();
        let file = PathBuf::from(&dirc)
            .join(&file)
            .to_string_lossy()
            .to_string();
        records.push(CompRecord { dirc, comm, file });
    }
    println!(
        "\r[{}/{}] PARSING BUILD LOG...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Generate JCDB
    curr_step = 6;
    print!("[{}/{}] GENERATING JCDB...", curr_step, num_steps);
    io::stdout().flush()?;
    let mut jcdb = json!([]);
    for item in records.iter() {
        jcdb.as_array_mut().unwrap().push(json!({
            "directory": item.dirc,
            "command": item.comm,
            "file": item.file,
        }));
    }
    fs::write(
        "compile_commands.json",
        serde_json::to_string_pretty(&jcdb)?,
    )?;
    println!(
        "\r[{}/{}] GENERATING JCDB...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    Ok(())
}
