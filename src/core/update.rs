use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use anyhow::Context;
use home::home_dir;
use indicatif::{ProgressBar, ProgressStyle};
use semver::Version;
use suppaftp::FtpStream;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ReleaseInfo {
    version: String,
}

pub(crate) fn update(version: Option<String>) -> anyhow::Result<()> {
    let mut ftp_stream = FtpStream::connect("10.100.6.10:21")?;
    ftp_stream.login("anonymous", "")?;
    ftp_stream.cwd("bzhao")?;

    let target_version = if let Some(v) = version {
        v
    } else {
        let pbar = ProgressBar::no_length().with_style(ProgressStyle::with_template(
            "Checking for updates...{msg}",
        )?);
        pbar.tick();
        let data = ftp_stream
            .retr_as_buffer("rua/releases.json")
            .unwrap()
            .into_inner();
        let release_info: Vec<ReleaseInfo> = serde_json::from_str(str::from_utf8(&data)?)?;
        let latest_version = release_info
            .iter()
            .fold(Version::parse("0.0.0")?, |o, i| {
                o.max(Version::parse(&i.version).unwrap())
            })
            .to_string();
        pbar.finish_and_clear();
        latest_version
    };

    let pbar = ProgressBar::no_length().with_style(ProgressStyle::with_template(
        format!("Downloading rua {}...", target_version).as_str(),
    )?);
    pbar.tick();
    let home = home_dir().context("Failed to get current user's home dir")?;
    let bin_dir = home.join(".local/bin");
    fs::create_dir_all(bin_dir.as_path())
        .context(format!("Failed to create dir {}", bin_dir.display()))?;
    let data = ftp_stream
        .retr_as_buffer(&format!("rua/{}/rua", target_version))?
        .into_inner();
    let mut temp =
        tempfile::NamedTempFile::new_in(bin_dir.as_path()).context("Failed to create tempfile")?;
    temp.write_all(&data)
        .context("Failed to write local file")?;

    pbar.set_style(ProgressStyle::with_template(
        format!("Installing rua {}...", target_version).as_str(),
    )?);
    pbar.tick();
    let mut perm = fs::metadata(temp.path())
        .context(format!("Failed to stat {}", temp.path().display()))?
        .permissions();
    perm.set_mode(0o755);
    fs::set_permissions(temp.path(), perm)?;
    let dest = bin_dir.join("rua");
    fs::rename(temp.path(), dest.as_path()).context(format!(
        "Failed to rename {} to {}",
        temp.path().display(),
        dest.display()
    ))?;
    pbar.finish_and_clear();
    println!("Updated rua to version {}", target_version);

    ftp_stream.quit().context("Failed to quit ftp stream")?;
    Ok(())
}
