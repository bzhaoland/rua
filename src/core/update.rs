use std::fs;
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
    let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
    let mut ftp_stream = FtpStream::connect("10.100.6.10:21")?;
    ftp_stream.login("anonymous", "")?;
    ftp_stream.cwd("bzhao")?;

    let target_version = semver::Version::parse(
        if let Some(v) = version {
            if current_version == semver::Version::parse(&v)? {
                println!("You are already on the version {} of rua", v);
                return Ok(());
            }
            v
        } else {
            // Checking for the latest release
            let pbar = ProgressBar::no_length()
                .with_style(ProgressStyle::with_template("Checking for updates...")?);
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
            if current_version == semver::Version::parse(&latest_version)? {
                println!(
                    "You are already on the latest version ({}) of rua",
                    latest_version
                );
                return Ok(());
            }

            latest_version
        }
        .as_str(),
    )?;

    let pbar = ProgressBar::no_length().with_style(ProgressStyle::with_template(
        format!("Updating rua to {}...", target_version).as_str(),
    )?);
    pbar.tick();
    let home = home_dir().context("Failed to get current user's home dir")?;
    let bin_dir = home.join(".local/bin");
    fs::create_dir_all(bin_dir.as_path())
        .context(format!("Failed to create dir {}", bin_dir.display()))?;
    let dest = bin_dir.join("rua");
    let data = ftp_stream
        .retr_as_buffer(&format!("rua/{}/rua", target_version))?
        .into_inner();
    fs::write(dest.as_path(), data.as_slice()).context("Failed to write the binary")?;
    let mut perm = fs::metadata(dest.as_path())
        .context(format!("Failed to stat {}", dest.display()))?
        .permissions();
    perm.set_mode(0o755);
    fs::set_permissions(dest.as_path(), perm)?;
    pbar.set_style(ProgressStyle::with_template(
        format!(
            "{} rua from {} to {}",
            if current_version < target_version {
                "Upgraded"
            } else {
                "Downgraded"
            },
            current_version,
            target_version
        )
        .as_str(),
    )?);
    pbar.finish();

    ftp_stream.quit().context("Failed to quit ftp stream")?;
    Ok(())
}
