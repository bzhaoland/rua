use std::fs;
use std::os::unix::fs::PermissionsExt;

use anyhow::Context;
use home::home_dir;
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

    let target_version = version.unwrap_or_else(|| {
        println!("Checking for updates...");
        let data = ftp_stream
            .retr_as_buffer("rua/releases.json")
            .unwrap()
            .into_inner();
        let release_info: Vec<ReleaseInfo> =
            serde_json::from_str(str::from_utf8(&data).unwrap()).unwrap();
        release_info
            .iter()
            .fold(Version::parse("0.0.0").unwrap(), |o, i| {
                o.max(Version::parse(&i.version).unwrap())
            })
            .to_string()
    });

    let data = ftp_stream
        .retr_as_buffer(&format!("rua/{}/rua", target_version))?
        .into_inner();
    let home = home_dir().context("Failed to get current user's home dir")?;
    let bin_dir = home.join(".local/bin");
    fs::create_dir_all(bin_dir.as_path())?;
    let dest = bin_dir.join("rua");
    fs::write(dest.as_path(), &data)?;
    let mut perm = fs::metadata(dest.as_path())?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(dest, perm)?;
    println!("Updated to version {}", target_version);

    ftp_stream.quit()?;

    Ok(())
}
