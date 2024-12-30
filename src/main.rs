mod app;
mod config;
mod submods;
mod utils;

use clap::Parser;
use config::RuaConf;

fn main() -> anyhow::Result<()> {
    // Suppress the following error info:
    // failed printing to stdout: Broken pipe
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = app::Cli::parse();
    // let conf = RuaConf::load()?;
    let conf = RuaConf::new();

    app::run_app(&args, Some(conf).as_ref())
}
