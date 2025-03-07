use std::cmp;
use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;

use anstyle::Style;
use anyhow::{bail, Context};
use chrono::TimeZone;
use clap::ValueEnum;
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use rusqlite::params;
use rusqlite::OptionalExtension;
use rusqlite::{self, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use zstd::{decode_all, encode_all};

use crate::utils::{SvnInfo, TICK_CHARS, TICK_INTERVAL};

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
    pub(crate) defines: IndexMap<String, String>,
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

pub(crate) const COMPDB_FILE: &str = "compile_commands.json";
pub(crate) const COMPDB_STORE: &str = ".rua/compdbs.db3";
const DEFAULT_BEAR_PATH: &str = "/devel/sw/bear/bin/bear";
const DEFAULT_INTERCEPT_BUILD_PATH: &str = "/devel/sw/llvm/bin/intercept-build";
const BUILDLOG_PATH: &str = ".rua.compdb.tmp";

pub(crate) fn gen_compdb_using_builtin_method(
    svninfo: &SvnInfo,
    make_directory: &str,
    make_target: &str,
    macros: &IndexMap<String, String>,
) -> anyhow::Result<()> {
    const NSTEPS: usize = 5;
    const LAST_RULES_MAKEFILE: &str = "scripts/last-rules.mk";
    const RULES_MAKEFILE: &str = "scripts/rules.mk";
    const TOP_MAKEFILE: &str = "Makefile";

    // Check if current working directory is svn repo root
    let at_proj_root = env::current_dir()? == svninfo.working_copy_root_path();

    // Check necessary files
    let lastrules_path = svninfo.working_copy_root_path().join(LAST_RULES_MAKEFILE);
    if !lastrules_path.is_file() {
        bail!(r#"File unavailable: "{}""#, lastrules_path.display());
    }

    let rules_path = svninfo.working_copy_root_path().join(RULES_MAKEFILE);
    if !rules_path.is_file() {
        bail!(r#"File unavailable: "{}""#, rules_path.display());
    }

    // Optional, only needed for running at project root
    let top_makefile = svninfo.working_copy_root_path().join(TOP_MAKEFILE);
    if at_proj_root && !top_makefile.is_file() {
        bail!(r#"File unavailable: "{}""#, top_makefile.display());
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
                "[{}/{}] Injecting mkfiles ({}) {{spinner:.green}}",
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
        "[{}/{}] Injecting makefiles ({} modified)...{{msg:.green}}",
        step, NSTEPS, modified_files_hint
    ))?);
    pb1.finish_with_message("ok");

    // Build the target (pseudoly)
    step += 1;
    let pb2 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] Building pseudoly {{spinner:.green}} [{{elapsed_precise}}]",
            step, NSTEPS
        ))?
        .tick_chars(TICK_CHARS),
    );
    pb2.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let vars = macros
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<String>>()
        .join(" ");
    let mut child = command
        .arg(
             // This whole line will be treated as a normal arg and passed into hsdocker7
            format!("make -C {} {} -iknBj8 ISBUILDRELEASE=1 NOTBUILDUNIWEBUI=1 HS_BUILD_COVERITY=0 {} >{} 2>&1", make_directory, make_target, vars, BUILDLOG_PATH),
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
        "[{}/{}] Building pseudoly...{{msg:.green}}",
        step, NSTEPS
    ))?);
    pb2.finish_with_message("ok");

    // Restore the original makefiles
    step += 1;
    let pb3 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] Restoring mkfiles ({}) {{spinner:.green}}",
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
        "[{}/{}] Restoring mkfiles ({} restored)...{{msg:.green}}",
        step, NSTEPS, modified_files_hint
    ))?);
    pb3.finish_with_message("ok");

    // Parse the build log
    step += 1;
    let pb4 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] Parsing buildlog {{spinner:.green}}",
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
        "[{}/{}] Parsing buildlog...{{msg:.green}}",
        step, NSTEPS
    ))?);
    pb4.finish_with_message("ok");

    // Generate JCDB
    step += 1;
    let pb5 = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(&format!(
            "[{}/{}] Generating JCDB {{spinner:.green}}",
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
    fs::write(COMPDB_FILE, serde_json::to_string_pretty(&jcdb)?)?;
    pb5.set_style(ProgressStyle::with_template(&format!(
        "[{}/{}] Generating JCDB...{{msg:.green}}",
        step, NSTEPS
    ))?);
    pb5.finish_with_message("ok");

    Ok(())
}

pub(crate) fn gen_compdb_using_intercept_build(
    _svninfo: &SvnInfo,
    intercept_build_path: &str,
    make_directory: &str,
    make_target: &str,
) -> anyhow::Result<()> {
    let pb = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(
            "Generating JCDB using intercept-build {spinner:.green} [{elapsed_precise}]",
        )?
        .tick_chars(TICK_CHARS),
    );
    pb.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let command_with_args = command.arg(format!(
        "{} make -C {} -j8 -B {} >{} 2>&1",
        intercept_build_path, make_directory, make_target, BUILDLOG_PATH
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
    fs::remove_file(BUILDLOG_PATH)?;
    pb.disable_steady_tick();
    pb.set_style(ProgressStyle::with_template(
        "Generating JCDB using intercept-build...{msg:.green}",
    )?);
    pb.finish_with_message("ok");
    Ok(())
}

pub(crate) fn gen_compdb_using_bear(
    _svninfo: &SvnInfo,
    bear_path: &str,
    make_directory: &str,
    make_target: &str,
) -> anyhow::Result<()> {
    let pb = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(
            "Generating JCDB using bear {spinner:.green} [{elapsed_precise}]",
        )?
        .tick_chars(TICK_CHARS),
    );
    pb.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let command_with_args = command.arg(format!(
        "{} -- make -C {} -j8 -B {} >{} 2>&1",
        bear_path, make_directory, make_target, BUILDLOG_PATH
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
    fs::remove_file(BUILDLOG_PATH)?;
    pb.disable_steady_tick();
    pb.set_style(ProgressStyle::with_template(
        "Generating JCDB using bear...{msg:.green}",
    )?);
    pb.finish_with_message("ok");
    Ok(())
}

pub(crate) fn gen_compdb(
    svninfo: &SvnInfo,
    make_directory: &str,
    make_target: &str,
    options: CompdbOptions,
) -> anyhow::Result<()> {
    let engine = options.engine.unwrap_or(CompdbEngine::BuiltIn);

    match engine {
        CompdbEngine::BuiltIn => {
            gen_compdb_using_builtin_method(svninfo, make_directory, make_target, &options.defines)
        }
        CompdbEngine::InterceptBuild => {
            let intercept_build_path = options
                .intercept_build_path
                .as_deref()
                .unwrap_or(DEFAULT_INTERCEPT_BUILD_PATH);
            gen_compdb_using_intercept_build(
                svninfo,
                intercept_build_path,
                make_directory,
                make_target,
            )
        }
        CompdbEngine::Bear => {
            let bear_path = options.bear_path.as_deref().unwrap_or(DEFAULT_BEAR_PATH);
            gen_compdb_using_bear(svninfo, bear_path, make_directory, make_target)
        }
    }?;

    Ok(())
}

#[allow(unused)]
#[derive(Clone, Debug)]
struct CompdbItem {
    generation: i64,
    name: Option<String>,
    branch: String,
    revision: i64,
    target: String,
    timestamp: i64,
    compdb: Vec<u8>,
    remark: Option<String>,
}

pub(crate) fn create_compdbs_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute("CREATE TABLE IF NOT EXISTS compdbs (generation INTEGER PRIMARY KEY AUTOINCREMENT, branch TEXT NOT NULL, revision INTEGER NOT NULL, target TEXT NOT NULL, timestamp INTEGER NOT NULL, compdb BLOB NOT NULL, name TEXT UNIQUE, remark TEXT)", ())?;
    Ok(())
}

fn add_compdb(
    conn: &Connection,
    branch: &str,
    revision: i64,
    target: &str,
    compdb: &[u8],
) -> anyhow::Result<usize> {
    let timestamp = chrono::Utc::now().timestamp();
    let rows = conn.execute("INSERT INTO compdbs (branch, revision, target, timestamp, compdb) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![
        branch, revision, target, timestamp, compdb
    ])?;
    Ok(rows)
}

const STYLE_BOLD: Style = Style::new().bold();
pub(crate) fn list_compdbs(conn: &Connection) -> anyhow::Result<()> {
    // Database querying
    let mut stmt = conn.prepare("SELECT generation, branch, revision, target, timestamp, name, remark FROM compdbs ORDER BY generation DESC")?;
    let data_iter = stmt.query_map([], |row| {
        Ok(CompdbItem {
            generation: row.get(0)?,
            branch: row.get(1)?,
            revision: row.get(2)?,
            target: row.get(3)?,
            timestamp: row.get(4)?,
            compdb: Vec::new(), // Fake content as the content in this field is huge
            name: row.get(5)?,
            remark: row.get(6)?,
        })
    })?;

    // Formatting
    const HEADERS: (&str, &str, &str, &str, &str, &str, &str) = (
        "Generation",
        "Branch",
        "Revision",
        "Target",
        "Date",
        "Name",
        "Remark",
    );
    let mut generation_col_width = HEADERS.0.len();
    let mut branch_col_width = HEADERS.1.len();
    let mut revision_col_width = HEADERS.2.len();
    let mut target_col_width = HEADERS.3.len();
    let mut date_col_width = HEADERS.4.len();
    let mut name_col_width = HEADERS.5.len();
    let mut remark_col_width = HEADERS.6.len();

    let mut displayed_entries: Vec<(i64, String, i64, String, String, String, String)> = Vec::new();
    for item in data_iter {
        let item = item?;
        let date = chrono::Local
            .timestamp_opt(item.timestamp, 0)
            .unwrap()
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let name = item.name.unwrap_or_else(|| "".to_string());
        let remark = item.remark.unwrap_or_else(|| "".to_string());

        generation_col_width = cmp::max(
            item.generation.to_string().chars().count(),
            generation_col_width,
        );
        branch_col_width = cmp::max(item.branch.chars().count(), branch_col_width);
        revision_col_width = cmp::max(
            item.revision.to_string().chars().count(),
            revision_col_width,
        );
        target_col_width = cmp::max(item.target.chars().count(), target_col_width);
        date_col_width = cmp::max(date.chars().count(), date_col_width);
        name_col_width = cmp::max(name.chars().count(), name_col_width);
        remark_col_width = cmp::max(remark.chars().count(), remark_col_width);

        displayed_entries.push((
            item.generation,
            item.branch,
            item.revision,
            item.target,
            date,
            name,
            remark,
        ));
    }

    if displayed_entries.is_empty() {
        println!("No compilation database generation available");
        return Ok(());
    }

    println!(
        "{}{:<generation_col_width$}   {:<branch_col_width$}   {:<revision_col_width$}   {:<target_col_width$}   {:<date_col_width$}   {:<name_col_width$}   {:<remark_col_width$}{:#}",
        STYLE_BOLD, HEADERS.0, HEADERS.1, HEADERS.2, HEADERS.3, HEADERS.4, HEADERS.5, HEADERS.6, STYLE_BOLD
    );
    for item in displayed_entries {
        println!(
        "{:<generation_col_width$}   {:<branch_col_width$}   {:<revision_col_width$}   {:<target_col_width$}   {:<date_col_width$}   {:<name_col_width$}   {:<remark_col_width$}",
            item.0, item.1, item.2, item.3, item.4, item.5, item.6
        );
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DelOpt {
    Generation(i64),
    Oldest(usize),
    Newest(usize),
    All,
}

/// Delete a compilation database generation from the store
pub(crate) fn del_compdb(conn: &Connection, opt: DelOpt) -> anyhow::Result<usize> {
    let rows = match opt {
        DelOpt::Generation(v) => {
            conn.execute("DELETE FROM compdbs WHERE generation = ?1", [v])?
        }
        DelOpt::All => conn.execute("DELETE FROM compdbs", ())?,
        DelOpt::Newest(n) => {
            conn.execute("DELETE FROM compdbs WHERE timestamp in (SELECT timestamp FROM compdbs ORDER BY generation DESC LIMIT ?1)", [n])?
        }
        DelOpt::Oldest(n) => {
            conn.execute("DELETE FROM compdbs WHERE timestamp in (SELECT timestamp FROM compdbs ORDER BY generation ASC LIMIT ?1)", [n])?
        }
    };
    conn.execute("VACUUM", ())?;
    Ok(rows)
}

pub(crate) fn use_compdb(conn: &Connection, generation: i64) -> anyhow::Result<()> {
    let item: Option<(Option<String>, Vec<u8>)> = conn
        .query_row(
            "SELECT name, compdb FROM compdbs WHERE generation=?1",
            [generation],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let item = item.context("Invalid generation id")?;
    let compile_commands = decode_all(&item.1[..])?;
    fs::write(COMPDB_FILE, compile_commands)?;
    Ok(())
}

/// Archive the compilation database into store as a new generation
pub(crate) fn ark_compdb<P>(
    conn: &Connection,
    target: &str,
    branch: &str,
    revision: i64,
    compdb: P,
) -> anyhow::Result<usize>
where
    P: AsRef<Path>,
{
    let content = fs::read_to_string(compdb)?;
    let compressed = encode_all(content.as_bytes(), 0)?;
    let rows = add_compdb(conn, branch, revision, target, &compressed)?;
    Ok(rows)
}

/// Name a compilation database generation in the store
///
/// Returns the number of rows that were changed, 1 on success, 0 on failure.
pub(crate) fn name_compdb(conn: &Connection, generation: i64, name: &str) -> anyhow::Result<usize> {
    let rows = conn.execute(
        "UPDATE compdbs SET name = ?1 WHERE generation = ?2",
        params![name, generation],
    )?;
    Ok(rows)
}

/// Remark a compilation database generation
///
/// Returns the number of affected rows, 1 on success, 0 on failure
pub(crate) fn remark_compdb(
    conn: &Connection,
    generation: i64,
    remark: &str,
) -> anyhow::Result<usize> {
    let rows = conn.execute(
        "UPDATE compdbs SET remark = ?1 WHERE generation = ?2",
        params![remark, generation],
    )?;
    Ok(rows)
}
