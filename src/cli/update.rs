use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct UpdateArgs {
    #[arg(
        long = "pin",
        value_name = "VERSION",
        help = "Pin to a specified rua version"
    )]
    pub pin: Option<String>,
}
