use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path;
use std::process;

use anyhow::{bail, Context};
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
            r#"{{ command: {}, directory: {}, file: {} }}"#,
            self.command, self.directory, self.file
        )
    }
}

pub type CompDB = Vec<CompDBRecord>;

const COLOR_ANSI_GRN: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));

pub fn gen_compdb(make_directory: &str, make_target: &str) -> anyhow::Result<()> {
    let svninfo = utils::SvnInfo::new()?;
    let proj_root = svninfo.working_copy_root_path();

    if env::current_dir()? != proj_root {
        bail!(
            r#"Wrong location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    let makefile_1 = path::Path::new("scripts/last-rules.mk");
    if !makefile_1.is_file() {
        bail!(r#"Makefile "{}" not found"#, makefile_1.display());
    }

    let makefile_2 = path::Path::new("scripts/rules.mk");
    if !makefile_2.is_file() {
        bail!(r#"Makefile "{}" not found"#, makefile_2.display());
    }

    const NSTEPS: usize = 5;
    let mut step: usize = 1;
    let mut stdout = io::stdout();

    // Inject hackrule
    print!(
        "[{}/{}] INJECTING MKFILES...({}&{})",
        step,
        NSTEPS,
        makefile_1.display(),
        makefile_2.display()
    );
    stdout.flush()?;

    let pattern_c = regex::Regex::new(r#"(?m)^\t[[:blank:]]*(\$\(HS_CC\)[[:blank:]]+\$\(CFLAGS[[:word:]]*\)[[:blank:]]+\$\(CFLAGS[[:word:]]*\)[[:blank:]]+-MMD[[:blank:]]+-c[[:blank:]]+-o[[:blank:]]+\$@[[:blank:]]+\$<)[[:blank:]]*$"#)
        .context("Failed to build regex pattern for C-oriented compile command")?;
    let maketext_1 = fs::read_to_string(makefile_1)?;
    let captures = pattern_c
        .captures(&maketext_1)
        .context(format!("Failed to capture pattern {}", pattern_c.as_str()))?;
    let compline_c = captures.get(0).unwrap().as_str();
    let compcomm_c = captures.get(1).unwrap().as_str();
    let makerule_1_hacked = pattern_c.replace_all(&maketext_1, format!("{}\n\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> {} >>:file:>> $<", compline_c, compcomm_c)).to_string();
    fs::write(makefile_1, makerule_1_hacked).context(format!(
        r#"Writing to file "{}" failed"#,
        makefile_1.display()
    ))?;

    let pattern_cc =
        regex::Regex::new(r#"(?m)^\t[[:blank:]]*(\$\(COMPILE_CXX_CP_E\))[[:blank:]]*$"#)
            .context("Building regex pattern for C++ compile command failed")?;
    let maketext_2 = fs::read_to_string(makefile_2)
        .context(format!(r#"Reading file "{}" failed"#, makefile_2.display()))?;
    let captures = pattern_cc.captures(&maketext_2).context(format!(
        r#"Capturing pattern "{}" failed"#,
        pattern_cc.as_str()
    ))?;
    let compline_cxx = captures.get(0).unwrap().as_str();
    let compcomm_cxx = captures.get(1).unwrap().as_str();
    let makerule_2_hacked = pattern_cc.replace_all(&maketext_2, format!("{}\n\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> {} >>:file:>> $<", compline_cxx, compcomm_cxx)).to_string();

    fs::write(makefile_2, makerule_2_hacked)?;
    println!(
        "\r[{}/{}] INJECTING MKFILES...{}DONE{:#}({} & {} MODIFIED)\x1B[0K",
        step,
        NSTEPS,
        COLOR_ANSI_GRN,
        COLOR_ANSI_GRN,
        makefile_1.display(),
        makefile_2.display()
    );

    // Build the target (pseudoly)
    step += 1;
    print!("[{}/{}] BUILDING PSEUDOLY...", step, NSTEPS);
    stdout.flush()?;
    let mut prog = process::Command::new("hsdocker7");
    let cmd = &mut prog.args([
        "make",
        "-C",
        make_directory,
        make_target,
        "-j8",
        "-iknwB", // pseudo building
        "HS_BUILD_COVERITY=0",
        "ISBUILDRELEASE=1",
        "HS_BUILD_UNIWEBUI=0",
        "HS_SHELL_PASSWORD=0",
        "IMG_NAME=RUA.DUMMY",
    ]);
    let output = cmd
        .output()
        .context("Failed to perform `hsdocker7 make ...`")?;
    let status = output.status;
    if !status.success() {
        bail!("Pseudo building failed: {:?}", status.code());
    }
    println!(
        "\r[{}/{}] BUILDING PSEUDOLY...{}DONE{:#}\x1B[0K",
        step, NSTEPS, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    );

    // Restore the original makefiles
    step += 1;
    print!(
        "[{}/{}] RESTORING MKFILES...({} & {})",
        step,
        NSTEPS,
        makefile_1.display(),
        makefile_2.display()
    );
    stdout.flush()?;
    fs::write(makefile_1, maketext_1)
        .context(format!(r#"Restoring "{}" failed"#, makefile_1.display()))?;
    fs::write(makefile_2, maketext_2)
        .context(format!(r#"Restoring "{}" failed"#, makefile_2.display()))?;
    println!(
        "\r[{}/{}] RESTORING MKFILES...{}DONE{:#}({} & {} RESTORED)\x1B[0K",
        step,
        NSTEPS,
        COLOR_ANSI_GRN,
        COLOR_ANSI_GRN,
        makefile_1.display(),
        makefile_2.display()
    );

    // Parse the build log
    step += 1;
    print!("[{}/{}] PARSING BUILDLOG...", step, NSTEPS);
    stdout.flush()?;
    let output_str = String::from_utf8(output.stdout).context("Error creating string")?;
    let pattern_hackrule = regex::Regex::new(
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
            file: path::Path::new(&dirc)
                .join(file)
                .to_string_lossy()
                .to_string(),
        });
    }
    println!(
        "\r[{}/{}] PARSING BUILDLOG...{}DONE{:#}\x1B[0K",
        step, NSTEPS, COLOR_ANSI_GRN, COLOR_ANSI_GRN
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
        "\r[{}/{}] GENERATING JCDB...{}DONE{:#}\x1B[0K",
        step, NSTEPS, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    );

    Ok(())
}
