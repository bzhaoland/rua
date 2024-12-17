mod app;
mod config;
mod submods;
mod utils;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    // Suppress the following error info:
    // failed printing to stdout: Broken pipe
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let conf = config::load_config()?;
    let args = app::Cli::parse();

    app::run_app(&args, conf.as_ref())
}
