use clap::Args;

#[derive(Args, Clone, Debug)]
pub(crate) struct ShowccArgs {
    #[arg(
        value_name = "SOURCE-FILE",
        help = "Source file name for which to fetch all the available compile commands"
    )]
    pub(crate) comp_unit: String,
    #[arg(
        value_name = "COMPDB",
        short = 'c',
        long = "compdb",
        help = r#"Compilation database (defaults to file "compile_commands.json" in the current directory)"#
    )]
    pub(crate) comp_db: Option<String>,
}
