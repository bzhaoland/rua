use clap::Args;

#[derive(Args, Clone, Debug)]
pub(crate) struct UpdateArgs {
    #[arg(
        long = "pin",
        value_name = "VERSION",
        help = "Pin to a specified rua version"
    )]
    pub(crate) pin: Option<String>,
}
