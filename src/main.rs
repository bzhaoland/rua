mod submods;
mod utils;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use submods::clean::clean_build;
use submods::compdb::gen_compdb;
use submods::getcc::fetch_compile_command;
use submods::mkinfo::{self, BuildMode, InetVer, MakeOpt};
use submods::profile::{self, dump_perfdata, proc_perfanno};
use submods::review;
use submods::silist::gen_silist;

#[derive(Parser)]
#[command(
    name = "rua",
    author = "bzhao",
    version = "0.8.1",
    about = "A sweety box for StoneOS devel.",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Comm,
}

#[derive(Subcommand)]
enum Comm {
    /// Clean build files (run under project root)
    Clean,

    /// Generate a JSON Compilation Database for the target
    Compdb {
        #[arg(
            value_name = "PATH",
            help = "Make path for the target, such as 'products/vfw'"
        )]
        product_dir: String,
        #[arg(value_name = "TARGET", help = "Target to make, such as 'aws'")]
        make_target: String,
    },

    /// Show the compile command for a specific file (depends on compilation database)
    Showcc {
        #[arg(value_name = "FILENAME", help = "Fetch compile command of which file")]
        filename: String
    },

    /// Generate a filelist only consisting of c/c++ source files
    Silist {
        #[arg(
            value_name = "PREFIX",
            help = "Path prefix for source files, such as '/home/user/repos/MX_MAIN' (for Linux) or 'F:/repos/MX_MAIN' (for Windows), etc."
        )]
        prefix: String,
    },

    /// Get all matched makeinfos for the product
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

        /// Build in debug mode (default is release)
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

    /// Enhance the given perf-annotated text extensively with address translation
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
        outfmt: profile::DumpFormat,

        #[arg(
            short = 'b',
            long = "bin",
            value_name = "BIN",
            help = "The binary file used to resolve the addresses"
        )]
        bin: PathBuf,
    },
    /// Launch a new review request or reuse an existing one
    Review {
        #[arg(
            value_name = "BUG-ID",
            short = 'n',
            long = "bug-id",
            help = "The bug id used for this review request"
        )]
        bug_id: u32,
        #[arg(
            value_name = "REVIEW-ID",
            short = 'r',
            long = "review-id",
            help = "The review id of an existing review request"
        )]
        review_id: Option<u32>,
        #[arg(
            value_name = "FILES",
            short = 'f',
            long = "file-list",
            help = "Files to be reviewed"
        )]
        file_list: Option<Vec<String>>,
        #[arg(
            value_name = "DIFF-FILE",
            short = 'd',
            long = "diff-file",
            help = "Diff file to upload"
        )]
        diff_file: Option<String>,
        #[arg(
            value_name = "REVIEWERS",
            short = 'u',
            long = "reviewers",
            help = "Reviewers who will review this commit"
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
            value_name = "REPO-NAME",
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Suppress the following error info:
    // failed printing to stdout: Broken pipe
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = Cli::parse();

    match args.command {
        Comm::Clean => clean_build(),
        Comm::Compdb {
            product_dir,
            make_target,
        } => gen_compdb(&product_dir, &make_target),
        Comm::Showcc { filename } => {
            let commands = fetch_compile_command(filename.as_str())?;
            for (idx, item) in commands.iter().enumerate() {
                println!("{}: {}", idx + 1, item);
            }

            Ok(())
        },
        Comm::Silist { prefix } => gen_silist(&prefix),
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
            let inet_ver = if ipv6 { InetVer::IPv6 } else { InetVer::IPv4 };
            let build_mode = if debug {
                BuildMode::Debug
            } else {
                BuildMode::Release
            };

            let printinfos = mkinfo::gen_mkinfo(
                &prodname,
                &MakeOpt {
                    coverity: coverity.to_owned(),
                    inet_ver,
                    passwd: password.to_owned(),
                    buildmode: build_mode,
                    webui: webui.to_owned(),
                },
            )?;

            mkinfo::dump_mkinfo(&printinfos, outfmt)
        }
        Comm::Perfan {
            file,
            daemon,
            bin,
            outfmt,
        } => {
            let data = proc_perfanno(&file, &bin, &daemon)?;
            dump_perfdata(&data, outfmt)
        }
        Comm::Review {
            bug_id,
            review_id,
            file_list,
            diff_file,
            reviewers,
            branch_name,
            repo_name,
            revisions,
        } => {
            let options = review::ReviewOptions {
                bug_id,
                review_id,
                file_list,
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
