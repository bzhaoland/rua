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

    let makefile_1 = proj_root.join("scripts/last-rules.mk");
    if !makefile_1.is_file() {
        bail!(r#"Makefile "{}" not found"#, makefile_1.display());
    }

    let makefile_2 = proj_root.join("scripts/rules.mk");
    if !makefile_2.is_file() {
        bail!(r#"Makefile "{}" not found"#, makefile_2.display());
    }

    const NSTEPS: usize = 5;
    let mut step: usize = 1;
    let mut stdout = io::stdout();

    // Inject hackrule
    print!(
        "[{}/{}] INJECTING MKFILES(MODIFYING {}&{})...",
        step,
        NSTEPS,
        makefile_1.display(),
        makefile_2.display()
    );
    stdout.flush()?;
    let pattern_c = Regex::new(r#"(?m)^\t[[:blank:]]*(\$\(HS_CC\)[[:blank:]]+\$\(CFLAGS\)[[:word:]]+\$\(CFLAGS[[:word:]]*\)[[:blank:]]+-MMD[[:blank:]]+-c[[:blank:]]+-o[[:blank:]]+\$@[[:blank:]]+\$<)[[:blank:]]*$"#)
        .context(format!("Error building regex pattern for C compile command"))?;
    let makerule_1 = fs::read_to_string(makefile_1.as_path())?;
    let captures = pattern_c
        .captures(&makerule_1)
        .context("Error capturing pattern_c")?;
    let compile_command_c = captures.get(1).unwrap().as_str();
    let makerule_1_hacked = pattern_c.replace_all(&makerule_1, format!("\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> {} >>:file:>> $<", compile_command_c)).to_string();
    fs::write(&makefile_1, makerule_1_hacked)
        .context(format!(r#"Error writing file "{}""#, makefile_1.display()))?;
    let pattern_cc = Regex::new(r#"(?m)^\t[[:blank:]]*\$\(COMPILE_CXX_CP_E\)[[:blank:]]*$"#)
        .context("Error building regex pattern for C++ compile command")?;
    let makerule_2 = fs::read_to_string(&makefile_2)
        .context(format!(r#"Error reading file "{}""#, makefile_2.display()))?;
    let makerule_2_hacked = pattern_cc.replace_all(&makerule_2, "\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> $(COMPILE_CXX_CP) >>:file:>> $<").to_string();
    fs::write(&makefile_2, makerule_2_hacked)?;
    println!(
        "\r[{}/{}] INJECTING MKFILES(MODIFIED {}&{})...{}\x1B[0K",
        step,
        NSTEPS,
        makefile_1.display(),
        makefile_2.display(),
        "DONE".dark_green()
    );

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
        .context("Dry-run command `hsdocker7 make ...` failed")?;
    let status = output.status;
    if !status.success() {
        bail!("Error building pseudoly: {:?}", status.code());
    }
    println!(
        "\r[{}/{}] BUILDING PSEUDOLY...{}\x1B[0K",
        step,
        NSTEPS,
        "DONE".dark_green()
    );

    // Restore the original makefiles
    step += 1;
    print!("[{}/{}] RESTORING MKFILES...", step, NSTEPS);
    stdout.flush()?;
    fs::write(&makefile_1, makerule_1)
        .context(format!("Error writing {}", makefile_1.display()))?;
    fs::write(&makefile_2, makerule_2)
        .context(format!("Error writing {}", makefile_2.display()))?;
    println!(
        "\r[{}/{}] RESTORING MKFILES({}&{} RESTORED)...{}\x1B[0K",
        step,
        NSTEPS,
        makefile_1.display(),
        makefile_2.display(),
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
