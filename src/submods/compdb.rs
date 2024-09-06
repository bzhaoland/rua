use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context};
use crossterm::style::Stylize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::utils::SvnInfo;

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
            r#"{{ command: {}, directory: {}, file: {} }}"#,
            self.command, self.directory, self.file
        )
    }
}

pub type CompDB = Vec<CompDBRecord>;

pub fn gen_compdb(make_directory: &str, make_target: &str) -> anyhow::Result<()> {
    let svninfo = SvnInfo::new()?;
    let proj_root = svninfo.working_copy_root_path();

    if env::current_dir()? != proj_root {
        bail!(
            r#"Error location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    let lastrule_mkfile = proj_root.join("scripts/last-rules.mk");
    if !lastrule_mkfile.is_file() {
        bail!(r#"File "{}" not available"#, lastrule_mkfile.display());
    }

    let rules_mkfile = proj_root.join("scripts/rules.mk");
    if !rules_mkfile.is_file() {
        bail!(r#"File "{}" not available"#, rules_mkfile.display());
    }

    const NSTEPS: usize = 5;
    let mut step: usize = 1;
    let mut stdout = io::stdout();

    // Inject hackrule
    print!("[{}/{}] INJECTING MKRULES...", step, NSTEPS);
    stdout.flush()?;
    let pattern_c = Regex::new(r#"(?m)^\t[[:blank:]]*(\$\(HS_CC\)[[:blank:]]+\$\(CFLAGS_GLOBAL_CP\)[[:blank:]]+\$\(CFLAGS_LOCAL_CP[[:word:]]*\)[[:blank:]]+-MMD[[:blank:]]+-c[[:blank:]]+-o[[:blank:]]+\$@[[:blank:]]+\$<)[[:blank:]]*$"#).context(format!("Error building regex pattern for C compile command"))?;
    let lastrule_text_orig = fs::read_to_string(lastrule_mkfile.as_path())?;
    let captures = pattern_c
        .captures(&lastrule_text_orig)
        .context("Error capturing pattern_c")?;
    let compile_command_c = captures.get(1).unwrap().as_str();
    let lastrule_text_hacked = pattern_c.replace_all(&lastrule_text_orig, format!("\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> {} >>:file:>> $<", compile_command_c)).to_string();
    fs::write(&lastrule_mkfile, lastrule_text_hacked).context(format!(
        r#"Error writing file "{}""#,
        lastrule_mkfile.display()
    ))?;
    let pattern_cc = Regex::new(r#"(?m)^\t[[:blank:]]*\$\(COMPILE_CXX_CP_E\)[[:blank:]]*$"#)
        .context("Error building regex pattern for C++ compile command")?;
    let rules_text_orig = fs::read_to_string(&rules_mkfile).context(format!(
        r#"Error reading file "{}""#,
        rules_mkfile.display()
    ))?;
    let rules_text_hacked = pattern_cc.replace_all(&rules_text_orig, "\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> $(COMPILE_CXX_CP) >>:file:>> $<").to_string();
    fs::write(&rules_mkfile, rules_text_hacked)?;
    println!(
        "\r[{}/{}] INJECTING MKRULES...{}\x1B[0K",
        step,
        NSTEPS,
        "DONE".dark_green()
    );
    // bail!("");

    // Build the target (pseudo)
    step += 1;
    print!("[{}/{}] BUILDING PSEUDOLY...", step, NSTEPS);
    stdout.flush()?;
    let output = Command::new("hsdocker7")
        .args([
            "make",
            "-C",
            make_directory,
            make_target,
            "-j8",
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
        bail!("Pseudo building failed: {}", status);
    }
    println!(
        "\r[{}/{}] BUILDING PSEUDOLY...{}\x1B[0K",
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    // Restore the original makefiles
    step += 1;
    print!("[{}/{}] RESTORING MKRULES...", step, NSTEPS);
    stdout.flush()?;
    fs::write(&lastrule_mkfile, lastrule_text_orig)
        .context(format!("Error writing {}", lastrule_mkfile.display()))?;
    fs::write(&rules_mkfile, rules_text_orig)
        .context(format!("Error writing {}", rules_mkfile.display()))?;
    println!(
        "\r[{}/{}] RESTORING MKRULES...{}\x1B[0K",
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    // Parse the build log
    step += 1;
    print!("[{}/{}] PARSING BUILDLOG...", step, NSTEPS);
    stdout.flush()?;
    let output_str = String::from_utf8(output.stdout).context("Error creating string")?;
    let pattern_hackrule = Regex::new(
        r#"(?m)^##JCDB##[[:blank:]]+>>:directory:>>[[:blank:]]+([^>]+?)[[:blank:]]+>>:command:>>[[:blank:]]+([^>]+?)[[:blank:]]+>>:file:>>[[:blank:]]+(.+)[[:blank:]]*$"#,
    ).context("Error building hackrule pattern")?;
    let mut records: Vec<CompDBRecord> = Vec::new();
    for (_, [dirc, comm, file]) in pattern_hackrule
        .captures_iter(&output_str)
        .map(|c| c.extract())
    {
        records.push(CompDBRecord {
            directory: dirc.to_string(),
            command: comm.to_string(),
            file: Path::new(&dirc).join(file).to_string_lossy().to_string(),
        });
    }
    println!(
        "\r[{}/{}] PARSING BUILDLOG...{}\x1B[0K",
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    // Generate JCDB
    step += 1;
    print!("[{}/{}] GENERATING JCDB...", step, NSTEPS);
    stdout.flush()?;
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
        "DONE".dark_green()
    );

    Ok(())
}
