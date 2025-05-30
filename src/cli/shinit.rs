use clap::Args;
use clap_complete::Shell;

#[derive(Args, Clone, Debug)]
pub(crate) struct ShinitArgs {
    #[arg(value_name = "SHELL", help = "Shell type", value_enum)]
    pub(crate) shell: Shell,
}
