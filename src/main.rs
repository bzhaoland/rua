mod submods;
mod utils;

use std::path::PathBuf;

use anstyle::AnsiColor;
use clap::builder::styling;
use clap::{Parser, Subcommand};

use crate::submods::clean;
use crate::submods::compdb;
use crate::submods::mkinfo::{self, MakeFlag};
use crate::submods::perfan;
use crate::submods::review;
use crate::submods::showcc;
use crate::submods::silist;

const STYLES: styling::Styles = styling::Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(
    name = "rua",
    author = "bzhao",
    version = "0.11.0",
    styles = STYLES,
    about = "Devbox for StoneOS project",
    long_about = "Devbox for StoneOS project",
    after_help = r#"Contact bzhao when encountering bugs. "#
)]
struct Cli {
    #[command(subcommand)]
    command: Comm,
}

#[derive(Subcommand)]
enum Comm {
    /// Clean build files (run under project root)
    Clean,

    /// Generate JSON Compilation Database for a specific target, such as a-dnv/a-dnv-ipv6
    #[command(after_help = r#"Examples:
  rua compdb products/ngfw_as a-dnv       # For A1000/A1100/A2000...
  rua compdb products/ngfw_as a-dnv-ipv6  # For A1000/A1100/A2000... with IPv6 enabled
  rua compdb products/ngfw_as kunlun-ipv6 # For X20803/X20812... with IPv6 enabled"#)]
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
    #[command(after_help = r#"Examples:
  rua mkinfo A1000
  rua mkinfo -6 A1000 # Dual-stack
  rua mkinfo X10800
  rua mkinfo X20812
  rua mkinfo 'X\d+' # Regex pattern for X6180/X7180/X8180..."#)]
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

        /// Output format
        #[arg(long = "outfmt", default_value = "list", value_name = "OUTPUT-FORMAT")]
        outfmt: mkinfo::DumpFormat,

        /// Build with password
        #[arg(short = 'p', long = "password", default_value_t = false)]
        password: bool,

        /// Build with WebUI
        #[arg(short = 'w', long = "webui")]
        webui: bool,

        /// Product name, such as 'A1000'. Regex is also supported, e.g. 'X\d+80'
        #[arg(value_name = "PRODUCT-NAME")]
        prodname: String,
    },

    /// Extensively translate addresses to file and locations
    Perfan {
        #[arg(help = "Annotated file to be processed (perf)", value_name = "FILE")]
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

    /// Initiate a new review request or refresh an existing one
    Review {
        #[arg(
            value_name = "BUG ID",
            help = "The bug id used for this review request"
        )]
        bug_id: u32,
        #[arg(
            value_name = "REVIEW ID",
            short = 'r',
            long = "review-id",
            help = "The review id of an existing review request"
        )]
        review_id: Option<u32>,
        #[arg(
            value_name = "FILES",
            short = 'f',
            long = "files",
            help = "Files to be reviewed"
        )]
        files: Option<Vec<String>>,
        #[arg(
            value_name = "DIFF FILE",
            short = 'd',
            long = "diff-file",
            help = "Diff files to be uploaded"
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
            value_name = "REPOSITORY NAME",
            short = 'p',
            long = "repo-name",
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
        #[arg(value_name = "FILENAME", help = "Fetch compile command of which file")]
        filename: String,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Suppress the following error info:
    // failed printing to stdout: Broken pipe
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = Cli::parse();

    match args.command {
        Comm::Clean => clean::clean_build(),
        Comm::Compdb {
            product_dir,
            make_target,
        } => compdb::gen_compdb(&product_dir, &make_target),
        Comm::Showcc { filename } => {
            let records = showcc::fetch_compile_command(filename.as_str())?;
            showcc::print_records(&records)?;
            Ok(())
        }
        Comm::Silist { prefix } => silist::gen_silist(&prefix),
        Comm::Mkinfo {
            coverity,
            ipv4: _,
            ipv6,
            password,
            prodname,
            debug,
            webui,
            outfmt,
        } => {
            let mut makeflag = mkinfo::MakeFlag::empty();
            if !debug {
                makeflag |= MakeFlag::R_BUILD;
            };
            if ipv6 {
                makeflag |= MakeFlag::INET_V6;
            }
            if webui {
                makeflag |= MakeFlag::WITH_UI;
            }
            if password {
                makeflag |= MakeFlag::WITH_PW;
            }
            if coverity {
                makeflag |= MakeFlag::COVERITY;
            }

            let printinfos = mkinfo::gen_mkinfo(&prodname, makeflag)?;

            mkinfo::dump_mkinfo(&printinfos, outfmt)
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
            review::review(&options).await
        }
    }
}
