use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::bail;
use anyhow::Context;
use crossterm::style::Stylize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::utils;

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

pub fn gen_compdb(product_dir: &str, make_target: &str) -> anyhow::Result<()> {
    // Must run under the project root
    if !utils::is_at_proj_root()? {
        anyhow::bail!("Location error! Please run command under the project root.");
    }

    // Files to be used
    const LASTRULE_MKFILE: &str = "./scripts/last-rules.mk";
    const RULES_MKFILE: &str = "./scripts/rules.mk";
    if !(Path::new(LASTRULE_MKFILE).is_file() && Path::new(LASTRULE_MKFILE).is_file()) {
        anyhow::bail!(
            r#"File "{}" or "{}" not found"#,
            LASTRULE_MKFILE,
            RULES_MKFILE
        );
    }

    const NSTEPS: usize = 5;
    let mut step: usize = 1;

    // Inject hackrule
    println!("[{}/{}] INJECTING MKRULES...", step, NSTEPS);
    let pattern_c = Regex::new(r#"(?m)^\t\s*\$\(HS_CC\)\s+\$\(CFLAGS_GLOBAL_CP\)\s+\$\(CFLAGS_LOCAL_CP\)\s+-MMD\s+-c\s+-o\s+\$@\s+\$<\s*?$"#).unwrap();
    let lastrules_orig = fs::read_to_string(LASTRULE_MKFILE)?;
    let lastrules_hacked = pattern_c.replace_all(&lastrules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(CC) $(CFLAGS_GLOBAL_CP) $(CFLAGS_LOCAL_CP) -MMD -c -o $$@ $$< >>:file:>> $$<").to_string();
    fs::write(LASTRULE_MKFILE, lastrules_hacked)?;
    let pattern_cc = Regex::new(r#"(?m)^\t\s*\$\(COMPILE_CXX_CP_E\)\s*?$"#).unwrap();
    let rules_orig = fs::read_to_string(RULES_MKFILE)?;
    let rules_hacked = pattern_cc.replace_all(&rules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(COMPILE_CXX_CP) >>:file:>> $$<").to_string();
    fs::write(RULES_MKFILE, rules_hacked)?;
    println!(
        r#"\x1B[1A\x1B[2K\r[{}/{}] INJECTING MKRULES...{}"#,
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    // Build the target (pseudo)
    step += 1;
    println!("[{}/{}] BUILDING PSEUDOLY...", step, NSTEPS);
    let output = Command::new("hsdocker7")
        .args([
            "make",
            "-C",
            product_dir,
            make_target,
            "-j16",
            "-iknwB", // For pseudo building forcefully
            "HS_BUILD_COVERITY=0",
            "ISBUILDRELEASE=1",
            "HS_BUILD_UNIWEBUI=0",
            "HS_SHELL_PASSWORD=0",
            "IMG_NAME=RUAIHA",
        ])
        .output()
        .context("Command `hsdocker7 make ...` failed")?;
    let status = output.status;
    if !status.success() {
        bail!("Pseudoly building failed: {}", status);
    }
    println!(
        r#"\x1B[1A\x1B[2K\r[{}/{}] BUILDING PSEUDOLY...{}"#,
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    // Restore original makefiles
    step += 1;
    println!("[{}/{}] RESTORING MKRULES...", step, NSTEPS);
    fs::write(LASTRULE_MKFILE, lastrules_orig)
        .context(format!("Error writing to {}", LASTRULE_MKFILE))?;
    fs::write(RULES_MKFILE, rules_orig).context(format!("Error writing to {}", RULES_MKFILE))?;
    println!(
        r#"\x1B[1A\x1B[2K\r[{}/{}] RESTORING MKRULES...{}"#,
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    // Parse the build log
    step += 1;
    println!("[{}/{}] PARSING BUILDLOG...", step, NSTEPS);
    let output_str = String::from_utf8(output.stdout).context("Error creating string")?;
    let hackrule_pattern = Regex::new(
        r#"(?m)^##JCDB##\s+>>:directory:>>\s+([^>]+?)\s+>>:command:>>\s+([^>]+?)\s+>>:file:>>\s+(.+)\s*?$"#,
    ).context("Error creating hackrule pattern")?;
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
        r#"\x1B[1A\x1B[2K\r[{}/{}] PARSING BUILDLOG...{}"#,
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    // Generate JCDB
    step += 1;
    println!("[{}/{}] GENERATING JCDB...", step, NSTEPS);
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
        r#"\x1B[1A\x1B[2K\r[{}/{}] GENERATING JCDB...{}"#,
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    Ok(())
}
