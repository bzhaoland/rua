use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use crossterm::style::Stylize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompDBRecord {
    pub command: String,
    pub directory: String,
    pub file: String,
}

impl fmt::Display for CompDBRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r"{{ command: {}, directory: {}, file: {} }}",
            self.command, self.directory, self.file
        )
    }
}

pub type CompDB = Vec<CompDBRecord>;

pub fn gen_compdb(product_dir: &str, make_target: &str) -> Result<()> {
    // Resources later used
    const LASTRULE_MKFILE: &str = "./scripts/last-rules.mk";
    const RULES_MKFILE: &str = "./scripts/rules.mk";
    const NSTEPS: usize = 6;
    let mut step: usize = 1;

    // Checking working directory, should run under project root
    print!("[{}/{}] CHECKING LOCATION...", step, NSTEPS);
    io::stdout().flush()?;
    let mut location_ok = true;
    let attr = fs::metadata(LASTRULE_MKFILE);
    if attr.is_err() || !attr.unwrap().is_file() {
        location_ok = false;
    }
    let attr = fs::metadata(RULES_MKFILE);
    if attr.is_err() || !attr.unwrap().is_file() {
        location_ok = false;
    }
    if !location_ok {
        println!("{}", "FAILED".red());
        bail!("Error location! Run command under project root.");
    }
    println!("{}", "OK".green());

    // Inject hacked make rules
    step += 1;
    print!("[{}/{}] INJECTING MKRULES...", step, NSTEPS);
    io::stdout().flush()?;
    let recipe_pattern_c = Regex::new(r#"(\t\s*\$\(HS_CC\)\s+\$\(CFLAGS_GLOBAL_CP\)\s+\$\(CFLAGS_LOCAL_CP\)\s+-MMD\s+-c(\s+-E)?\s+-o\s+\$@\s+\$<\s*?\n?)"#).unwrap();
    let lastrules_orig = fs::read_to_string(LASTRULE_MKFILE)?;
    let lastrules_hack = recipe_pattern_c.replace_all(&lastrules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(CC) $(CFLAGS_GLOBAL_CP) $(CFLAGS_LOCAL_CP) -MMD -c$2 -o $$@ $$< >>:file:>> $$<\n${1}").to_string();
    fs::write(LASTRULE_MKFILE, lastrules_hack)?;
    let recipe_pattern_cc = Regex::new(r#"(\t\s*\$\(COMPILE_CXX_CP_E\)(\s+-E)?\s*?\n?)"#).unwrap();
    let rules_orig = fs::read_to_string(RULES_MKFILE)?;
    let rules_hack = recipe_pattern_cc.replace_all(&rules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(COMPILE_CXX_CP)$2 >>:file:>> $$<\n${1}").to_string();
    fs::write(RULES_MKFILE, rules_hack)?;
    println!(
        "\r[{}/{}] INJECTING MAKERULES...{}\x1B[0K",
        step,
        NSTEPS,
        "OK".green()
    );

    // Build the target (pseudo)
    step += 1;
    print!("[{}/{}] BUILDING TARGET...", step, NSTEPS);
    io::stdout().flush()?;
    let output = Command::new("hsdocker7")
        .args([
            "make",
            "-C",
            product_dir,
            make_target,
            "-j16",
            "-iknwB",
            "HS_BUILD_COVERITY=0",
            "ISBUILDRELEASE=1",
            "HS_BUILD_UNIWEBUI=0",
            "HS_SHELL_PASSWORD=0",
            "IMG_NAME=RUAIHA",
        ])
        .output()
        .map_err(|e| {
            println!("{}", "FAILED".red());
            anyhow!("Failed to execute `hsdocker7 make ...`: {}", &e.to_string())
        })?;
    let status = output.status;
    if !status.success() {
        println!("{}", "FAILED".red());
        return Result::Err(anyhow!("Error: Failed to build target: {}", status));
    }
    println!(
        "\r[{}/{}] BUILDING TARGET...{}\x1B[0K",
        step,
        NSTEPS,
        "OK".green()
    );

    // Restore original makefiles
    step += 1;
    print!("[{}/{}] RESTORING MKRULES...", step, NSTEPS);
    io::stdout().flush()?;
    fs::write(LASTRULE_MKFILE, lastrules_orig).map_err(|e| {
        println!(
            "\r[{}/{}] RESTORING MKRULES...{}\x1B[0K",
            step,
            NSTEPS,
            "FAILED".red()
        );
        e
    })?;
    fs::write(RULES_MKFILE, rules_orig).map_err(|e| {
        println!(
            "\r[{}/{}] RESTORING MKRULES...{}\x1B[0K",
            step,
            NSTEPS,
            "FAILED".red()
        );
        e
    })?;
    println!(
        "\r[{}/{}] RESTORING MKRULES...{}\x1B[0K",
        step,
        NSTEPS,
        "OK".green()
    );

    // Parse the build log
    step += 1;
    print!("[{}/{}] PARSING BUILDLOG...", step, NSTEPS);
    io::stdout().flush()?;
    let output_str = String::from_utf8(output.stdout).map_err(|e| {
        println!(
            "[{}/{}] PARSING BUILDLOG...{}",
            step,
            NSTEPS,
            "FAILED".red()
        );
        e
    })?;
    let hackrule_pattern = Regex::new(
        r#"##JCDB##\s+>>:directory:>>\s+([^\n]+?)\s+>>:command:>>\s+([^\n]+?)\s+>>:file:>>\s+([^\n]+)\s*\n?"#,
    )?;
    let mut records: Vec<CompDBRecord> = Vec::new();
    for (_, [dirc, comm, file]) in hackrule_pattern
        .captures_iter(&output_str)
        .map(|c| c.extract())
    {
        let dirc = dirc.to_string();
        let comm = comm.to_string();
        let file = PathBuf::from(&dirc)
            .join(file)
            .to_string_lossy()
            .to_string();
        records.push(CompDBRecord {
            directory: dirc,
            command: comm,
            file,
        });
    }
    println!(
        "\r[{}/{}] PARSING BUILDLOG...{}\x1B[0K",
        step,
        NSTEPS,
        "OK".green()
    );

    // Generate JCDB
    step += 1;
    print!("[{}/{}] GENERATING JCDB...", step, NSTEPS);
    io::stdout().flush()?;
    let mut jcdb = json!([]);
    for item in records.iter() {
        jcdb.as_array_mut().unwrap().push(json!({
            "directory": item.directory,
            "command": item.command,
            "file": item.file,
        }));
    }
    fs::write(
        "compile_commands.json",
        serde_json::to_string_pretty(&jcdb)?,
    )?;
    println!(
        "\r[{}/{}] GENERATING JCDB...{}\x1B[0K",
        step,
        NSTEPS,
        "OK".green()
    );

    Ok(())
}
