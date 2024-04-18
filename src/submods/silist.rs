use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use anyhow::{Error, Result};
use colored::Colorize;
use regex::Regex;

pub fn gen_silist(product_dir: &str, make_target: &str, project_root: &str) -> Result<()> {
    let lastrules_file = "./scripts/last-rules.mk";
    let rules_file = "./scripts/rules.mk";
    let num_steps = 6usize;
    let mut curr_step;

    // Checking running directory, should run under project root
    curr_step = 1usize;
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
        return Err(Error::msg("Run this command under project root."));
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
        .output()?;
    if output.status.success() != true {
        eprintln!(
            "\r[{}/{}] BUILDING TARGET...{}\x1B[0K",
            curr_step,
            num_steps,
            "FAILED".red()
        );
        return Err(Error::msg("error: failed to build target"));
    }
    println!(
        "\r[{}/{}] BUILDING TARGET...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Restore the original make files
    curr_step = 4;
    print!("[{}/{} RESTORING MAKERULES...]", curr_step, num_steps);
    io::stdout().flush()?;
    fs::write(lastrules_file, lastrules_orig)?;
    fs::write(rules_file, rules_orig)?;
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
    let output_str = String::from_utf8(output.stdout)?;
    let hackrule_pattern = Regex::new(
        r#"##JCDB##\s+>>:directory:>>\s+([^\n]+?)\s+>>:command:>>\s+([^\n]+?)\s+>>:file:>>\s+([^\n]+)\s*\n?"#,
    )?;
    let current_dir = env::current_dir()?;
    let mut records: Vec<String> = Vec::new();
    for (_, [dirc, _, file]) in hackrule_pattern
        .captures_iter(&output_str)
        .map(|c| c.extract())
    {
        let dirc = dirc.to_string();
        let file = PathBuf::from(&dirc)
            .join(file)
            .strip_prefix(&current_dir)?
            .to_owned();
        let file = PathBuf::from(project_root)
            .join(file)
            .to_string_lossy()
            .to_string();
        records.push(file);
    }
    println!(
        "\r[{}/{}] PARSING BUILD LOG...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    // Generate FILELIST
    curr_step = 6;
    print!("[{}/{}] GENERATING FILELIST...", curr_step, num_steps);
    io::stdout().flush()?;
    let filelist_str = records.join("\r\n");
    fs::write("filelist.txt", filelist_str)?;
    println!(
        "\r[{}/{}] GENERATING FILELIST...{}\x1B[0K",
        curr_step,
        num_steps,
        "OK".green()
    );

    Ok(())
}
