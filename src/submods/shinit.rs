use clap::Command;
use clap_complete::{generate, Shell};

pub(crate) fn gen_completion(cmd: &mut Command, shell_type: Shell) {
    generate(
        shell_type,
        cmd,
        cmd.get_name().to_string(),
        &mut std::io::stdout(),
    );
}
