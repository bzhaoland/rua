use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fs, io};

use anstyle::{AnsiColor, Color, Style};
use anyhow::{bail, Context, Result};
use clap::builder::styling;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use indexmap::IndexMap;
use rusqlite::Connection;

use crate::config::RuaConf;
use crate::submods::compdb::{self, CompdbEngine};
use crate::submods::mkinfo::{self, MakeOpts};
use crate::submods::perfan;
use crate::submods::review;
use crate::submods::showcc;
use crate::submods::silist;
use crate::submods::{clean, initsh};

const STYLE_YELLOW: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
    .bold();
const STYLE_GREEN: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Green)))
    .bold();
const STYLE_CYAN: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Cyan)))
    .bold();
const STYLE_RED: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Red)))
    .bold();
const STYLES: styling::Styles = styling::Styles::styled()
    .header(STYLE_YELLOW)
    .usage(STYLE_YELLOW)
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
    /// Note that compilation database generated under submod dir only covers
    /// files in this module.
    #[command(after_help = format!(
        r#"{}Examples:{:#}
  rua compdb gen products/ngfw_as a-dnv                    # For A1000/A2000...
  rua compdb gen products/ngfw_as a-dnv-ipv6               # For A1000/A2000... with IPv6 support
  rua compdb gen -e intercept-build products/ngfw_as a-dnv # For A1000/A2000... using intercept-build
  rua compdb gen . a-dnv                                   # For A1000/A2000... under submod dir
  rua compdb gen -e bear . a-dnv                           # For A1000/A2000... under submod dir using bear 
  run compdb gen -e intercept-build . a-dnv                # For A1000/A2000... under submod dir using intercept-build

{}Caution:{:#}
  Some files are modified while running in built-in mode which is the default
  and faster:
  1. When running under project root dir:
     - scripts/last-rules.mk
     - scripts/rules.mk
     - Makefile
  2. When running under submod dir:
     - scripts/last-rules.mk
     - scripts/rules.mk
  These files may be left dirty if compdb process aborted unexpectedly. You
  could manually restore them by execute:
  {}svn revert Makefile scripts/last-rules.mk scripts/rules.mk{:#}"#,
      STYLE_YELLOW,
      STYLE_YELLOW,
      STYLE_RED,
      STYLE_RED,
      STYLE_YELLOW,
      STYLE_YELLOW))]
    Gen {
        #[arg(
            value_name = "PATH",
            help = "Path for the target where platform-specific makefiles reside, such as 'products/vfw'"
        )]
        product_dir: String,

        #[arg(value_name = "TARGET", help = "Target to build, such as 'a-dnv'")]
        make_target: String,

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
    },

    /// Add the currently used compilation database into store as a new generation
    Add {
        #[arg(value_name = "TARGET", help = "Target for the compilation database")]
        target: String,

        #[arg(
            short = 'f',
            long = "compilation-database",
            value_name = "COMPILATION-DATABASE",
            help = "Use this compilation database other than the default (compile_commands.json)"
        )]
        compdb_path: Option<String>,
    },

    /// Delete compilation database generation(s) from store
    #[command(visible_alias = "rm")]
    Del {
        #[arg(
            value_name = "GENERATION-ID",
            help = "Generation to delete",
            conflicts_with = "all"
        )]
        generation: Option<i64>,

        #[arg(short = 'a', long = "all", help = "Remove all generations")]
        all: bool,
    },

    /// List all compilation database generations in store
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
            help = "Remark for the compilation database generation"
        )]
        remark: String,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum Comm {
    /// Clean build files (run under project root)
    #[command(after_help = format!("{STYLE_YELLOW}Examples:{STYLE_YELLOW:#}
  rua clean  # Clean the entire project"))]
    Clean {
        #[arg(
            value_name = "ENTRY",
            help = "Files or directories to be cleaned ('target' is always included even if not specified)"
        )]
        dirs: Option<Vec<String>>,

        #[arg(
            short = 'n',
            long = "ignores",
            value_name = "IGNORES",
            help = "List of files and directories seperated by commas to be ignored"
        )]
        ignores: Option<Vec<String>>,
    },

    /// Manipulate compilation database.
    Compdb {
        #[clap(subcommand)]
        compdb_comm: CompdbCommand,
    },

    /// Get all matched makeinfos for product
    #[command(after_help = format!(r#"{STYLE_YELLOW}Examples:{STYLE_YELLOW:#}
  rua mkinfo A1000      # Makeinfo for A1000 without extra features
  rua mkinfo -6 A1000   # Makeinfo for A1000 with IPv6 enabled
  rua mkinfo -6w 'X\d+' # Makeinfos for X-series products with IPv6 and WebUI enabled using regex pattern"#))]
    Mkinfo {
        /// Build with only IPv4 enabled
        #[arg(short = '4', long = "ipv4", default_value_t = true)]
        ipv4: bool,

        /// Build with IPv6 enabled
        #[arg(
            short = '6',
            long = "ipv6",
            default_value_t = false,
            conflicts_with = "ipv4"
        )]
        ipv6: bool,

        /// Run coverage
        #[arg(short = 'g', long = "coverage", default_value_t = false)]
        coverage: bool,

        /// Run coverity
        #[arg(short = 'c', long = "coverity", default_value_t = false)]
        coverity: bool,

        /// Build in debug mode (default is release mode)
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

        /// Product name, such as 'A1000'. Regex is also supported, e.g. 'X\d+80'
        #[arg(value_name = "PRODUCT")]
        product_name: String,
    },

    /// Extensively map instructions to file locations (inline expanded)
    Perfan {
        #[arg(help = "File to process (perf annotate output)", value_name = "FILE")]
        file: PathBuf,

        #[arg(
            short = 'd',
            long = "daemon",
            value_name = "DAEMON",
            help = "Only resolve addresses owned by this daemon"
        )]
        daemon: String,

        #[arg(
            short = 'o',
            long = "outfmt",
            value_name = "OUTFMT",
            default_value = "table",
            help = "Output format"
        )]
        outfmt: perfan::DumpFormat,

        #[arg(
            short = 'b',
            long = "bin",
            value_name = "BIN",
            help = "The binary file used to resolve the addresses"
        )]
        bin: PathBuf,
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
    #[command(after_help = format!(r#"{STYLE_YELLOW}Note:{STYLE_YELLOW:#}
  eval "$(rua init bash)"  # Append this line to ~/.bashrc
  eval "$(rua init zsh)"   # Append this line to ~/.zshrc"#))]
    Init {
        #[arg(value_name = "SHELL", help = "Shell type", value_enum)]
        shell: Shell,
    },
}

#[derive(Clone, Debug, Parser)]
#[command(
    name = "rua",
    author = "bzhao",
    version = "0.22.0",
    styles = STYLES,
    about = "A toolbox for StoneOS project",
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
            let conf = RuaConf::load()?;
            let ignores = if ignores.is_some() {
                ignores.as_ref()
            } else if let Some(conf) = conf.as_ref() {
                if let Some(v) = conf.clean.as_ref() {
                    v.ignores.as_ref()
                } else {
                    None
                }
            } else {
                None
            };
            clean::clean_build(dirs.as_ref(), ignores)
        }
        Comm::Compdb { compdb_comm } => {
            let rua_cache = Path::new(compdb::COMPDB_STORE);
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

            let conn = Connection::open(compdb::COMPDB_STORE)?;
            compdb::create_compdbs_table(&conn)?;

            match compdb_comm {
                CompdbCommand::Gen {
                    product_dir,
                    make_target,
                    defines,
                    mut engine,
                    mut bear_path,
                    mut intercept_build_path,
                } => {
                    let conf = RuaConf::load()?;
                    if bear_path.is_none() {
                        if let Some(rua_conf) = conf.as_ref() {
                            if let Some(compdb_conf) = rua_conf.compdb.as_ref() {
                                if let Some(v) = compdb_conf.bear_path.as_ref() {
                                    bear_path = Some(v.to_owned());
                                }
                            }
                        }
                    }
                    if intercept_build_path.is_none() {
                        if let Some(rua_conf) = conf.as_ref() {
                            if let Some(compdb_conf) = rua_conf.compdb.as_ref() {
                                if let Some(v) = compdb_conf.intercept_build_path.as_ref() {
                                    intercept_build_path = Some(v.to_owned());
                                }
                            }
                        }
                    }

                    if engine.is_none() {
                        if let Some(rua_conf) = conf.as_ref() {
                            if let Some(compdb_conf) = rua_conf.compdb.as_ref() {
                                if let Some(engine_key) = compdb_conf.engine.as_ref() {
                                    engine = match engine_key.as_str() {
                                        "built-in" => Some(CompdbEngine::BuiltIn),
                                        "bear" => Some(CompdbEngine::Bear),
                                        "intercept-build" => Some(CompdbEngine::InterceptBuild),
                                        _ => bail!("Invalid config: engine = {}", engine_key),
                                    };
                                }
                            }
                        }
                    }

                    let mut defines_map: IndexMap<String, String> = IndexMap::new();
                    // Add defines provided as command line arguments
                    for item in defines.iter() {
                        if let Some((k, v)) = item.split_once("=") {
                            defines_map.insert(k.to_owned(), v.to_owned());
                        } else {
                            bail!("Invalid key-value pair: {}", item);
                        }
                    }
                    // Add defines defined in configuration file
                    if let Some(rua_conf) = conf.as_ref() {
                        if let Some(compdb_conf) = rua_conf.compdb.as_ref() {
                            if let Some(defines_conf) = compdb_conf.defines.as_ref() {
                                for (k, v) in defines_conf {
                                    defines_map.insert(k.to_owned(), v.to_owned());
                                }
                            }
                        }
                    }

                    let compdb_options = compdb::CompdbOptions {
                        defines: defines_map,
                        engine,
                        bear_path,
                        intercept_build_path,
                    };
                    compdb::gen_compdb(&product_dir, &make_target, compdb_options)?;

                    // Archive the newly generated compilation database
                    eprint!("Adding the newly generated compilation database to store...");
                    io::stderr().flush()?;
                    let rows =
                        compdb::ark_compdb(&conn, make_target.as_str(), "compile_commands.json")?;
                    if rows == 0 {
                        eprintln!(
                            "\rAdding the newly generated compilation database to store...err"
                        );
                        bail!("Add compilation database into store failed");
                    }
                    eprintln!("\rAdding the newly generated compilation database to store...ok");
                    Ok(())
                }
                CompdbCommand::Ls => compdb::list_compdbs(&conn),
                CompdbCommand::Use { generation } => {
                    eprint!("Switching to generation {}...", generation);
                    io::stderr().flush()?;
                    compdb::use_compdb(&conn, generation)?;
                    eprintln!("\rSwitching to generation {}...ok", generation);
                    Ok(())
                }
                CompdbCommand::Del { generation, all } => {
                    if all {
                        eprint!("Deleting all generations...");
                        io::stderr().flush()?;
                        compdb::del_compdb(&conn, 0)?;
                        eprintln!("\rDeleting all generations...ok");
                    } else {
                        let generation = generation
                            .context("Neither <GENERATION> nor --all option is specified")?;
                        eprint!("Deleting generation {}...", generation);
                        io::stderr().flush()?;
                        compdb::del_compdb(&conn, generation)?;
                        eprintln!("\rDeleting generation {}...ok", generation);
                    };
                    Ok(())
                }
                CompdbCommand::Add {
                    target,
                    compdb_path,
                } => {
                    let compdb_path = compdb_path
                        .as_ref()
                        .map_or_else(|| compdb::COMPDB_FILE, |x| x.as_str());
                    eprint!(
                        "Archiving compilation database ({}) into store as a new generation...",
                        compdb_path
                    );
                    io::stderr().flush()?;
                    compdb::ark_compdb(&conn, target.as_str(), compdb_path)?;
                    eprintln!(
                        "\rArchiving compilation database ({}) into store as a new generation...ok",
                        compdb_path
                    );
                    Ok(())
                }
                CompdbCommand::Name { generation, name } => {
                    eprint!(
                        "Naming compilation database generation {} {}...",
                        generation, name
                    );
                    io::stderr().flush()?;
                    let rows = compdb::name_compdb(&conn, generation, name.as_str())?;
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
                    let rows = compdb::remark_compdb(&conn, generation, remark.as_str())?;
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
            ipv4: _,
            ipv6,
            coverage,
            coverity,
            password,
            product_name,
            debug,
            webui,
            image_server,
            output_format,
        } => {
            let conf = RuaConf::load()?;
            let image_server = if let Some(image_server) = image_server {
                Some(image_server)
            } else if let Some(conf) = conf.as_ref() {
                if let Some(mkinfo_conf) = conf.mkinfo.as_ref() {
                    if let Some(v) = mkinfo_conf.image_server.as_ref() {
                        match v.to_lowercase().as_str() {
                            "beijing" | "bj" | "b" => Some(mkinfo::ImageServer::B),
                            "suzhou" | "sz" | "s" => Some(mkinfo::ImageServer::S),
                            other => {
                                eprintln!(
                                    r#"WARNING: Invalid config item: image_server = {:?}! Falling back to "Suzhou" as image server"#,
                                    other
                                );
                                Some(mkinfo::ImageServer::S)
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

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
            let printinfos = mkinfo::gen_mkinfo(
                &product_name,
                MakeOpts {
                    flag: makeflag,
                    image_server,
                },
            )?;

            mkinfo::dump_mkinfo(&printinfos, output_format)
        }
        Comm::Perfan {
            file,
            daemon,
            bin,
            outfmt,
        } => {
            let data = perfan::proc_perfanno(&file, &bin, &daemon)?;
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
            let conf = RuaConf::load()?;
            let template_file = if let Some(v) = template_file {
                Some(v)
            } else if let Some(conf) = conf.as_ref() {
                if let Some(v) = conf.review.as_ref() {
                    v.template_file.clone()
                } else {
                    None
                }
            } else {
                None
            };
            let options = review::ReviewOptions {
                bug_id,
                review_id,
                files,
                diff_file,
                reviewers,
                branch_name,
                repo_name,
                revisions,
                template_file,
            };
            tokio::runtime::Runtime::new()?.block_on(review::review(&options))
        }
        Comm::Init { shell } => {
            initsh::gen_completion(&mut Cli::command(), shell);
            Ok(())
        }
    }
}
