use anstyle::{Ansi256Color, Color, Style};
use clap::Args;

const STYLE_YELLOW_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(3))))
    .bold();
const STYLE_RED: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(1))));
const STYLE_RED_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(1))))
    .bold();

/// Clean build files (run under project root)
#[derive(Args, Clone, Debug)]
#[command(after_help = format!(r#"{0}Examples:{0:#}
  rua clean  # Clean the entire project

{1}Caution:{1:#}
  All unversioned files will be {2}REMOVED{2:#} permanantly, including files created by YOU but not
  added to svn/git. Use it carefully!"#,
  STYLE_YELLOW_BOLD,
  STYLE_RED_BOLD,
  STYLE_RED))]
pub(crate) struct CleanArgs {
    #[arg(
        value_name = "ENTRY",
        help = "Files or dirs to be cleaned ('target' is always included even if not specified)"
    )]
    pub(crate) dirs: Option<Vec<String>>,

    #[arg(
        short = 'n',
        long = "ignore",
        value_name = "FILE",
        help = "File or directory to be ignored while cleaning. You can add multiple ignores by specifying this option multiple times"
    )]
    pub(crate) ignores: Option<Vec<String>>,
}
