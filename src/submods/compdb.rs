use std::cmp;
use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;

use anstyle::{Ansi256Color, Color, Style};
use anyhow::{Context, bail};
use chrono::TimeZone;
use clap::ValueEnum;
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use rusqlite::OptionalExtension;
use rusqlite::params;
use rusqlite::{self, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use zstd::{decode_all, encode_all};

use crate::utils::SvnInfo;
use crate::utils::progress_bar::{TICK_CHARS, TICK_INTERVAL};

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
    defines: {:?},
    engine: {:?}
    intercept_build_path: {:?}
    bear_path: {:?}
}}"#,
            serde_json::to_string_pretty(&self.defines),
            self.engine,
            self.intercept_build_path,
            self.bear_path
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

pub(crate) fn gen_compdb_builtin(
    svninfo: &SvnInfo,
    make_directory: &str,
    make_target: &str,
    macros: &IndexMap<String, String>,
) -> anyhow::Result<()> {
    const NSTEPS: usize = 5;
    const LAST_RULES_MAKEFILE: &str = "scripts/last-rules.mk";
    const RULES_MAKEFILE: &str = "scripts/rules.mk";
    const TOP_MAKEFILE: &str = "Makefile";

    // Invoke svn firstly to check whether we are in a working copy
    let at_proj_root = env::current_dir()? == svninfo.working_copy_root_path();

    let lastrules_path = svninfo.working_copy_root_path().join(LAST_RULES_MAKEFILE);
    if !lastrules_path.is_file() {
        bail!(r#"File not found: "{}""#, lastrules_path.display());
    }

    let rules_path = svninfo.working_copy_root_path().join(RULES_MAKEFILE);
    if !rules_path.is_file() {
        bail!(r#"File not found: "{}""#, rules_path.display());
    }

    let top_makefile = svninfo.working_copy_root_path().join(TOP_MAKEFILE);
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
        .context("Failed to build regex for C compilation")?;
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
        .context("Building regex pattern for C++ compilation")?;
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
            .context("Failed to build regex for make target")?;
        top_makefile_text = fs::read_to_string(top_makefile.as_path())
            .context(format!(r#"Failed to read "{}"""#, top_makefile.display()))?;
        let captures = pattern_target
            .captures(&top_makefile_text)
            .context(format!(
                "Failed to capture '{}' from '{}'",
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

pub(crate) fn gen_compdb_by_intercept_build(
    _svninfo: &SvnInfo,
    intercept_build_path: &str,
    make_directory: &str,
    make_target: &str,
) -> anyhow::Result<()> {
    let pb = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(
            "Generating JCDB by intercept-build {spinner:.green} [{elapsed_precise}]",
        )?
        .tick_chars(TICK_CHARS),
    );
    pb.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let command_with_args = command.arg(format!(
        "{} make -C {} -j8 {} >{} 2>&1",
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
        "Generating JCDB by intercept-build...{msg:.green}",
    )?);
    pb.finish_with_message("ok");
    Ok(())
}

pub(crate) fn gen_compdb_by_bear(
    _svninfo: &SvnInfo,
    bear_path: &str,
    make_directory: &str,
    make_target: &str,
) -> anyhow::Result<()> {
    let pb = ProgressBar::no_length().with_style(
        ProgressStyle::with_template(
            "Generating JCDB by bear {spinner:.green} [{elapsed_precise}]",
        )?
        .tick_chars(TICK_CHARS),
    );
    pb.enable_steady_tick(TICK_INTERVAL);
    let mut command = Command::new("hsdocker7");
    let command_with_args = command.arg(format!(
        "{} -- make -C {} -j8 {} >{} 2>&1",
        bear_path, make_directory, make_target, BUILDLOG_PATH
    ));
    let mut child_proc = command_with_args
        .spawn()
        .context("Spawn child process failed")?;
    let status = loop {
        if let Some(v) = child_proc.try_wait()? {
            break v;
        }
        thread::sleep(TICK_INTERVAL);
    };
    if !status.success() {
        bail!("Bear run failed ({:?})", status.code());
    }
    fs::remove_file(BUILDLOG_PATH)?;
    pb.disable_steady_tick();
    pb.set_style(ProgressStyle::with_template(
        "Generating JCDB by bear...{msg:.green}",
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
            gen_compdb_builtin(svninfo, make_directory, make_target, &options.defines)
        }
        CompdbEngine::InterceptBuild => {
            let intercept_build_path = options
                .intercept_build_path
                .as_deref()
                .unwrap_or(DEFAULT_INTERCEPT_BUILD_PATH);
            gen_compdb_by_intercept_build(
                svninfo,
                intercept_build_path,
                make_directory,
                make_target,
            )
        }
        CompdbEngine::Bear => {
            let bear_path = options.bear_path.as_deref().unwrap_or(DEFAULT_BEAR_PATH);
            gen_compdb_by_bear(svninfo, bear_path, make_directory, make_target)
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
    // Note that the two generation fields should update independently
    conn.execute("CREATE TABLE IF NOT EXISTS compdbs (generation INTEGER PRIMARY KEY AUTOINCREMENT, branch TEXT NOT NULL, revision INTEGER NOT NULL, target TEXT NOT NULL, timestamp INTEGER NOT NULL, compdb BLOB NOT NULL, name TEXT UNIQUE, remark TEXT)", ())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS history (id INTEGER PRIMARY KEY, generation INTEGER)",
        (),
    )?;
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
const STYLE_GREEN: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(2))));
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
    const HEADERS: (&str, &str, &str, &str, &str, &str, &str, &str) = (
        "Generation",
        "Branch",
        "Revision",
        "Target",
        "Date",
        "Current",
        "Name",
        "Remark",
    );
    let mut generation_cols = HEADERS.0.chars().count();
    let mut branch_cols = HEADERS.1.chars().count();
    let mut revision_cols = HEADERS.2.chars().count();
    let mut target_cols = HEADERS.3.chars().count();
    let mut date_cols = HEADERS.4.chars().count();
    let current_cols = HEADERS.6.chars().count();
    let mut name_cols = HEADERS.5.chars().count();
    let mut remark_cols = HEADERS.7.chars().count();
    let current = history_get_current(conn)?;
    let indicator = "★";
    let indicator_len = indicator.chars().count();

    let mut displayed_entries: Vec<(i64, String, i64, String, String, bool, String, String)> =
        Vec::new();
    for item in data_iter {
        let item = item?;
        let date = chrono::Local
            .timestamp_opt(item.timestamp, 0)
            .unwrap()
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let name = item.name.unwrap_or_else(|| "".to_string());
        let remark = item.remark.unwrap_or_else(|| "".to_string());

        generation_cols = cmp::max(
            item.generation.to_string().chars().count() + indicator_len,
            generation_cols,
        );
        branch_cols = cmp::max(item.branch.chars().count(), branch_cols);
        revision_cols = cmp::max(item.revision.to_string().chars().count(), revision_cols);
        target_cols = cmp::max(item.target.chars().count(), target_cols);
        date_cols = cmp::max(date.chars().count(), date_cols);
        name_cols = cmp::max(name.chars().count(), name_cols);
        remark_cols = cmp::max(remark.chars().count(), remark_cols);

        displayed_entries.push((
            item.generation,
            item.branch,
            item.revision,
            item.target,
            date,
            if let Some(current) = current {
                if item.generation == current {
                    true
                } else {
                    false
                }
            } else {
                false
            },
            name,
            remark,
        ));
    }

    if displayed_entries.is_empty() {
        println!("No compilation database generation available");
        return Ok(());
    }

    println!(
        "{0}{1:<generation_cols$}   {2:<branch_cols$}   {3:<revision_cols$}   {4:<target_cols$}   {5:<date_cols$}   {6:<name_cols$}   {7:current_cols$}   {8:<remark_cols$}{0:#}",
        STYLE_BOLD,
        HEADERS.0,
        HEADERS.1,
        HEADERS.2,
        HEADERS.3,
        HEADERS.4,
        HEADERS.5,
        HEADERS.6,
        HEADERS.7,
    );
    for item in displayed_entries {
        println!(
            "{1:<generation_cols$}   {2:branch_cols$}   {3:<revision_cols$}   {4:target_cols$}   {5:<date_cols$}   {0}{6:^current_cols$}{0:#}   {7:<name_cols$}   {8:<remark_cols$}",
            STYLE_GREEN,
            item.0,
            item.1,
            item.2,
            item.3,
            item.4,
            if item.5 { indicator } else { "" },
            item.6,
            item.7
        );
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub(crate) enum DelOpt {
    Generations(Vec<i64>),
    Oldest(usize),
    Newest(usize),
    All,
}

/// Delete a compilation database generation from the store
pub(crate) fn del_compdb(conn: &Connection, opt: DelOpt) -> anyhow::Result<usize> {
    let rows = match opt {
        DelOpt::Generations(v) => {
            conn.execute(
                format!(
                    "DELETE FROM compdbs WHERE generation IN ({})",
                    v.iter().map(|_| "?").collect::<Vec<&str>>().join(", ")
                ).as_str(),
                rusqlite::params_from_iter(v.iter())
            )?
        }
        DelOpt::All => conn.execute("DELETE FROM compdbs", ())?,
        DelOpt::Newest(n) => {
            conn.execute(
                "DELETE FROM compdbs WHERE timestamp in (SELECT timestamp FROM compdbs ORDER BY generation DESC LIMIT ?1)",
                [n]
            )?
        }
        DelOpt::Oldest(n) => {
            conn.execute(
                "DELETE FROM compdbs WHERE timestamp in (SELECT timestamp FROM compdbs ORDER BY generation ASC LIMIT ?1)",
                [n]
            )?
        }
    };
    conn.execute("VACUUM", ())?;
    Ok(rows)
}

pub(crate) fn use_compdb(conn: &Connection, generation: i64) -> anyhow::Result<()> {
    let item: (Option<String>, Vec<u8>) = conn.query_row(
        "SELECT name, compdb FROM compdbs WHERE generation=?1",
        [generation],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let compile_commands = decode_all(&item.1[..])?;
    fs::write(COMPDB_FILE, compile_commands)?;
    history_set_current(conn, generation)?;
    Ok(())
}

/// Archive the compilation database into store as a new generation and
/// optionally update history table
pub(crate) fn ark_compdb<P>(
    conn: &Connection,
    branch: &str,
    revision: i64,
    target: &str,
    compdb: P,
) -> anyhow::Result<usize>
where
    P: AsRef<Path>,
{
    let compdb = compdb.as_ref();
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
/// Returns the number of affected rows, non-zero on success, zero on failure
pub(crate) fn mark_compdb(
    conn: &Connection,
    generation_id: i64,
    remark: &str,
) -> anyhow::Result<usize> {
    let rows = conn.execute(
        "UPDATE compdbs SET remark = ?1 WHERE generation = ?2",
        params![remark, generation_id],
    )?;
    Ok(rows)
}

/// Get the most recent compilation database which equips with the biggest generation id
pub(crate) fn get_first_compdb(conn: &Connection) -> anyhow::Result<Option<i64>> {
    let generation = conn.query_row(
        "SELECT generation FROM compdbs ORDER BY generation DESC LIMIT 1",
        (),
        |row| row.get(0),
    )?;

    Ok(generation)
}

/// Set the currently used compdb to the specified generation id
/// Please note this only takes effect on compdbs managed by store
pub(crate) fn history_set_current(conn: &Connection, generation: i64) -> anyhow::Result<usize> {
    let rows = conn.execute(
        "INSERT INTO history (generation) VALUES (?1)",
        rusqlite::params![generation],
    )?;
    Ok(rows)
}

/// Get the generation id of the currently used compdb
pub(crate) fn history_get_current(conn: &Connection) -> anyhow::Result<Option<i64>> {
    let generation: Option<i64> = conn
        .query_row(
            "SELECT generation FROM history ORDER BY id DESC LIMIT 1",
            (),
            |row| row.get(0),
        )
        .optional()?;
    Ok(generation)
}
