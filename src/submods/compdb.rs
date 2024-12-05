use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

use anyhow::{bail, Context};
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
            r#"Location error! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    let makefile_1 = Path::new("scripts/last-rules.mk");
    if !makefile_1.is_file() {
        bail!(r#"File not found: "{}""#, makefile_1.display());
    }

    let makefile_2 = Path::new("scripts/rules.mk");
    if !makefile_2.is_file() {
        bail!(r#"File not found: "{}""#, makefile_2.display());
    }

    let makefile_top = Path::new("Makefile");
    if !makefile_top.is_file() {
        bail!(r#"File not found: "{}""#, makefile_2.display());
    }

    const NSTEPS: usize = 5;
    let mut step: usize = 1;
    let mut stdout = io::stdout();

    // Inject hackrule
    print!(
        "[{}/{}] INJECTING MKFILES...({} & {} & {})",
        step,
        NSTEPS,
        makefile_1.display(),
        makefile_2.display(),
        makefile_top.display(),
    );
    stdout.flush()?;

    // Hacking for c files
    let pattern_c = Regex::new(r#"(?m)^\t[[:blank:]]*(\$\(HS_CC\)[[:blank:]]+\$\(CFLAGS[[:word:]]*\)[[:blank:]]+\$\(CFLAGS[[:word:]]*\)[[:blank:]]+-MMD[[:blank:]]+-c[[:blank:]]+-o[[:blank:]]+\$@[[:blank:]]+\$<)[[:blank:]]*$"#)
        .context("Failed to build regex pattern for C-oriented compile command")?;
    let maketext_1 = fs::read_to_string(makefile_1)
        .context(format!(r#"Can't read file "{}""#, makefile_1.display()))?;
    let captures = pattern_c
        .captures(&maketext_1)
        .context(format!("Failed to capture pattern {}", pattern_c.as_str()))?;
    let compline_c = captures.get(0).unwrap().as_str();
    let compcomm_c = captures.get(1).unwrap().as_str();
    let maketext_1_hacked = pattern_c.replace_all(&maketext_1, format!("{}\n\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> {} >>:file:>> $<", compline_c, compcomm_c)).to_string();
    fs::write(makefile_1, &maketext_1_hacked).context(format!(
        r#"Writing to file "{}" failed"#,
        makefile_1.display()
    ))?;

    // Hacking for cxx files
    let pattern_cc = Regex::new(r#"(?m)^\t[[:blank:]]*(\$\(COMPILE_CXX_CP_E\))[[:blank:]]*$"#)
        .context("Building regex pattern for C++ compile command failed")?;
    let maketext_2 = fs::read_to_string(makefile_2)
        .context(format!(r#"Can't read file "{}""#, makefile_2.display()))?;
    let captures = pattern_cc.captures(&maketext_2).context(format!(
        r#"Capturing pattern "{}" failed"#,
        pattern_cc.as_str()
    ))?;
    let compline_cxx = captures.get(0).unwrap().as_str();
    let compcomm_cxx = captures.get(1).unwrap().as_str();
    let maketext_2_hacked = pattern_cc.replace_all(&maketext_2, format!("{}\n\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> {} >>:file:>> $<", compline_cxx, compcomm_cxx)).to_string();
    fs::write(makefile_2, &maketext_2_hacked).context(format!(
        r#"Writing to file "{}" failed"#,
        makefile_2.display()
    ))?;

    // Hacking for make target
    let pattern_make = Regex::new(r#"(?m)^( *)stoneos-image:(.*)$"#)
        .context("Building regex pattern for make target failed")?;
    let maketext_top = fs::read_to_string(makefile_top)
        .context(format!(r#"Can't read file "{}"""#, makefile_top.display()))?;
    let captures = pattern_make.captures(&maketext_top).context(format!(
        "Can't capture pattern '{}' from '{}'",
        pattern_make.as_str(),
        makefile_top.display()
    ))?;
    let prefix = captures.get(1).unwrap();
    let suffix = captures.get(2).unwrap();
    let maketext_top_hacked = pattern_make
        .replace(
            &maketext_top,
            format!(
                "stoneos-image: make_sub\n\n{}stoneos-image.orig:{}",
                prefix.as_str(),
                suffix.as_str()
            ),
        )
        .to_string();
    fs::write(makefile_top, &maketext_top_hacked)
        .context(format!("Can't write file: '{}'", makefile_top.display()))?;

    println!(
        "\r[{}/{}] INJECTING MKFILES...{}DONE{:#}({} & {} & {} MODIFIED)\x1B[0K",
        step,
        NSTEPS,
        COLOR_ANSI_GRN,
        COLOR_ANSI_GRN,
        makefile_1.display(),
        makefile_2.display(),
        makefile_top.display(),
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
        make_target, // special target for submodules
        "-j8",
        "-iknB", // pseudo building
        "ISBUILDRELEASE=1",
        "NOTBUILDUNIWEBUI=1",
        "HS_SHELL_PASSWORD=0",
        "HS_BUILD_COVERITY=0",
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
        "[{}/{}] RESTORING MKFILES...({} & {} & {})",
        step,
        NSTEPS,
        makefile_1.display(),
        makefile_2.display(),
        makefile_top.display(),
    );
    stdout.flush()?;
    fs::write(makefile_1, &maketext_1)
        .context(format!(r#"Restoring "{}" failed"#, makefile_1.display()))?;
    fs::write(makefile_2, &maketext_2)
        .context(format!(r#"Restoring "{}" failed"#, makefile_2.display()))?;
    fs::write(makefile_top, &maketext_top)
        .context(format!(r#"Restoring "{}" failed"#, makefile_top.display()))?;
    println!(
        "\r[{}/{}] RESTORING MKFILES...{}DONE{:#}({} & {} & {} RESTORED)\x1B[0K",
        step,
        NSTEPS,
        COLOR_ANSI_GRN,
        COLOR_ANSI_GRN,
        makefile_1.display(),
        makefile_2.display(),
        makefile_top.display(),
    );

    // Parse the build log
    step += 1;
    print!("[{}/{}] PARSING BUILDLOG...", step, NSTEPS);
    stdout.flush()?;
    let output_str = String::from_utf8(output.stdout)?;
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
