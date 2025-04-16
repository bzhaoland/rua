use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};

use anstyle::{Ansi256Color, Color, Style};
use anyhow::{self, Context, Result, bail};
use bitflags::bitflags;
use clap::ValueEnum;
use console::Term;
use indexmap::IndexMap;
use regex::Regex;
use rustix::system::uname;
use serde_json::{Value, json};

use crate::utils;
use crate::utils::SvnInfo;

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(crate) struct MakeFlag: u64 {
        const RELEASE        = 0b00000001;
        const IPV6           = 0b00000010;
        const WEBUI          = 0b00000100; // Recommended, not mandatory
        const SHELL_PASSWORD = 0b00001000;
        const COVERITY       = 0b00010000;
        const COVERAGE       = 0b00100000;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(crate) enum ImageServer {
    B, // Beijing
    S, // Suzhou
}

impl fmt::Display for ImageServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageServer::B => write!(f, "b"),
            ImageServer::S => write!(f, "s"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MakeOpts {
    pub(crate) flag: MakeFlag,
    pub(crate) image_server: Option<ImageServer>,
    pub(crate) nostrip_bins: Vec<String>,
    pub(crate) defines: IndexMap<String, String>,
}

impl fmt::Display for MakeOpts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"MakeOpts {{
    flag: {:?},
    image_server: {:?},
    nostrip_bins: {:?},
    user_defines: {:?},
}}"#,
            self.flag, self.image_server, self.nostrip_bins, self.defines
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(crate) enum DumpFormat {
    Csv,
    Json,
    List,
    Tsv,
}

impl fmt::Display for DumpFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DumpFormat::Csv => write!(f, "DumpFormat::Csv"),
            DumpFormat::Json => write!(f, "DumpFormat::Json"),
            DumpFormat::List => write!(f, "DumpFormat::List"),
            DumpFormat::Tsv => write!(f, "DumpFormat::Tsv"),
        }
    }
}

/// Structure holding product information.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct ProductInfo {
    pub(crate) platform_model: String,
    pub(crate) product_model: String,
    pub(crate) name_id: usize,
    pub(crate) oem_id: String,
    pub(crate) family: Option<String>, // This field is available for branches >= R8, so make it optional
    pub(crate) short_name: String,
    pub(crate) long_name: String,
    pub(crate) snmp_descr: String,
    pub(crate) snmp_oid: String,
    pub(crate) icon: Option<String>,
}

// Makeinfo structure for R6 derivatives
#[derive(Clone, Debug)]
pub struct MakeInfoV1 {
    pub(crate) platform_model: String,
    pub(crate) make_target: String,
}

#[derive(Clone, Debug)]
pub struct MakeInfoV2 {
    pub(crate) platform_model: String,
    pub(crate) product_family: Option<String>,
    pub(crate) make_target: String,
    pub(crate) make_directory: String,
}

impl fmt::Display for MakeInfoV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"MakeInfo {{
  platform_model: "{}",
  product_family: "{:?}",
  make_target: "{}",
  make_directory: "{}",
}}"#,
            self.platform_model, self.product_family, self.make_target, self.make_directory,
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CompileInfo {
    pub(crate) product_name: String,
    pub(crate) product_model: String,
    pub(crate) product_oem_id: String,
    pub(crate) product_family: Option<String>,
    pub(crate) platform_model: String,
    pub(crate) make_target: String,
    pub(crate) make_directory: String,
    pub(crate) make_command: String,
}

impl fmt::Display for CompileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"CompileInfo {{
  product_name: "{}",
  product_model: "{}",
  product_family: "{:?}",
  platform_model: "{}",
  make_target: "{}",
  make_directory: "{}",
  make_command: "{}"
}}"#,
            self.product_name,
            self.product_model,
            self.product_family,
            self.platform_model,
            self.make_target,
            self.make_directory,
            self.make_command,
        )
    }
}

pub(crate) fn read_product_registry(svninfo: &SvnInfo) -> Result<Vec<ProductInfo>> {
    let proj_root = svninfo.working_copy_root_path();
    let product_info_path = proj_root.join("src/libplatform/hs_platform.c");
    if !product_info_path.is_file() {
        bail!(r#"File "{}" not available"#, product_info_path.display());
    }

    // Find out all matched records in src/libplatform/hs_platform.c
    let product_info_file = fs::File::open(&product_info_path)
        .context(format!("Can't open file {}", product_info_path.display()))?;
    let mut product_info_reader = BufReader::with_capacity(1024 * 512, product_info_file);
    let pattern_prodinfo = Regex::new(r#"(?i)^[[:blank:]]*\{[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:digit:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,(?:[[:blank:]]*([[:word:]]+)[[:blank:]]*,)?[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*(?:"([^"]*)"|(NULL))[[:blank:]]*\}"#).context("Failed to build regex for product info")?;
    let mut product_info_list: Vec<ProductInfo> = Vec::with_capacity(128);
    let mut line = String::with_capacity(512);
    while product_info_reader.read_line(&mut line)? != 0 {
        if let Some(captures) = pattern_prodinfo.captures(&line) {
            product_info_list.push(ProductInfo {
                platform_model: captures.get(1).unwrap().as_str().to_string(),
                product_model: captures.get(2).unwrap().as_str().to_string(),
                name_id: captures.get(3).unwrap().as_str().parse::<usize>()?,
                oem_id: captures.get(4).unwrap().as_str().to_string(),
                family: captures.get(5).map(|x| x.as_str().to_string()),
                short_name: captures.get(6).unwrap().as_str().to_string(),
                long_name: captures.get(7).unwrap().as_str().to_string(),
                snmp_descr: captures.get(8).unwrap().as_str().to_string(),
                snmp_oid: captures.get(9).unwrap().as_str().to_string(),
                icon: captures.get(10).map(|x| x.as_str().to_string()),
            })
        }
        line.clear();
    }
    product_info_list.shrink_to_fit();

    Ok(product_info_list)
}

/// Load makeinfos from the registry into a list
#[allow(unused)]
pub(crate) fn read_mkinfo_registry_v1(svninfo: &SvnInfo) -> anyhow::Result<Vec<MakeInfoV1>> {
    let makeinfo_path = svninfo
        .working_copy_root_path()
        .join("scripts/platform_table");
    if !makeinfo_path.is_file() {
        bail!(r#"File "{}" not available"#, makeinfo_path.display());
    }

    let makeinfo_file = fs::File::open(&makeinfo_path)
        .context(format!(r#"Can't open file "{}""#, makeinfo_path.display()))?;
    let mut makeinfo_reader = BufReader::with_capacity(1024 * 512, &makeinfo_file);
    let pattern_makeinfo = Regex::new(r#"^[[:blank:]]*([[:word:]]+)[[:blank:]]*([-[:word:]]+)"#)
        .context("Failed to build regex for makeinfo")?;
    let mut buf = String::with_capacity(128);
    let mut mkinfos: Vec<MakeInfoV1> = Vec::with_capacity(64);
    while makeinfo_reader.read_line(&mut buf)? != 0 {
        if let Some(captures) = pattern_makeinfo.captures(&buf) {
            mkinfos.push(MakeInfoV1 {
                platform_model: captures.get(1).unwrap().as_str().to_string(),
                make_target: captures.get(2).unwrap().as_str().to_string(),
            });
        }
        buf.clear()
    }
    mkinfos.shrink_to_fit();

    Ok(mkinfos)
}

/// Load makeinfos from the registry into a list
#[allow(unused)]
pub(crate) fn read_mkinfo_registry_v2(svninfo: &SvnInfo) -> anyhow::Result<Vec<MakeInfoV2>> {
    let makeinfo_path = svninfo
        .working_copy_root_path()
        .join("scripts/platform_table");
    if !makeinfo_path.is_file() {
        bail!(r#"File "{}" not available"#, makeinfo_path.display());
    }

    let makeinfo_file = fs::File::open(&makeinfo_path)
        .context(format!(r#"Can't open file "{}""#, makeinfo_path.display()))?;
    let mut makeinfo_reader = BufReader::with_capacity(1024 * 512, &makeinfo_file);
    let re_makeinfo = Regex::new(r#"^[[:blank:]]*([[:word:]]+),([-[:word:]]+),[^,]*,[[:blank:]]*"[[:blank:]]*(?:cd[[:blank:]]+)?([-[:word:]/]+)",[[:space:]]*[[:digit:]]+(?:[[:space:]]*,[[:space:]]*([[:word:]]+))?.*"#)
        .context("Failed to build regex for makeinfo")?;
    let mut buf = String::with_capacity(256);
    let mut mkinfos: Vec<MakeInfoV2> = Vec::with_capacity(256);
    while makeinfo_reader.read_line(&mut buf)? != 0 {
        if let Some(captures) = re_makeinfo.captures(&buf) {
            mkinfos.push(MakeInfoV2 {
                platform_model: captures.get(1).unwrap().as_str().to_string(),
                product_family: captures.get(4).map(|v| v.as_str().to_string()),
                make_target: captures.get(2).unwrap().as_str().to_string(),
                make_directory: captures.get(3).unwrap().as_str().to_string(),
            });
        }
        buf.clear()
    }
    mkinfos.shrink_to_fit();

    Ok(mkinfos)
}

const COLOR_GREEN: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(2))));
const COLOR_YELLOW: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(3))));

fn abbreviate_branch(svninfo: &SvnInfo) -> anyhow::Result<String> {
    let pattern_branch_part =
        Regex::new(r"HAWAII_([-[:word:]]+)").context("Build regex for nickname failed")?;
    let pattern_nonalnum =
        Regex::new(r#"[^[:alnum:]]+"#).context("Build regex for nonalnum failed")?;
    let captures = pattern_branch_part.captures(svninfo.branch_name());
    let nickname = pattern_nonalnum
        .replace_all(
            &match captures {
                Some(v) => v
                    .get(1)
                    .map_or(svninfo.branch_name().to_owned(), |x| x.as_str().to_string()),
                None => svninfo.branch_name().to_string(),
            },
            "",
        )
        .to_string();
    Ok(nickname)
}

/// Generate makeinfos for platforms in R6 releases
fn gen_mkinfo_by_nickname_v1(
    svninfo: &SvnInfo,
    nickname: &str,
    makeopts: MakeOpts,
) -> anyhow::Result<Vec<CompileInfo>> {
    // Read and filter products
    let re_nickname = Regex::new(format!(r#"(?i){}$"#, nickname).as_str())?;
    let product_infos = read_product_registry(svninfo)?
        .into_iter()
        .filter(|x| re_nickname.is_match(x.long_name.as_str()))
        .collect::<Vec<ProductInfo>>();

    // Read and hash makeinfos, allow duplicates
    let mkinfo_list = read_mkinfo_registry_v1(svninfo)?;
    let mut mkinfo_map = HashMap::with_capacity(256);
    for item in mkinfo_list {
        mkinfo_map
            .entry(item.platform_model.clone())
            .or_insert(Vec::with_capacity(1));
        let v = mkinfo_map.get_mut(item.platform_model.as_str()).unwrap();
        v.push(item);
    }
    mkinfo_map.shrink_to_fit();

    // Use branch name abbreviation to compose the image name
    let imagename_branch = abbreviate_branch(svninfo)?;

    // Compose an image name using product-series/make-target/IPv6-tag/date/username
    let mut imagename_suffix = String::with_capacity(64);
    imagename_suffix.push_str(if makeopts.flag.contains(MakeFlag::IPV6) {
        "V6-"
    } else {
        ""
    });
    imagename_suffix.push(if makeopts.flag.contains(MakeFlag::RELEASE) {
        'r'
    } else {
        'd'
    });
    imagename_suffix.push_str(chrono::Local::now().format("%m%d").to_string().as_str());
    if let Some(username) = utils::get_current_username() {
        imagename_suffix.push_str(format!("-{}", username).as_str())
    }

    let re_nonalnum = Regex::new(r#"[^[:alnum:]]+"#).context("Build regex for nonalnum failed")?;
    let mut compile_infos: Vec<CompileInfo> = Vec::new();
    for product in product_infos.iter() {
        let imagename_prodname = re_nonalnum.replace_all(&product.short_name, "");

        let mkinfo_arr = match mkinfo_map.get(&product.platform_model) {
            None => continue,
            Some(v) => v,
        };

        for mkinfo in mkinfo_arr.iter() {
            let mut make_target = mkinfo.make_target.clone();
            if makeopts.flag.contains(MakeFlag::IPV6) {
                make_target.push_str("-ipv6");
            }

            let imagename_target = re_nonalnum
                .replace_all(&mkinfo.make_target, "")
                .to_uppercase();
            let imagename = format!(
                "{}-{}-{}-{}",
                imagename_prodname, imagename_branch, imagename_target, imagename_suffix
            );

            let make_comm = format!(
                r#"hsdocker7 "make -C . -j8 {} ISBUILDRELEASE={} NOTBUILDUNIWEBUI={} HS_SHELL_PASSWORD={} HS_BUILD_COVERAGE={} HS_BUILD_COVERITY={} OS_IMAGE_FTP_IP={} {}IMG_NAME={} {}>build.log 2>&1""#,
                make_target,
                if makeopts.flag.contains(MakeFlag::RELEASE) {
                    1
                } else {
                    0
                },
                if makeopts.flag.contains(MakeFlag::WEBUI) {
                    0
                } else {
                    1
                },
                if makeopts.flag.contains(MakeFlag::SHELL_PASSWORD) {
                    1
                } else {
                    0
                },
                if makeopts.flag.contains(MakeFlag::COVERAGE) {
                    1
                } else {
                    0
                },
                if makeopts.flag.contains(MakeFlag::COVERITY) {
                    1
                } else {
                    0
                },
                makeopts.image_server.map_or_else(
                    || {
                        let nodename = uname().nodename().to_string_lossy().to_string();
                        if nodename.ends_with("-sz") {
                            "10.200.6.10"
                        } else {
                            "10.100.6.10"
                        }
                    },
                    |v| match v {
                        ImageServer::B => "10.100.6.10",
                        ImageServer::S => "10.200.6.10",
                    }
                ),
                if !makeopts.nostrip_bins.is_empty() {
                    format!(
                        "NOSTRIP={} ",
                        makeopts
                            .nostrip_bins
                            .iter()
                            .map(|x| x.trim())
                            .collect::<Vec<&str>>()
                            .join(",")
                            .as_str()
                    )
                } else {
                    String::new()
                },
                imagename,
                makeopts
                    .defines
                    .iter()
                    .map(|(k, v)| k.trim().to_owned() + "=" + v.trim())
                    .collect::<Vec<String>>()
                    .join(" ")
                    + " ",
            );
            compile_infos.push(CompileInfo {
                product_name: product.long_name.clone(),
                product_model: product.product_model.clone(),
                product_oem_id: product.oem_id.clone(),
                product_family: product.family.clone(),
                platform_model: mkinfo.platform_model.clone(),
                make_target,
                make_directory: ".".to_string(),
                make_command: make_comm,
            });
        }
    }

    anyhow::Ok(compile_infos)
}

/// Generate makeinfos for platforms in R8+ releases
fn gen_mkinfo_by_nickname_v2(
    svninfo: &SvnInfo,
    nickname: &str,
    makeopts: MakeOpts,
) -> anyhow::Result<Vec<CompileInfo>> {
    // Read and filter products
    let re_nickname = Regex::new(format!(r#"(?i){}$"#, nickname).as_str())?;
    let product_infos = read_product_registry(svninfo)?
        .into_iter()
        .filter(|x| re_nickname.is_match(x.long_name.as_str()))
        .collect::<Vec<ProductInfo>>();

    // Read and hash makeinfos, allow duplicates
    let mkinfo_list = read_mkinfo_registry_v2(svninfo)?;
    let mut mkinfo_map = HashMap::with_capacity(256);
    for item in mkinfo_list {
        mkinfo_map
            .entry(item.platform_model.clone())
            .or_insert(Vec::with_capacity(1));
        let v = mkinfo_map.get_mut(item.platform_model.as_str()).unwrap();
        v.push(item);
    }
    mkinfo_map.shrink_to_fit();

    // Product family field check
    let has_product_family_in_mkinfo = match svninfo
        .branch_name()
        .chars()
        .take(7)
        .collect::<String>()
        .as_str()
    {
        "MX_MAIN" if svninfo.revision() >= 293968 => true,
        "HAWAII_" => {
            let hawaii_release_ver = Regex::new(r#"HAWAII_(?:REL_)?R([[:digit:]]+)"#)
                .context("Failed to build regex for release version")?
                .captures(svninfo.branch_name())
                .context("Failed to capture release version")?
                .get(1)
                .unwrap()
                .as_str()
                .parse::<usize>()
                .context("Can't convert release version string to number")?;
            (hawaii_release_ver == 11 && svninfo.revision() >= 295630) || hawaii_release_ver > 11
        }
        _ => false,
    };

    // Use branch name abbreviation to compose the image name
    let imagename_branch = abbreviate_branch(svninfo)?;

    let mut imagename_suffix = String::with_capacity(64);
    imagename_suffix.push_str(if makeopts.flag.contains(MakeFlag::IPV6) {
        "V6-"
    } else {
        ""
    });
    imagename_suffix.push(if makeopts.flag.contains(MakeFlag::RELEASE) {
        'r'
    } else {
        'd'
    });
    imagename_suffix.push_str(chrono::Local::now().format("%m%d").to_string().as_str());
    if let Some(username) = utils::get_current_username() {
        imagename_suffix.push_str(format!("-{}", username).as_str())
    }

    let re_nonalnum = Regex::new(r#"[^[:alnum:]]+"#).context("Build regex for nonalnum failed")?;
    let mut compile_infos: Vec<CompileInfo> = Vec::new();
    for product in product_infos.iter() {
        let imagename_prodname = re_nonalnum.replace_all(&product.short_name, "");

        let mkinfo_arr = match mkinfo_map.get(&product.platform_model) {
            None => continue,
            Some(v) => v,
        };

        for mkinfo in mkinfo_arr.iter().filter(|x| {
            product.family.is_none()
                || !has_product_family_in_mkinfo
                || (x.product_family.is_some()
                    && x.product_family.as_ref().unwrap() == product.family.as_ref().unwrap())
        }) {
            let mut make_target = mkinfo.make_target.clone();
            if makeopts.flag.contains(MakeFlag::IPV6) {
                make_target.push_str("-ipv6");
            }

            let imagename_target = re_nonalnum
                .replace_all(&mkinfo.make_target, "")
                .to_uppercase();
            let imagename = format!(
                "{}-{}-{}-{}",
                imagename_prodname, imagename_branch, imagename_target, imagename_suffix
            );

            let make_comm = format!(
                r#"hsdocker7 "make -C {} -j8 {} ISBUILDRELEASE={} NOTBUILDUNIWEBUI={} HS_SHELL_PASSWORD={} HS_BUILD_COVERAGE={} HS_BUILD_COVERITY={} OS_IMAGE_FTP_IP={}{} IMG_NAME={} >build.log 2>&1""#,
                mkinfo.make_directory,
                make_target,
                if makeopts.flag.contains(MakeFlag::RELEASE) {
                    1
                } else {
                    0
                },
                if makeopts.flag.contains(MakeFlag::WEBUI) {
                    0
                } else {
                    1
                },
                if makeopts.flag.contains(MakeFlag::SHELL_PASSWORD) {
                    1
                } else {
                    0
                },
                if makeopts.flag.contains(MakeFlag::COVERAGE) {
                    1
                } else {
                    0
                },
                if makeopts.flag.contains(MakeFlag::COVERITY) {
                    1
                } else {
                    0
                },
                makeopts.image_server.map_or(
                    {
                        let nodename = uname().nodename().to_string_lossy().to_string();
                        if nodename.ends_with("-sz") {
                            "10.200.6.10".to_string()
                        } else {
                            "10.100.6.10".to_string()
                        }
                    },
                    |v| match v {
                        ImageServer::B => "10.100.6.10".to_string(),
                        ImageServer::S => "10.200.6.10".to_string(),
                    }
                ),
                if !makeopts.nostrip_bins.is_empty() {
                    format!(
                        r#" NOSTRIP="{}""#,
                        makeopts
                            .nostrip_bins
                            .iter()
                            .map(|x| x.trim().to_string())
                            .collect::<Vec<String>>()
                            .join(",")
                            .as_str()
                    )
                } else {
                    String::new()
                },
                imagename
            );
            compile_infos.push(CompileInfo {
                product_name: product.long_name.clone(),
                product_model: product.product_model.clone(),
                product_oem_id: product.oem_id.clone(),
                product_family: product.family.clone(),
                platform_model: mkinfo.platform_model.clone(),
                make_target,
                make_directory: mkinfo.make_directory.clone(),
                make_command: make_comm,
            });
        }
    }

    anyhow::Ok(compile_infos)
}

pub(crate) fn gen_mkinfo_by_nickname(
    nickname: &str,
    makeopts: MakeOpts,
) -> anyhow::Result<Vec<CompileInfo>> {
    let svninfo = utils::SvnInfo::new()?;

    // Check location
    let proj_root = svninfo.working_copy_root_path();
    if env::current_dir()?.as_path() != proj_root {
        bail!(
            r#"Wrong location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    // Check release
    let pattern_rel_num = Regex::new("^HAWAII_(?:REL_)?R([[:digit:]]+)|^MX_MAIN")
        .context("Failed to build regex for release num")?;
    let branch = svninfo.branch_name();
    match branch {
        "MX_MAIN" => gen_mkinfo_by_nickname_v2(&svninfo, nickname, makeopts),
        _ => {
            let rel_num: usize = pattern_rel_num
                .captures(svninfo.branch_name())
                .context("Failed to capture release num")?
                .get(1)
                .unwrap()
                .as_str()
                .parse()?;
            match rel_num {
                6 => gen_mkinfo_by_nickname_v1(&svninfo, nickname, makeopts),
                x if x >= 8 => gen_mkinfo_by_nickname_v2(&svninfo, nickname, makeopts),
                _ => bail!("Unsupported release"),
            }
        }
    }
}

pub(crate) fn gen_mkinfo_by_target(
    target: &str,
    makeopts: MakeOpts,
) -> anyhow::Result<Vec<CompileInfo>> {
    let svninfo = utils::SvnInfo::new()?;

    // Check location
    let proj_root = svninfo.working_copy_root_path();
    if env::current_dir()?.as_path() != proj_root {
        bail!(
            r#"Wrong location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    let mkinfo_list = read_mkinfo_registry_v2(&svninfo)?;
    let product_list = read_product_registry(&svninfo)?;

    // Compose an image name using product-series/make-target/IPv6-tag/date/username
    let re_nonalnum = Regex::new(r#"[^[:alnum:]]+"#).context("Build regex for nonalnum")?;
    let imagename_branch = abbreviate_branch(&svninfo)?;
    let mut imagename_suffix = String::with_capacity(32);
    if makeopts.flag.contains(MakeFlag::IPV6) {
        imagename_suffix.push_str("V6-");
    }
    imagename_suffix.push(if makeopts.flag.contains(MakeFlag::RELEASE) {
        'r'
    } else {
        'd'
    });
    let current_date = chrono::Local::now().format("%m%d").to_string();
    imagename_suffix.push_str(&current_date);
    let username = utils::get_current_username().context("Failed to get username")?;
    imagename_suffix.push('-');
    imagename_suffix.push_str(&username);

    let re_target =
        Regex::new(format!("(?i)^{}$", target.strip_suffix("-ipv6").unwrap_or(target)).as_str())?;
    let mut compile_infos: Vec<CompileInfo> = Vec::new();
    let imagename_prodname = "SG6000";
    for mkinfo in mkinfo_list
        .iter()
        .filter(|x| re_target.is_match(x.make_target.as_str()))
    {
        let imagename_target = re_nonalnum
            .replace_all(mkinfo.make_target.as_str(), "")
            .to_uppercase();
        let make_target = if !target.ends_with("-ipv6") && makeopts.flag.contains(MakeFlag::IPV6) {
            target.to_string() + "-ipv6"
        } else {
            target.to_string()
        };
        let make_comm = format!(
            r#"hsdocker7 "make -C {} -j8 {} ISBUILDRELEASE={} NOTBUILDUNIWEBUI={} HS_SHELL_PASSWORD={} HS_BUILD_COVERAGE={} HS_BUILD_COVERITY={} OS_IMAGE_FTP_IP={} IMG_NAME={}-{}-{}-{} >build.log 2>&1""#,
            mkinfo.make_directory,
            make_target,
            if makeopts.flag.contains(MakeFlag::RELEASE) {
                1
            } else {
                0
            },
            if makeopts.flag.contains(MakeFlag::WEBUI) {
                0
            } else {
                1
            },
            if makeopts.flag.contains(MakeFlag::SHELL_PASSWORD) {
                1
            } else {
                0
            },
            if makeopts.flag.contains(MakeFlag::COVERAGE) {
                1
            } else {
                0
            },
            if makeopts.flag.contains(MakeFlag::COVERITY) {
                1
            } else {
                0
            },
            makeopts.image_server.map_or(
                {
                    let nodename = uname().nodename().to_string_lossy().to_string();
                    if nodename.ends_with("-sz") {
                        "10.200.6.10".to_string()
                    } else {
                        "10.100.6.10".to_string()
                    }
                },
                |v| match v {
                    ImageServer::B => "10.100.6.10".to_string(),
                    ImageServer::S => "10.200.6.10".to_string(),
                }
            ),
            // Image name parts begin
            imagename_prodname,
            imagename_branch,
            imagename_target,
            imagename_suffix
        );

        for item in product_list
            .iter()
            .filter(|x| x.platform_model == mkinfo.platform_model)
        {
            if let Some(family1) = item.family.as_ref() {
                if let Some(family2) = mkinfo.product_family.as_ref() {
                    if family2 != family1 {
                        continue;
                    }
                }
            }

            compile_infos.push(CompileInfo {
                product_name: item.long_name.clone(),
                product_model: item.product_model.clone(),
                product_oem_id: item.oem_id.clone(),
                product_family: mkinfo
                    .product_family
                    .clone()
                    .or_else(|| item.family.clone()),
                platform_model: mkinfo.platform_model.clone(),
                make_target: make_target.clone(),
                make_directory: mkinfo.make_directory.clone(),
                make_command: make_comm.clone(),
            });
        }
    }

    anyhow::Ok(compile_infos)
}

#[derive(Clone, Debug)]
pub(crate) enum GenBy {
    Nickname(String),
    Target(String),
}

pub(crate) fn gen_mkinfo(by_what: GenBy, makeopts: MakeOpts) -> anyhow::Result<Vec<CompileInfo>> {
    match by_what {
        GenBy::Nickname(nickname) => gen_mkinfo_by_nickname(nickname.as_str(), makeopts),
        GenBy::Target(target) => gen_mkinfo_by_target(target.as_str(), makeopts),
    }
}

const MKINFO_DUMP_FIELDS: [&str; 8] = [
    "Product",
    "Model",
    "OEMID",
    "Family",
    "Platform",
    "Target",
    "Directory",
    "Command",
];

fn dump_json(compile_infos: &[CompileInfo]) -> anyhow::Result<()> {
    let mut output: Value = json!([]);
    for item in compile_infos.iter() {
        output.as_array_mut().unwrap().push(json!({
            MKINFO_DUMP_FIELDS[0]: item.product_name,
            MKINFO_DUMP_FIELDS[1]: item.product_model,
            MKINFO_DUMP_FIELDS[2]: item.product_oem_id,
            MKINFO_DUMP_FIELDS[3]: item.product_family,
            MKINFO_DUMP_FIELDS[4]: item.platform_model,
            MKINFO_DUMP_FIELDS[5]: item.make_target,
            MKINFO_DUMP_FIELDS[6]: item.make_directory,
            MKINFO_DUMP_FIELDS[7]: item.make_command,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&output)?);

    anyhow::Ok(())
}

fn dump_list(compile_infos: &[CompileInfo]) -> anyhow::Result<()> {
    // Style control
    let term_cols = Term::stdout().size().1;

    if compile_infos.is_empty() {
        println!("No matched info.");
        return anyhow::Ok(());
    }

    // Decorations
    let outerline = format!("{0}{1}{0:#}", COLOR_GREEN, "═".repeat(term_cols as usize));
    let innerline = format!("{0}{1}{0:#}", COLOR_GREEN, "─".repeat(term_cols as usize));

    println!(
        "{} matched info{}:",
        compile_infos.len(),
        if compile_infos.len() > 1 { "s" } else { "" }
    );

    println!("{}", outerline);
    let header_len = MKINFO_DUMP_FIELDS
        .iter()
        .map(|x| x.chars().count())
        .max()
        .unwrap()
        + 1;
    for (idx, item) in compile_infos.iter().enumerate() {
        println!(
            "{:<header_len$}: {}\n{:<header_len$}: {}\n{:<header_len$}: {}\n{}{:<header_len$}: {}\n{:<header_len$}: {}\n{:<header_len$}: {}\n{:<header_len$}: {}",
            MKINFO_DUMP_FIELDS[0],
            item.product_name,
            MKINFO_DUMP_FIELDS[1],
            item.product_model,
            MKINFO_DUMP_FIELDS[2],
            item.product_oem_id,
            item.product_family
                .as_ref()
                .map_or(String::new(), |x| format!(
                    "{:<header_len$}: {}\n",
                    MKINFO_DUMP_FIELDS[3], x
                )),
            MKINFO_DUMP_FIELDS[4],
            item.platform_model,
            MKINFO_DUMP_FIELDS[5],
            item.make_target,
            MKINFO_DUMP_FIELDS[6],
            item.make_directory,
            MKINFO_DUMP_FIELDS[7],
            item.make_command,
        );
        if idx < compile_infos.len() - 1 {
            println!("{}", innerline);
        }
    }
    println!("{}", outerline);

    println!(
        r#"{0}Run make command under the project root, i.e. "{1}"{0:#}"#,
        COLOR_YELLOW,
        utils::SvnInfo::new()?.working_copy_root_path().display(),
    );

    anyhow::Ok(())
}

fn dump_csv(infos: &[CompileInfo], delimiter: u8) -> anyhow::Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(std::io::stdout());

    writer.write_record(MKINFO_DUMP_FIELDS)?;
    let empty_string = String::new();
    for info in infos.iter() {
        writer.write_record([
            info.product_name.as_str(),
            info.product_model.as_str(),
            info.product_oem_id.as_str(),
            info.product_family.as_ref().unwrap_or(&empty_string),
            info.platform_model.as_str(),
            info.make_target.as_str(),
            info.make_directory.as_str(),
            info.make_command.as_str(),
        ])?;
    }
    writer.flush()?;

    anyhow::Ok(())
}

pub(crate) fn dump_mkinfo(infos: &[CompileInfo], format: DumpFormat) -> anyhow::Result<()> {
    match format {
        DumpFormat::Csv => dump_csv(infos, b','),
        DumpFormat::Json => dump_json(infos),
        DumpFormat::List => dump_list(infos),
        DumpFormat::Tsv => dump_csv(infos, b'\t'),
    }
}
