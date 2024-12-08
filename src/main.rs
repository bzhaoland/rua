mod submods;
mod utils;

use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;

use anstyle::{Ansi256Color, Color, Style};
use clap::builder::styling;
use clap::{Parser, Subcommand};
use url::Url;

use crate::submods::clean;
use crate::submods::compdb;
use crate::submods::mkinfo;
use crate::submods::perfan;
use crate::submods::review;
use crate::submods::showcc;
use crate::submods::silist;

const CLAP_STYLE_HEADER: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(3))))
    .bold();
const CLAP_STYLE_USAGE: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(3))))
    .bold();
const CLAP_STYLE_LITERAL: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(2))))
    .bold();
const CLAP_STYLE_PLACEHOLDER: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(6))))
    .bold();
const CLAP_STYLE_CAUTION: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(1))))
    .bold();
const STYLE_YELLOW: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(3))));
const STYLES: styling::Styles = styling::Styles::styled()
    .header(CLAP_STYLE_HEADER)
    .usage(CLAP_STYLE_USAGE)
    .literal(CLAP_STYLE_LITERAL)
    .placeholder(CLAP_STYLE_PLACEHOLDER);

#[derive(Parser)]
#[command(
    name = "rua",
    author = "bzhao",
    version = "0.16.0",
    styles = STYLES,
    about = "Devbox for StoneOS project",
    long_about = "Devbox for StoneOS project",
    after_help = r#"Contact bzhao@hillstonenet.com if encountered bugs. "#
)]
struct Cli {
    #[command(subcommand)]
    command: Comm,

    #[arg(short = 'd', long = "debug", help = "Enable debug option")]
    debug: bool,
}

#[derive(Subcommand)]
enum Comm {
    /// Clean build files (run under project root)
    #[command(after_help = format!("{CLAP_STYLE_HEADER}Examples:{CLAP_STYLE_HEADER:#}
  rua clean  # Clean the entire project"))]
    Clean {
        #[arg(
            value_name = "ENTRY",
            help = "Files or directories to be cleaned ('target' is always included even if not specified)"
        )]
        dirs: Option<Vec<OsString>>,

        #[arg(
            short = 'n',
            long = "ignores",
            value_name = "IGNORES",
            help = "List of files and directories seperated by commas to be ignored"
        )]
        ignores: Option<Vec<OsString>>,
    },

    /// Generate JSON compilation database (JCDB) for a specific target
    #[command(after_help = format!(
        r#"{}Examples:{:#}
  rua compdb products/ngfw_as a-dnv        # For A1000/A1100/A2000...
  rua compdb products/ngfw_as a-dnv-ipv6   # For A1000/A1100/A2000... with IPv6 enabled
  rua compdb products/ngfw_as kunlun-ipv6  # For X20803/X20812... with IPv6 enabled

{}Caution:{:#}
  Three files ("scripts/last-rules.mk", "scripts/rules.mk" and "Makefile") are
  hacked while running, and would be left in hacked state if the command aborts
  unexpectedly. Use the following command to manually restore them:
  {}svn revert Makefile scripts/last-rules.mk scripts/rules.mk{:#}"#,
      CLAP_STYLE_HEADER,
      CLAP_STYLE_HEADER,
      CLAP_STYLE_CAUTION,
      CLAP_STYLE_CAUTION,
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
    },

    /// Get all matched makeinfos for product
    #[command(after_help = format!(r#"{CLAP_STYLE_HEADER}Examples:{CLAP_STYLE_HEADER:#}
  rua mkinfo A1000      # Makeinfo for A1000 without extra features
  rua mkinfo -6 A1000   # Makeinfo for A1000 with IPv6 enabled
  rua mkinfo -6w 'X\d+' # Makeinfos for X-series products with IPv6 and WebUI enabled using regex pattern"#))]
    Mkinfo {
        #[arg(
            short = '4',
            long = "ipv4",
            default_value_t = true,
            conflicts_with = "ipv6",
            help = r"Build with IPv4 only"
        )]
        ipv4: bool,

        #[arg(
            short = '6',
            long = "ipv6",
            default_value_t = false,
            conflicts_with = "ipv4",
            help = r"Build with IPv4 & IPv6"
        )]
        ipv6: bool,

        #[arg(
            short = 'c',
            long = "coverity",
            default_value_t = false,
            help = r"Build with coverity check"
        )]
        coverity: bool,

        /// Build in debug mode (default is release mode)
        #[arg(short = 'd', long = "debug", default_value_t = false)]
        debug: bool,

        /// Output format for makeinfos
        #[arg(long = "format", default_value = "list", value_name = "FORMAT")]
        output_format: mkinfo::DumpFormat,

        /// Build with password
        #[arg(short = 'p', long = "password", default_value_t = false)]
        password: bool,

        /// Build with WebUI
        #[arg(short = 'w', long = "webui")]
        webui: bool,

        /// Server to upload the output image to
        #[arg(short = 's', long = "image-server", value_name = "IMAGE-SERVER-IP")]
        image_server: Option<String>,

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
            help = "Bug id for this review request"
        )]
        bug_id: u32,

        #[arg(
            value_name = "REVIEW",
            short = 'r',
            long = "review",
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
    },

    /// Show all possible compile commands for filename (based on compilation database)
    Showcc {
        #[arg(
            value_name = "SOURCE-FILE",
            help = "Source file name for which to fetch all the available compile commands"
        )]
        compilation_unit: String,
        #[arg(
            value_name = "COMPDB",
            short = 'c',
            long = "compdb",
            help = r#"Compilation database (defaults to file "compile_commands.json" in the current directory)"#
        )]
        compilation_db: Option<String>,
    },

    /// Generate a filelist for Source Insight
    Silist {
        #[arg(
            value_name = "PREFIX",
            help = "Path prefix for source files, such as '/home/user/repos/MX_MAIN' (for Linux) or 'F:/repos/MX_MAIN' (for Windows), etc."
        )]
        prefix: String,
    },
}

fn main() -> anyhow::Result<()> {
    // Suppress the following error info:
    // failed printing to stdout: Broken pipe
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = Cli::parse();

    match args.command {
        Comm::Clean { dirs, ignores } => clean::clean_build(dirs, ignores.as_ref()),
        Comm::Compdb {
            product_dir,
            make_target,
        } => compdb::gen_compdb(&product_dir, &make_target),
        Comm::Showcc {
            compilation_unit,
            compilation_db,
        } => {
            let compilation_db = match compilation_db {
                Some(v) => PathBuf::from_str(v.as_str())?,
                None => PathBuf::from_str("compile_commands.json")?,
            };
            let records =
                showcc::fetch_compile_command(compilation_unit.as_str(), compilation_db.as_path())?;
            showcc::print_records(&records)?;
            Ok(())
        }
        Comm::Silist { prefix } => silist::gen_silist(&prefix),
        Comm::Mkinfo {
            coverity,
            ipv4: _,
            ipv6,
            password,
            product_name,
            debug,
            webui,
            image_server,
            output_format,
        } => {
            let mut makeflag = mkinfo::MakeFlag::empty();
            if !debug {
                makeflag |= mkinfo::MakeFlag::RELEASE_BUILD;
            };
            if ipv6 {
                makeflag |= mkinfo::MakeFlag::ENABLE_IPV6;
            }
            if webui {
                makeflag |= mkinfo::MakeFlag::ENABLE_WEBUI;
            }
            if password {
                makeflag |= mkinfo::MakeFlag::ENABLE_SHELL_PASSWORD;
            }
            if coverity {
                makeflag |= mkinfo::MakeFlag::ENABLE_COVERITY;
            }
            if image_server.is_some() {
                let url_str = image_server.as_deref().unwrap();
                assert!(
                    Url::parse(url_str).is_ok(),
                    "Invalid URL specified as image server"
                );
            }
            let printinfos = mkinfo::gen_mkinfo(&product_name, makeflag, image_server.as_deref())?;

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
        } => {
            let options = review::ReviewOptions {
                bug_id,
                review_id,
                files,
                diff_file,
                reviewers,
                branch_name,
                repo_name,
                revisions,
            };
            tokio::runtime::Runtime::new()?.block_on(review::review(&options))
        }
    }
}
