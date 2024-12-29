use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context};
use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::utils;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
pub(crate) enum CompdbEngine {
    BuiltIn,
    InterceptBuild,
    Bear,
}

impl fmt::Display for CompdbEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuiltIn => write!(f, "built-in"),
            Self::InterceptBuild => write!(f, "intercept-build"),
            Self::Bear => write!(f, "bear"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompdbOptions {
    pub(crate) engine: Option<CompdbEngine>,
    pub(crate) intercept_build_path: Option<String>,
    pub(crate) bear_path: Option<String>,
}

impl fmt::Display for CompdbOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"CompdbOptions {{
    engine: {:?}
    intercept_build_path: {:?}
    bear_path: {:?}
}}"#,
            self.engine, self.intercept_build_path, self.bear_path
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompdbRecord {
    pub command: String,
    pub directory: String,
    pub file: String,
}

impl fmt::Display for CompdbRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{ command: {}, directory: {}, file: {} }}"#,
            self.command, self.directory, self.file
        )
    }
}

pub(crate) type CompDB = Vec<CompdbRecord>;

const DEFAULT_INTERCEPT_BUILD_PATH: &str = "/devel/sw/llvm/bin/intercept-build";
const DEFAULT_BEAR_PATH: &str = "/devel/sw/bear/bin/bear";
const TICK_INTERVAL: Duration = Duration::from_millis(200);
const TICK_CHARS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

pub(crate) fn gen_compdb_using_builtin_method(
    make_directory: &str,
    make_target: &str,
) -> anyhow::Result<()> {
    const NSTEPS: usize = 5;

    // Check if current working directory is svn repo root
    let svninfo = utils::SvnInfo::new()?;
    let at_proj_root = env::current_dir()? == svninfo.working_copy_root_path();

    // Check necessary files
    let lastrules_path = svninfo
        .working_copy_root_path()
        .join("scripts/last-rules.mk");
    if !lastrules_path.is_file() {
        bail!(r#"File not found: "{}""#, lastrules_path.display());
    }

    let rules_path = svninfo.working_copy_root_path().join("scripts/rules.mk");
    if !rules_path.is_file() {
        bail!(r#"File not found: "{}""#, rules_path.display());
    }

    // Optional, only needed for running at project root
    let top_makefile = svninfo.working_copy_root_path().join("Makefile");
    if at_proj_root && !top_makefile.is_file() {
        bail!(r#"File not found: "{}""#, top_makefile.display());
    }

    let mut step: usize = 1;
    let modified_files_hint = if at_proj_root {
        format!(
            "{} & {} & {}",
            lastrules_path.display(),
            rules_path.display(),
            top_makefile.display(),
        )
    } else {
        format!("{} & {}", lastrules_path.display(), rules_path.display())
    };
    let pb1 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(
            format!(
                "[{}/{}] INJECTING MKFILES ({}) {{spinner:.green}}",
                step, NSTEPS, modified_files_hint
            )
            .as_str(),
        )?
        .tick_chars(TICK_CHARS),
    );
    pb1.enable_steady_tick(TICK_INTERVAL);

    // Hacking for c files
    let pattern_c = Regex::new(r#"(?m)^\t[[:blank:]]*\$\(HS_CC\)[[:blank:]]+(\$\(CFLAGS[[:word:]]*\)[[:blank:]]+\$\(CFLAGS[[:word:]]*\)[[:blank:]]+-MMD[[:blank:]]+-c[[:blank:]]+-o[[:blank:]]+\$@[[:blank:]]+\$<)[[:blank:]]*$"#)
        .context("Failed to build regex pattern for C-oriented compile command")?;
    let lastrules_text = fs::read_to_string(lastrules_path.as_path())
        .context(format!(r#"Can't read file "{}""#, lastrules_path.display()))?;
    let captures = pattern_c
        .captures(&lastrules_text)
        .context(format!("Failed to capture pattern {}", pattern_c.as_str()))?;
    let comp_args_c = captures.get(1).unwrap().as_str();
    let lastrules_text_hacked = pattern_c.replace_all(&lastrules_text, format!("\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> $(CC) {} >>:file:>> $<", comp_args_c)).to_string();
    fs::write(lastrules_path.as_path(), &lastrules_text_hacked).context(format!(
        r#"Writing to file "{}" failed"#,
        lastrules_path.display()
    ))?;

    // Hacking for cxx files
    let pattern_cxx = Regex::new(r#"(?m)^\t[[:blank:]]*\$\(COMPILE_CXX_CP_E\)[[:blank:]]*$"#)
        .context("Building regex pattern for C++ compile command failed")?;
    let rules_text = fs::read_to_string(rules_path.as_path())
        .context(format!(r#"Can't read file "{}""#, rules_path.display()))?;
    let rules_text_hacked = pattern_cxx.replace_all(&rules_text, "\t##JCDB## >>:directory:>> $(shell pwd | sed -z 's/\\n//g') >>:command:>> $(COMPILE_CXX_CP) >>:file:>> $<").to_string();
    fs::write(rules_path.as_path(), &rules_text_hacked).context(format!(
        r#"Writing to file "{}" failed"#,
        rules_path.display()
    ))?;

    // Hacking for make target when running at project root
    let mut top_makefile_text = String::new();
    if at_proj_root {
        let pattern_target = Regex::new(r#"(?m)^( *)stoneos-image:(.*)$"#)
            .context("Building regex pattern for make target failed")?;
        top_makefile_text = fs::read_to_string(top_makefile.as_path())
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
        fs::write(top_makefile.as_path(), &top_makefile_text_hacked)
            .context(format!("Can't write file: '{}'", top_makefile.display()))?;
    }
    pb1.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] INJECTING MKFILES ({} MODIFIED)...{{msg:.green}}",
        step, NSTEPS, modified_files_hint
    ))?);
    pb1.finish_with_message("OK");

    // Build the target (pseudoly)
    step += 1;
    const BUILDLOG_PATH: &str = ".rua.compdb.tmp";
    let pb2 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] BUILDING PSEUDOLY {{spinner:.green}} [{{elapsed_precise}}]",
            step, NSTEPS
        ))?
        .tick_chars(TICK_CHARS),
    );
    pb2.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let mut child = command
        .arg(
            format!("make -C {} {} -iknBj8 ISBUILDRELEASE=1 NOTBUILDUNIWEBUI=1 HS_SHELL_PASSWORD=0 HS_BUILD_COVERITY=0 >{} 2>&1", make_directory, make_target, BUILDLOG_PATH), // This will be treated as a normal arg and passed into hsdocker7
        )
        .spawn()
        .context("Failed to execute hsdocker7")?;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .context("Error attempting to wait: pseudo building")?
        {
            break status;
        }
        thread::sleep(TICK_INTERVAL);
    };
    if !status.success() {
        bail!("Pseudo building failed ({:?})", status.code());
    }
    pb2.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] BUILDING PSEUDOLY...{{msg:.green}}",
        step, NSTEPS
    ))?);
    pb2.finish_with_message("OK");

    // Restore the original makefiles
    step += 1;
    let pb3 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] RESTORING MKFILES ({}) {{spinner:.green}}",
            step, NSTEPS, modified_files_hint
        ))?
        .tick_chars(TICK_CHARS),
    );
    pb3.enable_steady_tick(TICK_INTERVAL);
    fs::write(lastrules_path.as_path(), &lastrules_text).context(format!(
        r#"Restoring "{}" failed"#,
        lastrules_path.display()
    ))?;
    fs::write(rules_path.as_path(), &rules_text)
        .context(format!(r#"Restoring "{}" failed"#, rules_path.display()))?;
    if at_proj_root {
        fs::write(top_makefile.as_path(), &top_makefile_text)
            .context(format!(r#"Restoring "{}" failed"#, top_makefile.display()))?;
    }
    pb3.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] RESTORING MKFILES ({} RESTORED)...{{msg:.green}}",
        step, NSTEPS, modified_files_hint
    ))?);
    pb3.finish_with_message("OK");

    // Parse the build log
    step += 1;
    let pb4 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] PARSING BUILDLOG {{spinner:.green}}",
            step, NSTEPS
        ))?
        .tick_chars(TICK_CHARS),
    );
    pb4.enable_steady_tick(TICK_INTERVAL);
    let output_str = fs::read_to_string(BUILDLOG_PATH)?;
    fs::remove_file(BUILDLOG_PATH)?;
    let pattern_hackrule = Regex::new(
        r#"(?m)^##JCDB##[[:blank:]]+>>:directory:>>[[:blank:]]+([^>]+?)[[:blank:]]+>>:command:>>[[:blank:]]+([^>]+?)[[:blank:]]+>>:file:>>[[:blank:]]+(.+)[[:blank:]]*$"#,
    ).context("Failed to build pattern for hackrules")?;
    let mut records: Vec<CompdbRecord> = Vec::new();
    for (_, [dirc, comm, file]) in pattern_hackrule
        .captures_iter(&output_str)
        .map(|c| c.extract())
    {
        records.push(CompdbRecord {
            directory: dirc.to_string(),
            command: comm.to_string(),
            file: Path::new(&dirc).join(file).to_string_lossy().to_string(),
        });
    }
    pb4.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] PARSING BUILDLOG...{{msg:.green}}",
        step, NSTEPS
    ))?);
    pb4.finish_with_message("OK");

    // Generate JCDB
    step += 1;
    let pb5 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] GENERATING JCDB {{spinner:.green}}",
            step, NSTEPS
        ))?
        .tick_chars(TICK_CHARS),
    );
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
        "[{}/{}] GENERATING JCDB...{{msg:.green}}",
        step, NSTEPS
    ))?);
    pb5.finish_with_message("OK");

    Ok(())
}

pub(crate) fn gen_compdb_using_intercept_build(
    intercept_build_path: &str,
    make_directory: &str,
    make_target: &str,
) -> anyhow::Result<()> {
    let pb = ProgressBar::no_length().with_style(ProgressStyle::with_template(
        "GENERATING JCDB USING INTERCEPT-BUILD {spinner:.green}",
    )?);
    pb.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let command_with_args = command.arg(format!(
        "{} make -C {} -j8 -B {}",
        intercept_build_path, make_directory, make_target
    ));
    let mut child_proc = command_with_args
        .spawn()
        .context("Error spawning child process")?;
    let status = loop {
        if let Some(v) = child_proc.try_wait()? {
            break v;
        }
        thread::sleep(TICK_INTERVAL);
    };
    if !status.success() {
        bail!("Intercept-build running failed ({:?})", status.code());
    }
    pb.disable_steady_tick();
    pb.set_style(ProgressStyle::with_template(
        "GENERATING JCDB USING INTERCEPT-BUILD...{msg: .green}",
    )?);
    pb.finish_with_message("OK");
    Ok(())
}

pub(crate) fn gen_compdb_using_bear(
    bear_path: &str,
    make_directory: &str,
    make_target: &str,
) -> anyhow::Result<()> {
    let pb = ProgressBar::no_length().with_style(ProgressStyle::with_template(
        "GENERATING JCDB USING BEAR {spinner:.green}",
    )?);
    pb.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let command_with_args = command.arg(format!(
        "{} -- make -C {} -j8 -B {}",
        bear_path, make_directory, make_target
    ));
    let mut child_proc = command_with_args
        .spawn()
        .context("Error spawning child process")?;
    let status = loop {
        if let Some(v) = child_proc.try_wait()? {
            break v;
        }
        thread::sleep(TICK_INTERVAL);
    };
    if !status.success() {
        bail!("Bear running failed ({:?})", status.code());
    }
    pb.disable_steady_tick();
    pb.set_style(ProgressStyle::with_template(
        "GENERATING JCDB USING BEAR...{msg: .green}",
    )?);
    pb.finish_with_message("OK");
    Ok(())
}

pub(crate) fn gen_compdb(
    make_directory: &str,
    make_target: &str,
    options: CompdbOptions,
) -> anyhow::Result<()> {
    let engine = options.engine.unwrap_or(CompdbEngine::BuiltIn);

    match engine {
        CompdbEngine::BuiltIn => gen_compdb_using_builtin_method(make_directory, make_target),
        CompdbEngine::InterceptBuild => {
            let intercept_build_path = options
                .intercept_build_path
                .as_deref()
                .unwrap_or(DEFAULT_INTERCEPT_BUILD_PATH);
            gen_compdb_using_intercept_build(intercept_build_path, make_directory, make_target)
        }
        CompdbEngine::Bear => {
            let bear_path = options.bear_path.as_deref().unwrap_or(DEFAULT_BEAR_PATH);
            gen_compdb_using_bear(bear_path, make_directory, make_target)
        }
    }
}
