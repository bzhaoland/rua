use clap::Args;

#[derive(Args, Clone, Debug)]
pub(crate) struct UpdateArgs {
    #[arg(
        long = "--pin",
        value_name = "VERSION",
        help = "Pin a specified version"
    )]
    pub(crate) pin: Option<String>,
}
