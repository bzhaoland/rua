use anstyle::{Ansi256Color, Color, Style};
use clap::{ArgGroup, Subcommand};

use crate::core::compdb::CompdbEngine;

const STYLE_YELLOW: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(3))));
const STYLE_YELLOW_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(3))))
    .bold();
const STYLE_RED_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(1))))
    .bold();
const STYLE_ITALIC: Style = Style::new().italic();

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum CompdbCmd {
    /// Generate a JSON compilation database (JCDB) for the given target.
    ///
    /// Run this command under either project root or submod dir. If you want a
    /// a compilation database for a specific module, run under submod dir. You
    /// may have to compile the target first before generating the compilation
    /// database under submod dir.
    ///
    /// Note:
    /// 1. Compilation database generated under submod dir only includes
    ///    files in this module.
    /// 2. R4+ releases are also supported by compdb.
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
  Some files are modified while running in built-in mode which is the default and faster:
  1. When running under project root dir:
     - scripts/last-rules.mk
     - scripts/rules.mk or scripts/common-rules.mk
     - Makefile
  2. When running under submod dir:
     - scripts/last-rules.mk
     - scripts/rules.mk or scripts/common-rules.mk
  These files may be left dirty if compdb aborted unexpectedly. You can restore the by executing
  (make sure you have backed up the changes you made):
  {2}svn revert Makefile scripts/last-rules.mk scripts/rules.mk scripts/common-rules.mk{2:#}"#,
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
            long = "merge",
            value_name = "OTHER-COMPDB",
            help = "Other compilation databases to be merged in. Multiple compilation databases can be passed in by specifying this option multiple times"
        )]
        merge_seq: Option<Vec<String>>,

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
    rua compdb add --commit 307164 hygon # Archive compilation database for hygon with a svn revision provided"#,
    STYLE_YELLOW_BOLD
    ))]
    Add {
        #[arg(
            value_name = "TARGET",
            help = "To which the new compilation database belongs"
        )]
        target: String,

        #[arg(
            short = 'c',
            long = "commit",
            value_name = "COMMIT",
            help = "Commit for compilation database (defaults to current repo revision/commit)"
        )]
        commit: Option<String>,

        #[arg(
            short = 'f',
            long = "compilation-database",
            value_name = "COMPILATION-DATABASE",
            help = "Use this compilation database other than the default (compile_commands.json)"
        )]
        compdb: Option<String>,
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
        new: Option<i64>,

        #[arg(
            short = 'o',
            long = "old",
            value_name = "N",
            help = format!("Remove {}N{:#} oldest generations", STYLE_ITALIC, STYLE_ITALIC)
        )]
        old: Option<i64>,
    },

    /// List all compilation database generations in store
    #[command(visible_alias = "list")]
    Ls,

    /// Merge compilation databases into the one in the current directory
    Merge {
        #[arg(
            short = 't',
            long = "target",
            value_name = "TARGET",
            help = "Target for the new compilation database"
        )]
        target: String,

        #[arg(
            short = 'r',
            long = "commit",
            value_name = "COMMIT",
            help = "Commit ID for the new compilation database (defaults to current svn/git commit ID)"
        )]
        commit: Option<String>,

        #[arg(value_name = "FILE", help = "Compilation database to be joined")]
        files: Vec<String>,
    },

    /// Select a compilation database generation from store to use
    Use {
        #[arg(value_name = "GENERATION", help = "Compilation database generation id")]
        generation: i64,
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
