use std::any::Any;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::string::String;

use anyhow::{Context, Error, Result};
use colored::Colorize;
use regex::Regex;
use serde_json::{self, json};

pub fn gen_compdb(product_dir: &str, make_target: &str) -> Result<()> {
    let lastrules_file = "./scripts/last-rules.mk";
    let rules_file = "./scripts/rules.mk";
    let num_steps = 6;
    let mut curr_step;

    // Checking running directory, should run under project root
    curr_step = 1;
    print!("[{}/{}] CHECKING LOCATION...", curr_step, num_steps);
    io::stdout().flush()?;
    let location_ok = true;
    let attr = fs::metadata(lastrules_file);
    if attr.is_err() || attr.unwrap().is_file() != true {
        location_ok = false;
    }
    let attr = fs::metadata(rules_file);
    if attr.is_err() || attr.unwrap().is_file() != true {
        location_ok = false;
    }
    if location_ok == false {
        println!(
            "\r[{}/{}] CHECKING LOCATION...{}\x1B[0K",
            curr_step,
            num_steps,
            "FAILED".red()
        );
        return Err(Error::msg("Run this command under project root."));
    }
    println!(
        "\r[{}/{}] CHECKING LOCATION...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Back up the original make files
    curr_step = 2;
    print!(
        "[{}/{} BACKING UP ORIGINAL MAKERULES...]",
        curr_step, num_steps
    );
    io::stdout().flush()?;
    let lastrules_file_bak = &format!("{}.bak", lastrules_file);
    let rules_file_bak = &format!("{}.bak", rules_file);
    fs::rename(lastrules_file, lastrules_file_bak)?;
    fs::rename(rules_file, rules_file_bak)?;
    println!(
        "\r[{}/{} BACKING UP ORIGINAL MAKERULES...{}]\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Create hacked make files
    curr_step = 3;
    print!("[{}/{}] CREATING HACKED MAKERULES...", curr_step, num_steps);
    io::stdout().flush()?;
    let recipe_pattern_c = Regex::new("$(HS_CC) $(GLOBAL) $(LOCAL) -c -o $@ $<").unwrap();
    let recipe_pattern_cc = Regex::new("$(COMPILE_CXX_CP)").unwrap();
    println!(
        "\r[{}/{}] CREATING HACKED MAKERULES...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Restore the original make files
    curr_step = 4;
    print!(
        "[{}/{} RESTORING ORIGINAL MAKERULES...]",
        curr_step, num_steps
    );
    io::stdout().flush()?;
    fs::rename(lastrules_file_bak, lastrules_file)?;
    fs::rename(rules_file_bak, rules_file)?;
    println!(
        "\r[{}/{}] RESTORING ORIGINAL MAKERULES...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Build the target (pseudo)
    curr_step = 5;
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
        .output()?;
    println!(
        "\r[{}/{}] BUILDING TARGET...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Parse the build log
    curr_step = 6;
    print!("[{}/{}] PARSING BUILD LOG...", curr_step, num_steps);
    io::stdout().flush()?;
    println!(
        "\r[{}/{}] PARSING BUILD LOG...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Generate JCDB
    curr_step = 7;
    print!("[{}/{}] GENERATING JCDB...", curr_step, num_steps);
    io::stdout().flush()?;
    println!(
        "\r[{}/{}] GENERATING JCDB...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    let cc_pat = Regex::new(r"(\b[\w-]*(?:g?cc|g++|clang|clang++)\s+[^;]+?)\s*$").unwrap();
    let obj_pat = Regex::new(r#"\s-o\s*(\S+)"#).unwrap();
    let src_pat = Regex::new(r#"\S+\.c(?:c|pp|xx)?\b"#).unwrap();
    let mut logical_line = String::new(); // logical line
    let mut jcdb = json!([]);
    let logfile = File::open(logfile)
        .with_context(|| format!("Cannot open file {}", logfile.as_ref().to_str().unwrap()))?;
    let reader = BufReader::new(logfile);

    for physical_line in reader.lines().map(|l| l.unwrap()) {
        logical_line.push_str(&physical_line);

        // Continues with the next line
        let mut continu = false;
        for c in physical_line.chars().rev() {
            if c != '\\' {
                break;
            }
            continu = !continu;
        }
        if continu {
            logical_line.pop();
            continue;
        }

        let res = cc_pat.captures(&logical_line);
        if let Some(cccap) = res {
            let ccline = cccap.get(1).unwrap().as_str();
            let objfile = match obj_pat.captures(ccline) {
                Some(v) => PathBuf::from(v.get(1).unwrap().as_str()),
                None => {
                    logical_line.clear();
                    continue;
                }
            };
            let srcfile = match src_pat.find(ccline) {
                Some(v) => PathBuf::from(v.as_str()),
                None => {
                    logical_line.clear();
                    continue;
                }
            };

            let mut skip = false;
            let mut directory = PathBuf::new();

            for component in objfile.parent().unwrap().components() {
                if skip {
                    skip = false;
                    continue;
                }
                if component == Component::Normal("target".as_ref()) {
                    skip = true; // skip next component
                    continue;
                }

                directory.push(component);
            }

            jcdb.as_array_mut().unwrap().push(json!({
                "command": ccline,
                "directory": directory.to_str(),
                "file": srcfile,
            }));
        }

        logical_line.clear();
    }

    let outfile = File::create("compile_commands.json")
        .with_context(|| "Failed to create compile_commands.json file")?;
    serde_json::to_writer_pretty(outfile, &jcdb)
        .with_context(|| "Failed to save JSON compilation database")?;

    Ok(())
}
