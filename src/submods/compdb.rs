use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context};
use indicatif::{ProgressBar, ProgressStyle};
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
    // Check if current working directory is svn repo root
    let svninfo = utils::SvnInfo::new()?;
    if env::current_dir()? != svninfo.working_copy_root_path() {
        bail!(
            r#"Location error! Please run this command under the project root, i.e. "{}"."#,
            svninfo.working_copy_root_path().display()
        );
    }

    const LASTRULES_MAKEFILE: &str = "scripts/last-rules.mk";
    const RULES_MAKEFILE: &str = "scripts/rules.mk";
    const TOP_MAKEFILE: &str = "Makefile";

    let lastrules_path = Path::new(LASTRULES_MAKEFILE);
    if !lastrules_path.is_file() {
        bail!(r#"File not found: "{}""#, lastrules_path.display());
    }

    let rules_path = Path::new(RULES_MAKEFILE);
    if !rules_path.is_file() {
        bail!(r#"File not found: "{}""#, rules_path.display());
    }

    let top_makefile = Path::new(TOP_MAKEFILE);
    if !top_makefile.is_file() {
        bail!(r#"File not found: "{}""#, top_makefile.display());
    }

    const NSTEPS: usize = 5;
    let mut step: usize = 1;
    const TICK_INTERVAL: Duration = Duration::from_millis(200);
    const TICK_CHARS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

    // Hack makefiles
    let pb1 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] INJECTING MKFILES ({} & {} & {}) {{spinner:.green}}",
        step,
        NSTEPS,
        lastrules_path.display(),
        rules_path.display(),
        top_makefile.display(),
    ))?.tick_chars(TICK_CHARS));
    pb1.enable_steady_tick(TICK_INTERVAL);
    // Hacking for c files
    let pattern_c = Regex::new(r#"(?m)^\t[[:blank:]]*(\$\(HS_CC\)[[:blank:]]+\$\(CFLAGS[[:word:]]*\)[[:blank:]]+\$\(CFLAGS[[:word:]]*\)[[:blank:]]+-MMD[[:blank:]]+-c[[:blank:]]+-o[[:blank:]]+\$@[[:blank:]]+\$<)[[:blank:]]*$"#)
        .context("Failed to build regex pattern for C-oriented compile command")?;
    let lastrules_text = fs::read_to_string(lastrules_path)
        .context(format!(r#"Can't read file "{}""#, lastrules_path.display()))?;
    let captures = pattern_c
        .captures(&lastrules_text)
        .context(format!("Failed to capture pattern {}", pattern_c.as_str()))?;
    let compline_c = captures.get(0).unwrap().as_str();
    let compcomm_c = captures.get(1).unwrap().as_str();
    let lastrules_text_hacked = pattern_c.replace_all(&lastrules_text, format!("{}\n\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> {} >>:file:>> $<", compline_c, compcomm_c)).to_string();
    fs::write(lastrules_path, &lastrules_text_hacked).context(format!(
        r#"Writing to file "{}" failed"#,
        lastrules_path.display()
    ))?;
    // Hacking for cxx files
    let pattern_cc = Regex::new(r#"(?m)^\t[[:blank:]]*(\$\(COMPILE_CXX_CP_E\))[[:blank:]]*$"#)
        .context("Building regex pattern for C++ compile command failed")?;
    let rules_text = fs::read_to_string(rules_path)
        .context(format!(r#"Can't read file "{}""#, rules_path.display()))?;
    let captures = pattern_cc.captures(&rules_text).context(format!(
        r#"Capturing pattern "{}" failed"#,
        pattern_cc.as_str()
    ))?;
    let compline_cxx = captures.get(0).unwrap().as_str();
    let compcomm_cxx = captures.get(1).unwrap().as_str();
    let rules_text_hacked = pattern_cc.replace_all(&rules_text, format!("{}\n\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> {} >>:file:>> $<", compline_cxx, compcomm_cxx)).to_string();
    fs::write(rules_path, &rules_text_hacked).context(format!(
        r#"Writing to file "{}" failed"#,
        rules_path.display()
    ))?;

    // Hacking for make target
    let pattern_target = Regex::new(r#"(?m)^( *)stoneos-image:(.*)$"#)
        .context("Building regex pattern for make target failed")?;
    let top_makefile_text = fs::read_to_string(top_makefile)
        .context(format!(r#"Can't read file "{}"""#, top_makefile.display()))?;
    let captures = pattern_target
        .captures(&top_makefile_text)
        .context(format!(
            "Can't capture pattern '{}' from '{}'",
            pattern_target.as_str(),
            top_makefile.display()
        ))?;
    let prefix = captures.get(1).unwrap();
    let suffix = captures.get(2).unwrap();
    let top_makefile_text_hacked = pattern_target
        .replace(
            &top_makefile_text,
            format!(
                "stoneos-image: make_sub\n\n{}stoneos-image.orig:{}",
                prefix.as_str(),
                suffix.as_str()
            ),
        )
        .to_string();
    fs::write(top_makefile, &top_makefile_text_hacked)
        .context(format!("Can't write file: '{}'", top_makefile.display()))?;
    pb1.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] INJECTING MKFILES ({} & {} & {} MODIFIED)...{}OK{:#}",
        step,
        NSTEPS,
        lastrules_path.display(),
        rules_path.display(),
        top_makefile.display(),
        COLOR_ANSI_GRN,
        COLOR_ANSI_GRN,
    ))?);
    pb1.finish();

    // Build the target (pseudoly)
    step += 1;
    const BUILDLOG_PATH: &str = ".rua.compdb.tmp";
    let pb2 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] BUILDING PSEUDOLY {{spinner:.green}} [{{elapsed_precise}}]",
        step, NSTEPS
    ))?.tick_chars(TICK_CHARS));
    pb2.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let mut child = command
        .arg(
            &format!("make -C {} {} -iknBj8 ISBUILDRELEASE=1 NOTBUILDUNIWEBUI=1 HS_SHELL_PASSWORD=0 HS_BUILD_COVERITY=0 >{} 2>&1", make_directory, make_target, BUILDLOG_PATH), // This will be treated as a normal arg and passed into hsdocker7
        )
        .spawn()
        .context("Failed to execute hsdocker7")?;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .context("error attempting to wait: pseudo building")?
        {
            break status;
        }
        thread::sleep(TICK_INTERVAL);
    };
    if !status.success() {
        bail!("Pseudo building failed: {:?}", status.code());
    }
    pb2.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] BUILDING PSEUDOLY...{{msg:.green}}",
        step, NSTEPS
    ))?);
    pb2.finish_with_message("OK");

    // Restore the original makefiles
    step += 1;
    let pb3 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] RESTORING MKFILES ({} & {} & {}) {{spinner:.green}}",
        step,
        NSTEPS,
        lastrules_path.display(),
        rules_path.display(),
        top_makefile.display(),
    ))?.tick_chars(TICK_CHARS));
    pb3.enable_steady_tick(TICK_INTERVAL);
    fs::write(lastrules_path, &lastrules_text).context(format!(
        r#"Restoring "{}" failed"#,
        lastrules_path.display()
    ))?;
    fs::write(rules_path, &rules_text)
        .context(format!(r#"Restoring "{}" failed"#, rules_path.display()))?;
    fs::write(top_makefile, &top_makefile_text)
        .context(format!(r#"Restoring "{}" failed"#, top_makefile.display()))?;
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] RESTORING MKFILES ({} & {} & {} RESTORED)...{}OK{:#}",
        step,
        NSTEPS,
        lastrules_path.display(),
        rules_path.display(),
        top_makefile.display(),
        COLOR_ANSI_GRN,
        COLOR_ANSI_GRN,
    ))?);
    pb3.finish();

    // Parse the build log
    step += 1;
    let pb4 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] PARSING BUILDLOG {{spinner:.green}}",
        step, NSTEPS
    ))?.tick_chars(TICK_CHARS));
    pb4.enable_steady_tick(TICK_INTERVAL);
    let output_str = fs::read_to_string(BUILDLOG_PATH)?;
    fs::remove_file(BUILDLOG_PATH)?;
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
    pb4.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] PARSING BUILDLOG...{}OK{:#}",
        step, NSTEPS, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    ))?);
    pb4.finish();

    // Generate JCDB
    step += 1;
    let pb5 = ProgressBar::no_length().with_style(ProgressStyle::with_template(&format!(
        "[{}/{}] GENERATING JCDB {{spinner:.green}}",
        step, NSTEPS
    ))?.tick_chars(TICK_CHARS));
    pb5.enable_steady_tick(TICK_INTERVAL);
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
    pb5.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] GENERATING JCDB...{}OK{:#}",
        step, NSTEPS, COLOR_ANSI_GRN, COLOR_ANSI_GRN
    ))?);
    pb5.finish();

    Ok(())
}
