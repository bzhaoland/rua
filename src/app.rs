use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs, io};

use anstyle::{Ansi256Color, Color, Style};
use anyhow::{Context, Result, bail};
use clap::builder::styling;
use clap::{ArgGroup, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::Connection;

use crate::config::{CLANGD_CACHE, COMPDB_FILE, COMPDB_STORE, PROJ_LEVEL_RUA_DIR, RuaConf};
use crate::submods::clean;
use crate::submods::compdb::{self, CompdbEngine};
use crate::submods::mkinfo::{self, GenBy, MakeOpts};
use crate::submods::perfan;
use crate::submods::review;
use crate::submods::shinit;
use crate::submods::showcc;
use crate::submods::silist;
use crate::utils;

const STYLE_YELLOW: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(3))));
const STYLE_YELLOW_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(3))))
    .bold();
const STYLE_GREEN: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(2))));
const STYLE_CYAN: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(6))));
const STYLE_RED: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(1))));
const STYLE_RED_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(1))))
    .bold();
const STYLE_ITALIC: Style = Style::new().italic();
const STYLES: styling::Styles = styling::Styles::styled()
    .header(STYLE_YELLOW_BOLD)
    .usage(STYLE_YELLOW_BOLD)
    .literal(STYLE_GREEN)
    .placeholder(STYLE_CYAN);

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum CompdbCommand {
    /// Generate a JSON compilation database (JCDB) for the given target.
    ///
    /// Run this command under either project root or submod dir. If you want a
    /// a compilation database for a specific module, run under submod dir. You
    /// may have to compile the target first before generating the compilation
    /// database under submod dir.
    ///
    /// Note:
    /// 1. Compilation database generated under submod dir only covers
    ///    files in this module.
    /// 2. R4+ releases are supported by compdb.
    #[command(visible_aliases = ["generate"],
        after_help = format!(
        r#"{0}Examples:{0:#}
  rua compdb gen products/ngfw_as a-dnv                    # For A1000/A2000...
  rua compdb gen products/ngfw_as a-dnv-ipv6               # For A1000/A2000... with IPv6 support
  rua compdb gen -e intercept-build products/ngfw_as a-dnv # For A1000/A2000... using intercept-build
  rua compdb gen . a-dnv                                   # For A1000/A2000... under submod dir
  rua compdb gen -e bear . a-dnv                           # For A1000/A2000... under submod dir using bear 
  run compdb gen -e intercept-build . a-dnv                # For A1000/A2000... under submod dir using intercept-build

{1}Caution:{1:#}
  Some files are modified while running in built-in mode which is the default
  and faster:
  1. When running under project root dir:
     - scripts/last-rules.mk
     - scripts/rules.mk
     - Makefile
  2. When running under submod dir:
     - scripts/last-rules.mk
     - scripts/rules.mk
  These files may be left dirty if compdb aborted unexpectedly. You can restore
  them by command (make sure you have backed up the changes you made):
  {2}svn revert Makefile scripts/last-rules.mk scripts/rules.mk{2:#}"#,
      STYLE_YELLOW_BOLD,
      STYLE_RED_BOLD,
      STYLE_YELLOW))]
    Gen {
        #[arg(
            short = 'D',
            long = "define",
            value_name = "KEY=VAL",
            help = "Define a variable which will be passed to the underlying make command"
        )]
        defines: Vec<String>,

        #[arg(
            short = 'e',
            long = "engine",
            value_name = "ENGINE",
            help = "Engine for generating compilation database (defaults to built-in)"
        )]
        engine: Option<CompdbEngine>,

        #[arg(
            short = 'b',
            long = "bear-path",
            value_name = "BEAR",
            help = "Path to the bear binary (defaults to /devel/sw/bear/bin/bear)"
        )]
        bear_path: Option<String>,

        #[arg(
            short = 'i',
            long = "intercept-build-path",
            value_name = "INTERCEPT-BUILD",
            help = "Path to the intercept-build binary (defaults to /devel/sw/llvm/bin/intercept-build)"
        )]
        intercept_build_path: Option<String>,

        #[arg(
            value_name = "PATH",
            help = "Path for the target where platform-specific makefiles reside, such as 'products/vfw'"
        )]
        product_dir: String,

        #[arg(value_name = "TARGET", help = "Target to build, such as 'a-dnv'")]
        make_target: String,
    },

    /// Archive the currently used compilation database into store as a new generation
    #[command(visible_aliases = ["ark", "archive"], after_help = format!(
        r#"{0}Examples:{0:#}
    rua compdb add hygon-ipv6 # Archive compilation database for hygon-ipv6
    rua compdb add --revision 307164 hygon # Archive compilation database for hygon with a revision provided"#,
    STYLE_YELLOW_BOLD
    ))]
    Add {
        #[arg(
            value_name = "TARGET",
            help = "Target specified for the compilation database"
        )]
        target: String,

        #[arg(
            short = 'r',
            long = "revision",
            value_name = "REVISION",
            help = "Revision for compilation database (defaults to current repo revision)"
        )]
        revision: Option<i64>,

        #[arg(
            short = 'f',
            long = "compilation-database",
            value_name = "COMPILATION-DATABASE",
            help = "Use this compilation database other than the default (compile_commands.json)"
        )]
        compdb_path: Option<String>,
    },

    /// Delete compilation database generation(s) from store
    #[command(visible_aliases = ["delete", "rm", "remove"], group = ArgGroup::new("number").args(["some", "all", "new", "old"]))]
    Del {
        #[arg(value_name = "GENERATION-ID", help = "Generations to remove")]
        some: Option<Vec<i64>>,

        #[arg(short = 'a', long = "all", help = "Remove all generations")]
        all: bool,

        #[arg(
            short = 'n',
            long = "new",
            value_name = "N",
            help = format!("Remove {}N{:#} newest generations", STYLE_ITALIC, STYLE_ITALIC)
        )]
        new: Option<usize>,

        #[arg(
            short = 'o',
            long = "old",
            value_name = "N",
            help = format!("Remove {}N{:#} oldest generations", STYLE_ITALIC, STYLE_ITALIC)
        )]
        old: Option<usize>,
    },

    /// List all compilation database generations in store
    #[command(visible_alias = "list")]
    Ls,

    /// Select a compilation database generation from store to use
    Use {
        #[arg(value_name = "GENERATION", help = "Compilation database generation id")]
        generation: i64,
    },

    /// Name a compilation database generation
    Name {
        #[arg(
            value_name = "GENERATION",
            help = "The compilation database generation"
        )]
        generation: i64,

        #[arg(value_name = "NAME", help = "Name for the compilation database")]
        name: String,
    },

    /// Remark a compilation database generation
    Remark {
        #[arg(
            value_name = "GENERATION",
            help = "The compilation database generation"
        )]
        generation: i64,

        #[arg(
            value_name = "REMARK",
            help = "Remark the compilation database generation"
        )]
        remark: String,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum Comm {
    /// Clean build files (run under project root)
    #[command(after_help = format!(r#"{0}Examples:{0:#}
  rua clean  # Clean the entire project

{1}Caution:{1:#}
  All unversioned files will be {2}REMOVED{2:#} permanantly, including files created by YOU but not
  added to SVN. Use it carefully!"#,
  STYLE_YELLOW_BOLD,
  STYLE_RED_BOLD,
  STYLE_RED))]
    Clean {
        #[arg(
            value_name = "ENTRY",
            help = "Files or dirs to be cleaned ('target' is always included even if not specified)"
        )]
        dirs: Option<Vec<String>>,

        #[arg(
            short = 'n',
            long = "ignore",
            value_name = "FILE",
            help = "File or directory to be ignored while cleaning. You can add multiple ignores by specifying this option multiple times"
        )]
        ignores: Option<Vec<String>>,
    },

    /// Manipulate compilation database.
    Compdb {
        #[clap(subcommand)]
        compdb_comm: CompdbCommand,
    },

    /// Get all matched makeinfos for product
    ///
    /// Note: R6+ releases are supported by mkinfo.
    #[command(
        after_help = format!(r#"{0}Examples:{0:#}
  rua mkinfo A1000      # Makeinfo for A1000 without extra features
  rua mkinfo -6 A1000   # Makeinfo for A1000 with IPv6 enabled
  rua mkinfo -6w 'X\d+' # Makeinfos for X-series products with IPv6 and WebUI enabled using regex pattern
  rua mkinfo --by-target a-dnv  # Makeinfos for a-dnv target"#, STYLE_YELLOW_BOLD)
    )]
    Mkinfo {
        /// Build with IPv6 enabled
        #[arg(short = '6', long = "ipv6", default_value_t = false)]
        ipv6: bool,

        /// Enable coverage
        #[arg(short = 'g', long = "coverage", default_value_t = false)]
        coverage: bool,

        /// Enable coverity
        #[arg(short = 'c', long = "coverity", default_value_t = false)]
        coverity: bool,

        /// Build in debug mode
        #[arg(short = 'd', long = "debug", default_value_t = false)]
        debug: bool,

        /// Output format for makeinfos
        #[arg(long = "format", default_value = "list", value_name = "FORMAT")]
        output_format: mkinfo::DumpFormat,

        /// Build with shell password enabled
        #[arg(short = 'p', long = "password", default_value_t = false)]
        password: bool,

        /// Build with WebUI enabled
        #[arg(short = 'w', long = "webui")]
        webui: bool,

        /// Server to upload the output image to
        #[arg(short = 's', long = "image-server", value_name = "IMAGE-SERVER")]
        image_server: Option<mkinfo::ImageServer>,

        /// Binaries without stripping
        #[arg(long = "nostrip", value_name = "BINARY")]
        bins_without_strip: Vec<String>,

        /// Treat the positional arg as a build target other than a product name
        #[arg(long = "by-target")]
        by_target: bool,

        /// Product name like A1000, or compile target (when specify --by-target) like a-dnv.
        /// Can also be provided in regex like 'X\d+80' representing X6180/X7180/X8180, etc.
        #[arg(value_name = "NAME")]
        name: String,
    },

    /// Extensively map instructions to file locations (inline expanded)
    Perfan {
        #[arg(help = "Profiling text generated by perfan", value_name = "FILE")]
        file: PathBuf,

        #[arg(
            short = 'o',
            long = "format",
            value_name = "FORMAT",
            default_value = "table",
            help = "Output format"
        )]
        format: perfan::DumpFormat,

        #[arg(
            short = 'e',
            long = "elf",
            visible_aliases= ["exe", "executable"],
            value_name = "ELF",
            help = "Binary files used for addresses resolving"
        )]
        elfs: Vec<PathBuf>,
    },

    /// Start a new review request or refresh the existing one if review-id provided
    Review {
        #[arg(
            short = 'n',
            long = "bug",
            value_name = "BUG",
            help = "Bug id for this review request (required)"
        )]
        bug_id: u32,

        #[arg(
            short = 'r',
            long = "review-id",
            value_name = "REVIEW-ID",
            help = "Existing review id"
        )]
        review_id: Option<u32>,

        #[arg(
            short = 'd',
            long = "diff-file",
            value_name = "DIFF-FILE",
            help = "Diff file to be used"
        )]
        diff_file: Option<String>,

        #[arg(
            short = 'u',
            long = "reviewers",
            value_name = "REVIEWERS",
            help = "Reviewers"
        )]
        reviewers: Option<Vec<String>>,

        #[arg(
            short = 'b',
            long = "branch",
            value_name = "BRANCH",
            help = "Branch name for this commit"
        )]
        branch_name: Option<String>,

        #[arg(
            short = 'p',
            long = "repo",
            value_name = "REPO",
            help = "Repository name"
        )]
        repo_name: Option<String>,

        #[arg(
            short = 's',
            long = "revision",
            value_name = "REVISION",
            help = "Revision to be used"
        )]
        revisions: Option<String>,

        #[arg(
            short = 't',
            long = "template-file",
            value_name = "TEMPLATE-FILE",
            help = "Use customized template file (please ensure it can run through svn commit hooks)"
        )]
        template_file: Option<String>,

        #[arg(value_name = "FILE", help = "Files to be reviewed")]
        files: Option<Vec<String>>,
    },

    /// Show all possible compile commands for filename (based on compilation database)
    Showcc {
        #[arg(
            value_name = "SOURCE-FILE",
            help = "Source file name for which to fetch all the available compile commands"
        )]
        comp_unit: String,
        #[arg(
            value_name = "COMPDB",
            short = 'c',
            long = "compdb",
            help = r#"Compilation database (defaults to file "compile_commands.json" in the current directory)"#
        )]
        comp_db: Option<String>,
    },

    /// Generate a filelist for Source Insight
    Silist {
        #[arg(
            value_name = "PREFIX",
            help = "Path prefix for source files, such as '/home/user/repos/MX_MAIN' (for Linux) or 'F:/repos/MX_MAIN' (for Windows), etc."
        )]
        prefix: String,
    },

    /// Generate completion for the given shell
    #[command(after_help = format!(r#"{0}Note:{0:#}
  eval "$(rua init bash)"  # Append this line to ~/.bashrc
  eval "$(rua init zsh)"   # Append this line to ~/.zshrc"#, STYLE_YELLOW_BOLD))]
    Init {
        #[arg(value_name = "SHELL", help = "Shell type", value_enum)]
        shell: Shell,
    },
}

#[derive(Clone, Debug, Parser)]
#[command(
    name = "rua",
    author = "bzhao",
    version = "1.2.3",
    styles = STYLES,
    about = "A toolbox for developers of StoneOS and its derivatives",
    after_help = r#"Contact bzhao@hillstonenet.com if encountered bugs"#
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Comm,

    #[arg(short = 'd', long = "debug", help = "Enable debug option")]
    debug: bool,
}

pub(crate) fn run_app(args: &Cli) -> Result<()> {
    match args.command.clone() {
        Comm::Clean { dirs, ignores } => {
            let conf = RuaConf::new()?;
            let mut ignore_globset_builder = globset::GlobSetBuilder::new();
            ignore_globset_builder
                .add(globset::Glob::new(
                    format!("{}/*", PROJ_LEVEL_RUA_DIR.as_path().to_str().unwrap()).as_str(),
                )?)
                .add(globset::Glob::new(COMPDB_FILE.as_path().to_str().unwrap())?)
                .add(globset::Glob::new(
                    format!("{}/*", CLANGD_CACHE.as_path().to_str().unwrap()).as_str(),
                )?);

            if let Some(v) = ignores {
                for item in v {
                    ignore_globset_builder.add(
                        globset::Glob::new(item.as_str())
                            .context(format!("Failed to parse glob {}", item))?,
                    );
                }
            }
            if let Some(v) = conf.clean {
                if let Some(x) = v.ignores {
                    for item in x {
                        ignore_globset_builder.add(
                            globset::Glob::new(item.as_str())
                                .context(format!("Failed to parse glob {}", item))?,
                        );
                    }
                }
            }

            let ignore_globset = ignore_globset_builder
                .build()
                .context("Failed to build globset for ignores")?;
            clean::clean_build(dirs.as_ref(), &ignore_globset)
        }
        Comm::Compdb { compdb_comm } => {
            let conf = RuaConf::new()?;
            let rua_cache = COMPDB_STORE.as_path();
            if !rua_cache.is_file() {
                print!("The compilation database store does not exist, create it? [Y/n]: ");
                io::stdout().flush()?;
                let mut input_buf = String::new();
                io::stdin().read_line(&mut input_buf)?;
                let input = input_buf.trim();
                match input.trim().to_lowercase().as_str() {
                    "y" | "yes" | "" => {
                        fs::create_dir_all(".rua")?;
                    }
                    _ => return Ok(()),
                }
            }

            let conn = Connection::open(COMPDB_STORE.as_path())?;
            compdb::create_tables(&conn)?;

            match compdb_comm {
                CompdbCommand::Gen {
                    product_dir,
                    make_target,
                    defines,
                    engine,
                    bear_path,
                    intercept_build_path,
                } => {
                    let compdb_conf = conf.compdb;

                    // Get bear path from config or argument
                    let mut final_bear_path = None;
                    if let Some(v) = compdb_conf.as_ref() {
                        if let Some(x) = v.bear_path.as_ref() {
                            final_bear_path = Some(Path::new(x))
                        }
                    }
                    if let Some(v) = bear_path.as_ref() {
                        final_bear_path = Some(Path::new(v));
                    }

                    // Get intercept-build path from config or argument
                    let mut final_intercept_build_path = None;
                    if let Some(v) = compdb_conf.as_ref() {
                        if let Some(x) = v.intercept_build_path.as_ref() {
                            final_intercept_build_path = Some(Path::new(x));
                        }
                    }
                    if let Some(v) = intercept_build_path.as_ref() {
                        final_intercept_build_path = Some(Path::new(v));
                    }

                    let mut final_engine = None;
                    if let Some(v) = compdb_conf.as_ref() {
                        if let Some(x) = v.engine.as_ref() {
                            final_engine = match x.as_str() {
                                "built-in" => Some(CompdbEngine::BuiltIn),
                                "bear" => Some(CompdbEngine::Bear),
                                "intercept-build" => Some(CompdbEngine::InterceptBuild),
                                y => bail!("Invalid engine specified in config: {}", y),
                            };
                        }
                    }
                    if let Some(v) = engine {
                        final_engine = Some(v);
                    }

                    let svninfo = utils::SvnInfo::new()?;

                    // Add defines from config and cli
                    let mut defines_map: IndexMap<String, String> = IndexMap::new();
                    if let Some(configi) = compdb_conf.as_ref() {
                        if let Some(x) = configi.defines.as_ref() {
                            defines_map.extend(x.clone());
                        }
                    }
                    for item in defines.iter() {
                        if let Some((k, v)) = item.split_once("=") {
                            defines_map.insert(k.to_string(), v.to_string());
                        } else {
                            bail!("Invalid key-value pair: {}", item);
                        }
                    }

                    let compdb_options = compdb::CompdbOptions {
                        defines: defines_map,
                        engine: final_engine,
                        bear_path: final_bear_path.map(|x| x.to_path_buf()),
                        intercept_build_path: final_intercept_build_path.map(|x| x.to_path_buf()),
                    };
                    compdb::gen_compdb(&svninfo, &product_dir, &make_target, compdb_options)?;

                    // Archive the newly generated compilation database
                    let pb = ProgressBar::no_length().with_style(ProgressStyle::with_template(
                        "Archiving the newly generated compilation database to store...{msg}",
                    )?);
                    pb.tick();
                    let rows = compdb::archive_compdb(
                        &conn,
                        svninfo.branch_name(),
                        svninfo.revision(),
                        make_target.as_str(),
                        "compile_commands.json",
                    )?;
                    if rows == 0 {
                        eprintln!();
                        bail!(
                            "\rFailed to archive the newly generated compilation database to store"
                        );
                    }
                    pb.finish_with_message("ok");

                    // Get the generation id and insert it into the history table
                    if let Some(generation) = compdb::get_biggest_generation(&conn)? {
                        compdb::set_current_generation(&conn, generation)?;
                    }
                    Ok(())
                }
                CompdbCommand::Ls => compdb::list_generations(&conn),
                CompdbCommand::Use { generation } => {
                    compdb::use_generation(&conn, generation)?;
                    Ok(())
                }
                CompdbCommand::Del {
                    some,
                    old,
                    new,
                    all,
                } => {
                    let mut stderr_ = io::stderr();
                    if some.is_some() {
                        let generations = some.unwrap();
                        let generations_string = generations
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<String>>()
                            .join(" ");
                        let many = generations.len() > 1;
                        eprint!(
                            "Removing generation{} {}...",
                            if many { "s" } else { "" },
                            generations_string
                        );
                        stderr_.flush()?;
                        compdb::remove_generation(&conn, compdb::DelOpt::Generations(generations))?;
                        eprintln!(
                            "\rRemoving generation{} {}...ok",
                            if many { "s" } else { "" },
                            generations_string
                        );
                    } else if old.is_some() {
                        let n = old.unwrap();
                        eprint!(
                            "Removing {} oldest generation{}...",
                            n,
                            if n > 1 { "s" } else { "" }
                        );
                        stderr_.flush()?;
                        compdb::remove_generation(&conn, compdb::DelOpt::Oldest(n))?;
                        eprintln!(
                            "\rRemoving {} oldest generation{}...ok",
                            n,
                            if n > 1 { "s" } else { "" }
                        );
                    } else if new.is_some() {
                        let n = new.unwrap();
                        eprint!(
                            "Removing {} newest generation{}...",
                            n,
                            if n > 1 { "s" } else { "" }
                        );
                        stderr_.flush()?;
                        compdb::remove_generation(&conn, compdb::DelOpt::Newest(n))?;
                        eprintln!(
                            "\rRemoving {} newest generation{}...ok",
                            n,
                            if n > 1 { "s" } else { "" }
                        );
                    } else if all {
                        eprint!("Removing all generations...");
                        stderr_.flush()?;
                        compdb::remove_generation(&conn, compdb::DelOpt::All)?;
                        eprintln!("\rRemoving all generations...ok");
                    };
                    Ok(())
                }
                CompdbCommand::Add {
                    target,
                    revision,
                    compdb_path,
                } => {
                    let svninfo = utils::SvnInfo::new()?;
                    let compdb_path = compdb_path
                        .as_ref()
                        .map_or_else(|| COMPDB_FILE.as_path(), |x| Path::new(x.as_str()));
                    eprint!(
                        "Archiving compilation database for {} into store...",
                        target
                    );
                    io::stderr().flush()?;
                    let revision = revision.unwrap_or_else(|| svninfo.revision());
                    compdb::archive_compdb(
                        &conn,
                        svninfo.branch_name(),
                        revision,
                        target.as_str(),
                        compdb_path,
                    )?;
                    eprintln!(
                        "\rArchiving compilation database for {} into store...ok",
                        target
                    );
                    let file = Path::new(compdb_path);
                    let file_name = file.file_name();
                    let parent_dir = file.parent().unwrap();
                    let current_dir = env::current_dir().unwrap();
                    if file_name.is_some_and(|x| x == "compile_commands.json")
                        && parent_dir == current_dir
                    {
                        let generation = compdb::get_biggest_generation(&conn)?.unwrap();
                        compdb::set_current_generation(&conn, generation)?;
                    }
                    Ok(())
                }
                CompdbCommand::Name { generation, name } => {
                    eprint!(
                        "Naming compilation database generation {} {}...",
                        generation, name
                    );
                    io::stderr().flush()?;
                    let rows = compdb::name_generation(&conn, generation, name.as_str())?;
                    if rows == 0 {
                        eprintln!(
                            "\rNaming compilation database generation {} {}...err",
                            generation, name
                        );
                        bail!("No such generation");
                    }
                    eprintln!(
                        "\rNaming compilation database generation {} {}...ok",
                        generation, name
                    );
                    Ok(())
                }
                CompdbCommand::Remark { generation, remark } => {
                    eprint!(
                        "Remarking compilation database generation {}...",
                        generation
                    );
                    io::stderr().flush()?;
                    let rows = compdb::remark_generation(&conn, generation, remark.as_str())?;
                    if rows == 0 {
                        eprintln!(
                            "\rRemarking compilation database generation {}...",
                            generation
                        );
                        bail!("No such generation");
                    }
                    eprintln!(
                        "\rRemarking compilation database generation {}...ok",
                        generation
                    );
                    Ok(())
                }
            }
        }
        Comm::Showcc { comp_unit, comp_db } => {
            let compilation_db = match comp_db {
                Some(v) => PathBuf::from_str(v.as_str())?,
                None => PathBuf::from_str("compile_commands.json")?,
            };
            showcc::show_compile_command(comp_unit.as_str(), compilation_db.as_path())
        }
        Comm::Silist { prefix } => silist::gen_silist(&prefix),
        Comm::Mkinfo {
            ipv6,
            coverage,
            coverity,
            password,
            debug,
            webui,
            image_server,
            bins_without_strip,
            output_format,
            by_target,
            name: product_name_or_compile_target,
        } => {
            let conf = RuaConf::new()?;
            let mkinfo_conf = conf.mkinfo;

            let mut final_image_server = None;
            if let Some(ref v) = mkinfo_conf {
                if let Some(x) = v.image_server.as_deref() {
                    final_image_server = match x {
                        "beijing" | "bj" | "b" => Some(mkinfo::ImageServer::B),
                        "suzhou" | "sz" | "s" => Some(mkinfo::ImageServer::S),
                        other => {
                            eprintln!(
                                r#"WARNING: Invalid config item: image_server = {:?}! Falling back to "Suzhou" as image server"#,
                                other
                            );
                            Some(mkinfo::ImageServer::S)
                        }
                    };
                }
            }
            if let Some(v) = image_server {
                final_image_server = Some(v);
            }

            let mut define_map = IndexMap::new();
            if let Some(ref v) = mkinfo_conf {
                if let Some(x) = v.defines.as_ref() {
                    for (key, val) in x.clone() {
                        define_map.insert(key, val);
                    }
                }
            }

            let mut makeflag = mkinfo::MakeFlag::empty();
            if !debug {
                makeflag |= mkinfo::MakeFlag::RELEASE;
            };
            if ipv6 {
                makeflag |= mkinfo::MakeFlag::IPV6;
            }
            if webui {
                makeflag |= mkinfo::MakeFlag::WEBUI;
            }
            if password {
                makeflag |= mkinfo::MakeFlag::SHELL_PASSWORD;
            }
            if coverage {
                makeflag |= mkinfo::MakeFlag::COVERAGE;
            }
            if coverity {
                makeflag |= mkinfo::MakeFlag::COVERITY;
            }

            let makeopts = MakeOpts {
                flag: makeflag,
                image_server: final_image_server,
                nostrip_bins: bins_without_strip,
                defines: define_map,
            };

            let mkinfos = mkinfo::gen_mkinfo(
                if by_target {
                    GenBy::Target(product_name_or_compile_target)
                } else {
                    GenBy::Nickname(product_name_or_compile_target)
                },
                makeopts,
            )?;

            mkinfo::dump_mkinfo(&mkinfos, output_format)
        }
        Comm::Perfan {
            file,
            elfs,
            format: outfmt,
        } => {
            let data = perfan::proc_perfanno(&file, elfs.iter().collect::<Vec<&PathBuf>>())?;
            perfan::dump_perfdata(&data, outfmt)
        }
        Comm::Review {
            bug_id,
            review_id,
            files,
            diff_file,
            reviewers,
            branch_name,
            repo_name,
            revisions,
            template_file,
        } => {
            let conf = RuaConf::new()?;
            let mut final_template_file = None;
            if let Some(review_conf) = conf.review.as_ref() {
                if let Some(v) = review_conf.template_file.as_ref() {
                    final_template_file = Some(v.to_owned());
                }
            }
            if let Some(v) = template_file.as_deref() {
                final_template_file = Some(v.to_string());
            }

            let options = review::ReviewOptions {
                bug_id,
                review_id,
                files,
                diff_file,
                reviewers,
                branch_name,
                repo_name,
                revisions,
                template_file: final_template_file,
            };
            tokio::runtime::Runtime::new()?.block_on(review::review(&options))
        }
        Comm::Init { shell } => {
            shinit::gen_completion(&mut Cli::command(), shell);
            Ok(())
        }
    }
}
