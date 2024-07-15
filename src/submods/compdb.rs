use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Error, Result};
use console::Style;
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

    // Style control
    let color_grn = Style::new().green();
    let color_red = Style::new().red();

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
        println!("{}", color_red.apply_to("FAILED"));
        return Result::Err(Error::msg("Error: Run this command under project root."));
    }
    println!("{}", color_grn.apply_to("OK"));

    // Inject hacked make rules
    step += 1;
    print!("[{}/{}] INJECTING MAKEFILES...", step, NSTEPS);
    io::stdout().flush()?;
    let recipe_pattern_c = Regex::new(r#"(\t\s*\$\(HS_CC\)\s+\$\(CFLAGS_GLOBAL_CP\)\s+\$\(CFLAGS_LOCAL_CP\)\s+-MMD\s+-c(\s+-E)?\s+-o\s+\$@\s+\$<\s*?\n?)"#).unwrap();
    let lastrules_orig = fs::read_to_string(LASTRULE_MKFILE)?;
    let lastrules_hack = recipe_pattern_c.replace_all(&lastrules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(CC) $(CFLAGS_GLOBAL_CP) $(CFLAGS_LOCAL_CP) -MMD -c$2 -o $$@ $$< >>:file:>> $$<\n${1}").to_string();
    fs::write(LASTRULE_MKFILE, lastrules_hack)?;
    let recipe_pattern_cc = Regex::new(r#"(\t\s*\$\(COMPILE_CXX_CP_E\)(\s+-E)?\s*?\n?)"#).unwrap();
    let rules_orig = fs::read_to_string(RULES_MKFILE)?;
    let rules_hack = recipe_pattern_cc.replace_all(&rules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(COMPILE_CXX_CP)$2 >>:file:>> $$<\n${1}").to_string();
    fs::write(RULES_MKFILE, rules_hack)?;
    println!("{}", color_grn.apply_to("OK"));

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
            println!("{}", color_red.apply_to("FAILED"));
            Error::msg(format!(
                "Failed to execute `hsdocker7 make ...`: {}",
                &e.to_string()
            ))
        })?;
    let status = output.status;
    if !status.success() {
        println!("{}", color_red.apply_to("FAILED"));
        return Result::Err(Error::msg(format!(
            "Error: Failed to build target: {}",
            status
        )));
    }
    println!("{}", color_grn.apply_to("OK"));

    // Restore original makefiles
    step += 1;
    print!("[{}/{}] RESTORING MAKERULES...", step, NSTEPS);
    io::stdout().flush()?;
    fs::write(LASTRULE_MKFILE, lastrules_orig).map_err(|e| {
        println!("{}", color_red.apply_to("FAILED"));
        e
    })?;
    fs::write(RULES_MKFILE, rules_orig).map_err(|e| {
        println!("{}", color_grn.apply_to("FAILED"));
        e
    })?;
    println!("{}", color_grn.apply_to("OK"));

    // Parse the build log
    step += 1;
    print!("[{}/{}] PARSING BUILD LOG...", step, NSTEPS);
    io::stdout().flush()?;
    let output_str = String::from_utf8(output.stdout).map_err(|e| {
        println!("{}", color_red.apply_to("FAILED"));
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
    println!("{}", color_grn.apply_to("OK"));

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
    println!("{}", color_grn.apply_to("OK"));

    Ok(())
}
