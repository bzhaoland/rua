use std::path::PathBuf;
use std::str::FromStr;

use anstyle::{AnsiColor, Color, Style};
use anyhow::{bail, Result};
use clap::builder::styling;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

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

#[derive(Clone, Debug, Parser)]
#[command(
    name = "rua",
    author = "bzhao",
    version = "0.19.1",
    styles = STYLES,
    about = "Devbox for StoneOS project",
    long_about = "Devbox for StoneOS project",
    after_help = r#"Contact bzhao@hillstonenet.com if encountered bugs."#
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Comm,

    #[arg(short = 'd', long = "debug", help = "Enable debug option")]
    debug: bool,
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

    /// Generate JSON compilation database (JCDB) for a specific target.
    ///
    /// You may run this command at either project root directory or submodule
    /// directory.
    /// However, you may have to compile the target completely first before
    /// running at submodule directory.
    #[command(after_help = format!(
        r#"{}Examples:{:#}
  rua compdb products/ngfw_as a-dnv           # For A1000/A2000...
  rua compdb products/ngfw_as a-dnv-ipv6      # For A1000/A2000... with IPv6 support
  rua compdb . a-dnv                          # For A1000/A2000... at submodule directory
  rua compdb --engine=bear . a-dnv            # For A1000/A2000... using bear at submodule directory
  run compdb --engine=intercept-build . a-dnv # For A1000/A2000... using intercept-build at submodule directory

{}Caution:{:#}
  Several files will be hacked while running with default engine (built-in):
  1. When running at project root dir:
     scripts/last-rules.mk
     scripts/rules.mk
     Makefile
  2. When running at submodule dir:
     scripts/last-rules.mk
     scripts/rules.mk
  These files may be left dirty if compdb aborted unexpectedly. You could use
  the following command to manually restore them:
  {}svn revert Makefile scripts/last-rules.mk scripts/rules.mk{:#}"#,
      STYLE_YELLOW,
      STYLE_YELLOW,
      STYLE_RED,
      STYLE_RED,
      STYLE_YELLOW,
      STYLE_YELLOW))]
    Compdb {
        #[arg(
            value_name = "PATH",
            help = "Make path for the target, such as 'products/vfw'"
        )]
        product_dir: String,

        #[arg(value_name = "TARGET", help = "Target to make, such as 'aws'")]
        make_target: String,

        #[arg(
            short = 'e',
            long = "engine",
            value_name = "ENGINE",
            help = "Engine used to generate compilation database"
        )]
        engine: Option<CompdbEngine>,

        #[arg(
            short = 'b',
            long = "bear-path",
            value_name = "BEAR",
            help = "Path to bear (defaults to /devel/sw/bear/bin/bear)"
        )]
        bear_path: Option<String>,

        #[arg(
            short = 'i',
            long = "intercept-build-path",
            value_name = "INTERCEPT-BUILD",
            help = "Path to intercept-build (defaults to /devel/sw/llvm/bin/intercept-build)"
        )]
        intercept_build_path: Option<String>,
    },

    /// Get all matched makeinfos for product
    #[command(after_help = format!(r#"{STYLE_YELLOW}Examples:{STYLE_YELLOW:#}
  rua mkinfo A1000      # Makeinfo for A1000 without extra features
  rua mkinfo -6 A1000   # Makeinfo for A1000 with IPv6 enabled
  rua mkinfo -6w 'X\d+' # Makeinfos for X-series products with IPv6 and WebUI enabled using regex pattern"#))]
    Mkinfo {
        /// Build with only IPv4 enabled
        #[arg(
            short = '4',
            long = "ipv4",
            default_value_t = true,
        )]
        ipv4: bool,

        /// Build with IPv6 enabled
        #[arg(
            short = '6',
            long = "ipv6",
            default_value_t = false,
            conflicts_with = "ipv4",
        )]
        ipv6: bool,

        /// Run coverage
        #[arg(
            short = 'g',
            long = "coverage",
            default_value_t = false,
        )]
        coverage: bool,

        /// Run coverity
        #[arg(
            short = 'c',
            long = "coverity",
            default_value_t = false,
        )]
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
            value_name = "REVIEW-ID",
            short = 'r',
            long = "review-id",
            help = "Existing review id"
        )]
        review_id: Option<u32>,

        #[arg(value_name = "FILE", help = "Files to be reviewed")]
        files: Option<Vec<String>>,

        #[arg(
            value_name = "DIFF-FILE",
            short = 'd',
            long = "diff-file",
            help = "Diff file to be used"
        )]
        diff_file: Option<String>,

        #[arg(
            value_name = "REVIEWERS",
            short = 'u',
            long = "reviewers",
            help = "Reviewers"
        )]
        reviewers: Option<Vec<String>>,

        #[arg(
            value_name = "BRANCH",
            short = 'b',
            long = "branch",
            help = "Branch name for this commit"
        )]
        branch_name: Option<String>,

        #[arg(
            value_name = "REPO",
            short = 'p',
            long = "repo",
            help = "Repository name"
        )]
        repo_name: Option<String>,

        #[arg(
            value_name = "REVISION",
            short = 's',
            long = "revision",
            help = "Revision to be used"
        )]
        revisions: Option<String>,

        #[arg(
            value_name = "FILE",
            short = 't',
            long = "template-file",
            help = "Use customized template file (please ensure it can run through svn commit hooks)"
        )]
        template_file: Option<String>,
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
        Comm::Compdb {
            product_dir,
            make_target,
            mut engine,
            mut intercept_build_path,
            mut bear_path,
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

            let compdb_options = compdb::CompdbOptions {
                engine,
                bear_path,
                intercept_build_path,
            };
            compdb::gen_compdb(&product_dir, &make_target, compdb_options)
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
