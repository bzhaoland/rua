use std::os::unix::fs::PermissionsExt;
use std::{fs, io::Write};

use anstyle::{Ansi256Color, Color, Style};
use anyhow::Context;
use home::home_dir;
use indicatif::{ProgressBar, ProgressStyle};
use rustix::system::uname;
use semver::Version;
use suppaftp::FtpStream;
use tempfile::NamedTempFile;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ReleaseInfo {
    version: String,
}

const STYLE_BLUE_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(4))))
    .bold();

pub(crate) fn update(version: Option<String>) -> anyhow::Result<()> {
    let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
    let mut ftp_stream = FtpStream::connect(format!(
        "{}:21",
        if uname().nodename().to_string_lossy().ends_with("-sz") {
            "10.200.6.10"
        } else {
            "10.100.6.10"
        }
    ))?;
    ftp_stream.login("anonymous", "")?;
    ftp_stream.cwd("bzhao/rua")?;

    let target_version = semver::Version::parse(
        if let Some(v) = version {
            if current_version == semver::Version::parse(&v)? {
                println!(
                    "You're already on the target version of rua ({1}v{0}{1:#})",
                    v, STYLE_BLUE_BOLD
                );
                return Ok(());
            }
            v
        } else {
            // Checking for the latest release
            let pbar = ProgressBar::no_length()
                .with_style(ProgressStyle::with_template("Checking for updates...")?);
            pbar.tick();
            let data = ftp_stream
                .retr_as_buffer("releases.json")
                .unwrap()
                .into_inner();
            let release_info: Vec<ReleaseInfo> = serde_json::from_str(str::from_utf8(&data)?)?;
            let latest_version = release_info
                .iter()
                .fold(Version::parse("0.0.0")?, |o, i| {
                    o.max(Version::parse(&i.version).unwrap())
                })
                .to_string();
            pbar.finish();
            if current_version == semver::Version::parse(&latest_version)? {
                println!(
                    "You're already on the latest version of rua ({1}v{0}{1:#})",
                    latest_version, STYLE_BLUE_BOLD
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
        .retr_as_buffer(&format!("{}/rua", target_version))
        .context(format!(
            "Failed to retrieve rua v{} from ftp server",
            target_version
        ))?
        .into_inner();
    let mut tmpfile = NamedTempFile::with_prefix_in("rua.", dest.parent().unwrap())?;
    tmpfile
        .write_all(data.as_slice())
        .context("Failed to save the binary data")?;
    tmpfile.flush().context("Failed to flush to temp file")?;
    fs::rename(tmpfile.path(), dest.as_path()).context("Failed to rename the binary")?;
    let mut perm = fs::metadata(dest.as_path())
        .context(format!("Failed to stat {}", dest.display()))?
        .permissions();
    perm.set_mode(0o755);
    fs::set_permissions(dest.as_path(), perm)?;
    pbar.set_style(ProgressStyle::with_template(
        format!(
            "{0} rua from {3}v{1}{3:#} to {3}v{2}{3:#}",
            if current_version < target_version {
                "Upgraded"
            } else {
                "Downgraded"
            },
            current_version,
            target_version,
            STYLE_BLUE_BOLD
        )
        .as_str(),
    )?);
    pbar.finish();

    ftp_stream.quit().context("Failed to quit ftp stream")
}
