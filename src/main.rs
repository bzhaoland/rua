mod submods;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use submods::clean::clean_build;
use submods::compdb::gen_compdb;
use submods::mkinfo::{self, BuildMode, InetVer, MakeOpt};
use submods::profile::{self, dump_perfdata, proc_perfdata};
use submods::silist::gen_silist;

#[derive(Parser)]
#[command(
    name = "rua",
    author = "bzhao",
    version = "0.3.0",
    about = r"A tiny box combining many functionalities.",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Comm,
}

#[derive(Subcommand)]
enum Comm {
    /// Clean build files
    Clean,

    /// Generate JSON Compilation Database for the specified target
    Compdb {
        #[arg(
            value_name = "PRODUCT_DIR",
            help = r"The directory containing makefiles for the specified product"
        )]
        product_dir: String,
        #[arg(value_name = "MAKE_TARGET", help = r"Target to make")]
        make_target: String,
    },

    /// Generate file list used by Source Insight
    Silist {
        #[arg(
            value_name = "PRODUCT_DIR",
            help = "The directory containing makefiles for the specified product"
        )]
        product_dir: String,
        #[arg(value_name = "MAKE_TARGET", help = "Target to make")]
        make_target: String,
        #[arg(value_name = "PROJECT_ROOT", help = "Project root path on winbuilder")]
        project_root: String,
    },

    /// Generate make info for the specified product
    Mkinfo {
        #[arg(
            short = '4',
            long = "ipv4",
            default_value_t = true,
            conflicts_with = "ipv6",
            help = r"Build with ipv4 only"
        )]
        ipv4: bool,

        #[arg(
            short = '6',
            long = "ipv6",
            default_value_t = false,
            conflicts_with = "ipv4",
            help = r"Build with ipv4 & ipv6"
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
        #[arg(long = "ofmt", default_value = "list", value_name = "OUTPUT FORMAT")]
        ofmt: mkinfo::DumpFormat,

        /// Build with password
        #[arg(short = 'p', long = "password", default_value_t = false)]
        password: bool,

        /// Build with WebUI
        #[arg(short = 'w', long = "webui")]
        webui: bool,

        /// Product name, such as 'A3000', 'VM04', etc.
        /// Regex is supported, e.g. 'X\d+80'
        #[arg(value_name = "PRODNAME")]
        prodname: String,
    },

    /// Stat the given profiling file (only perf anno output supported now)
    Digest {
        #[arg(help = "File to be processed", value_name = "FILE")]
        file: PathBuf,

        #[arg(
            short = 'd',
            long = "daemon",
            value_name = "DAEMON",
            help = "Only match addresses owned by this daemon"
        )]
        daemon: String,

        #[arg(
            short = 's',
            long = "shared-object",
            value_name = "SHARED OBJECT",
            help = "The binary file used to translate the addressesto file lines"
        )]
        dso: PathBuf,
    },
}

fn main() -> Result<()> {
    // Suppress the following error info:
    // failed printing to stdout: Broken pipe
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = Cli::parse();

    match args.command {
        Comm::Clean => {
            clean_build()
        }
        Comm::Compdb {
            product_dir,
            make_target,
        } => {
            gen_compdb(&product_dir, &make_target)
        }
        Comm::Silist {
            product_dir,
            make_target,
            project_root,
        } => {
            gen_silist(&product_dir, &make_target, &project_root)
        }
        Comm::Mkinfo {
            coverity,
            ipv4: _,
            ipv6,
            password,
            prodname,
            debug,
            webui,
            ofmt,
        } => {
            let mut inet_ver = InetVer::IPv4;
            if ipv6 {
                inet_ver = InetVer::IPv6;
            }

            let mut buildtyp = BuildMode::Release;
            if debug {
                buildtyp = BuildMode::Debug;
            }

            let printinfos = mkinfo::gen_mkinfo(
                &prodname,
                &MakeOpt {
                    coverity: coverity.to_owned(),
                    inet_ver,
                    passwd: password.to_owned(),
                    buildmode: buildtyp,
                    webui: webui.to_owned(),
                },
            )?;

            mkinfo::dump_mkinfo(&printinfos, ofmt)?;

            Ok(())
        }
        Comm::Digest {
            file: datafile,
            daemon,
            dso: sofile,
        } => {
            let data = proc_perfdata(&datafile, &sofile, &daemon)?;
            dump_perfdata(&data, profile::DumpFormat::Table)?;
            Ok(())
        }
    }
}
