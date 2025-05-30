use clap::Args;

#[derive(Args, Clone, Debug)]
pub(crate) struct SilistArgs {
    #[arg(
        value_name = "PREFIX",
        help = "Path prefix for source files, such as '/home/user/repos/MX_MAIN' (for Linux) or 'F:/repos/MX_MAIN' (for Windows), etc."
    )]
    pub(crate) prefix: String,
}
