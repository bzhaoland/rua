use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};

use anstyle::{AnsiColor, Color, Style};
use anyhow::{self, bail, Context, Result};
use bitflags::bitflags;
use clap::ValueEnum;
use regex::Regex;
use rustix::system::uname;
use serde_json::{json, Value};

use crate::utils;
use crate::utils::SvnInfo;

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(crate) struct MakeFlag: u64 {
        const RELEASE        = 0b00000001;
        const IPV6           = 0b00000010;
        const WEBUI          = 0b00000100; // Only recommend
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct MakeOpts {
    pub(crate) flag: MakeFlag,
    pub(crate) image_server: Option<ImageServer>,
}

impl fmt::Display for MakeOpts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"MakeOpts {{
    flag: {:?}
    image_server: {:?}
}}"#,
            self.flag, self.image_server
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
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ProductInfo {
    pub(crate) platform_code: String,
    pub(crate) model: String,
    pub(crate) name_id: usize,
    pub(crate) oem_id: String,
    pub(crate) family: String,
    pub(crate) short_name: String,
    pub(crate) long_name: String,
    pub(crate) snmp_descr: String,
    pub(crate) snmp_oid: String,
    pub(crate) icon_path: Option<String>,
}

#[derive(Debug)]
pub struct MakeInfo {
    pub(crate) platform_model: String,
    pub(crate) product_family: Option<String>,
    pub(crate) make_target: String,
    pub(crate) make_directory: String,
}

#[derive(Debug)]
pub(crate) struct CompileInfo {
    pub(crate) product_name: String,
    pub(crate) product_model: String,
    pub(crate) product_family: String,
    pub(crate) platform_model: String,
    pub(crate) make_target: String,
    pub(crate) make_directory: String,
    pub(crate) make_command: String,
}

impl fmt::Display for MakeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"MakeInfo {{
  platform_model: "{}",
  target: "{}",
  directory: "{}",
}}"#,
            self.platform_model, self.make_target, self.make_directory,
        )
    }
}

impl fmt::Display for CompileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"CompileInfo {{
  platform_model: "{}",
  target: "{}",
  directory: "{}",
  command: "{}"
}}"#,
            self.platform_model, self.make_target, self.make_directory, self.make_command,
        )
    }
}

pub(crate) fn load_product_infos(svninfo: &SvnInfo, nickname: &str) -> Result<Vec<ProductInfo>> {
    let proj_root = svninfo.working_copy_root_path();
    let product_info_path = proj_root.join("src/libplatform/hs_platform.c");
    if !product_info_path.is_file() {
        bail!(r#"File "{}" not available"#, product_info_path.display());
    }

    // Find out all matched records in src/libplatform/hs_platform.c
    let product_info_file = fs::File::open(&product_info_path)
        .context(format!("Can't open file {}", product_info_path.display()))?;
    let mut product_info_reader = BufReader::with_capacity(1024 * 512, product_info_file);
    let product_info_pattern = Regex::new(&format!(r#"(?i)^[[:blank:]]*\{{[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:digit:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*{})"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*(?:"([^"]*)"|(NULL))[[:blank:]]*\}}"#, nickname)).context("Failed to build pattern for product info")?;
    let mut product_info_list: Vec<ProductInfo> = Vec::with_capacity(128);
    let mut line = String::with_capacity(512);
    while product_info_reader.read_line(&mut line)? != 0 {
        if let Some(captures) = product_info_pattern.captures(&line) {
            product_info_list.push(ProductInfo {
                platform_code: captures.get(1).unwrap().as_str().to_string(),
                model: captures.get(2).unwrap().as_str().to_string(),
                name_id: captures.get(3).unwrap().as_str().parse::<usize>()?,
                oem_id: captures.get(4).unwrap().as_str().to_string(),
                family: captures.get(5).unwrap().as_str().to_string(),
                short_name: captures.get(6).unwrap().as_str().to_string(),
                long_name: captures.get(7).unwrap().as_str().to_string(),
                snmp_descr: captures.get(8).unwrap().as_str().to_string(),
                snmp_oid: captures.get(9).unwrap().as_str().to_string(),
                icon_path: captures.get(10).map(|x| x.as_str().to_string()),
            })
        }
        line.clear();
    }
    product_info_list.shrink_to_fit();

    Ok(product_info_list)
}

pub(crate) fn load_makeinfo_table(svninfo: &SvnInfo) -> Result<HashMap<String, Vec<MakeInfo>>> {
    let proj_root = svninfo.working_copy_root_path();
    let makeinfo_path = proj_root.join("scripts/platform_table");
    if !makeinfo_path.is_file() {
        bail!(r#"File "{}" not available"#, makeinfo_path.display());
    }

    let makeinfo_file = fs::File::open(&makeinfo_path)
        .context(format!(r#"Can't open file "{}""#, makeinfo_path.display()))?;
    let mut makeinfo_reader = BufReader::with_capacity(1024 * 512, &makeinfo_file);
    let makeinfo_pattern =
    Regex::new(r#"^[[:blank:]]*([[:word:]]+),([-[:word:]]+),[^,]*,[[:blank:]]*"[[:blank:]]*(?:cd[[:blank:]]+)?([-[:word:]/]+)",[[:space:]]*[[:digit:]]+(?:[[:space:]]*,[[:space:]]*([[:word:]]+))?.*"#)
        .context("Failed to build pattern for makeinfo")?;
    let mut line = String::with_capacity(512);
    let mut mkinfos: HashMap<String, Vec<MakeInfo>> = HashMap::with_capacity(256);
    while makeinfo_reader.read_line(&mut line)? != 0 {
        if let Some(captures) = makeinfo_pattern.captures(&line) {
            let makeinfo_item = MakeInfo {
                platform_model: captures.get(1).unwrap().as_str().to_string(),
                product_family: captures.get(4).map(|v| v.as_str().to_string()),
                make_target: captures.get(2).unwrap().as_str().to_string(),
                make_directory: captures.get(3).unwrap().as_str().to_string(),
            };

            mkinfos
                .entry(makeinfo_item.platform_model.clone())
                .or_insert(Vec::with_capacity(1));

            let v = mkinfos.get_mut(&makeinfo_item.platform_model).unwrap();
            v.push(makeinfo_item);
        }

        line.clear()
    }
    mkinfos.shrink_to_fit();

    Ok(mkinfos)
}

const COLOR_GREEN: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
const COLOR_YELLOW: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));

/// Generate the make information for the given platform.
/// This function must run under the project root which is a valid svn repo.
pub(crate) fn gen_mkinfo(nickname: &str, makeopts: MakeOpts) -> anyhow::Result<Vec<CompileInfo>> {
    let svninfo = utils::SvnInfo::new()?;

    // Check location
    let proj_root = svninfo.working_copy_root_path();
    if env::current_dir()?.as_path() != proj_root {
        bail!(
            r#"Wrong location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    let product_infos = load_product_infos(&svninfo, nickname)?;
    let mkinfos = load_makeinfo_table(&svninfo)?;

    let has_family_field_in_plattable = match svninfo
        .branch_name()
        .chars()
        .take(7)
        .collect::<String>()
        .as_str()
    {
        "MX_MAIN" if svninfo.revision() >= 293968 => true,
        "HAWAII_" => {
            let hawaii_release_ver = Regex::new(r#"HAWAII_(?:REL_)?R([[:digit:]]+)"#)
                .context("Failed to build pattern for release version")?
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

    // Compose an image name using product-series/make-target/IPv6-tag/date/username
    let mut imagename_suffix = String::with_capacity(32);

    let pattern_nonalnum =
        Regex::new(r#"[^[:alnum:]]+"#).context("Error building regex pattern for nonalnum")?;

    // Use branch name abbreviation to compose the image name
    let nickname_pattern = Regex::new(r"HAWAII_([-[:word:]]+)")
        .context("Error building regex pattern for nickname")
        .unwrap();
    let captures = nickname_pattern.captures(svninfo.branch_name());
    let branch_nickname = pattern_nonalnum
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

    // IPv6 check
    if makeopts.flag.contains(MakeFlag::IPV6) {
        imagename_suffix.push_str("V6-");
    }

    // Building mode
    imagename_suffix.push(if makeopts.flag.contains(MakeFlag::RELEASE) {
        'r'
    } else {
        'd'
    });

    // Timestamp
    imagename_suffix.push_str(&chrono::Local::now().format("%m%d").to_string());

    // Username
    let username = utils::get_current_username().context("Failed to get username")?;
    imagename_suffix.push('-');
    imagename_suffix.push_str(&username);

    let mut compile_infos: Vec<CompileInfo> = Vec::new();
    for product in product_infos.iter() {
        let imagename_prodname = pattern_nonalnum.replace_all(&product.short_name, "");

        let mkinfo_arr = mkinfos.get(&product.platform_code);
        if mkinfo_arr.is_none() {
            continue;
        }
        let mkinfo_set = mkinfo_arr.unwrap();

        for mkinfo in mkinfo_set.iter().filter(|x| {
            !has_family_field_in_plattable
                || (x.product_family.is_some()
                    && x.product_family.as_ref().unwrap() == &product.family)
        }) {
            let mut make_target = mkinfo.make_target.clone();
            if makeopts.flag.contains(MakeFlag::IPV6) {
                make_target.push_str("-ipv6");
            }

            let imagename_target = pattern_nonalnum
                .replace_all(&mkinfo.make_target, "")
                .to_uppercase();
            let imagename = format!(
                "{}-{}-{}-{}",
                imagename_prodname, branch_nickname, imagename_target, imagename_suffix
            );

            let make_comm = format!(
                r#"hsdocker7 "make -C {} -j8 {} ISBUILDRELEASE={} NOTBUILDUNIWEBUI={} HS_SHELL_PASSWORD={} HS_BUILD_COVERAGE={} HS_BUILD_COVERITY={} OS_IMAGE_FTP_IP={} IMG_NAME={} >build.log 2>&1""#,
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
                imagename
            );
            compile_infos.push(CompileInfo {
                product_name: product.long_name.clone(),
                product_model: product.model.clone(),
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

/// Dump mkinfo records as csv
fn dump_csv(infos: &[CompileInfo]) -> anyhow::Result<()> {
    let mut writer = csv::Writer::from_writer(std::io::stdout());

    writer.write_record([
        "Product",
        "Model",
        "Family",
        "Platform",
        "Target",
        "Directory",
        "Command",
    ])?;
    for info in infos.iter() {
        writer.write_record([
            &info.product_name,
            &info.product_model,
            &info.product_family,
            &info.platform_model,
            &info.make_target,
            &info.make_directory,
            &info.make_command,
        ])?;
    }

    writer.flush()?;

    anyhow::Ok(())
}

fn dump_json(compile_infos: &[CompileInfo]) -> anyhow::Result<()> {
    let mut output: Value = json!([]);
    for item in compile_infos.iter() {
        output.as_array_mut().unwrap().push(json!({
            "Product": item.product_name,
            "Model": item.product_name,
            "Family": item.product_family,
            "Platform": item.platform_model,
            "Target": item.make_target,
            "Directory": item.make_directory,
            "Command": item.make_command,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&output)?);

    anyhow::Ok(())
}

fn dump_list(compile_infos: &[CompileInfo]) -> anyhow::Result<()> {
    // Style control
    let term_size = crossterm::terminal::window_size()?;

    if compile_infos.is_empty() {
        println!("No matched info.");
        return anyhow::Ok(());
    }

    // Decorations
    let outer_decor = format!(
        "{COLOR_GREEN}{}{COLOR_GREEN:#}",
        "=".repeat(term_size.columns as usize)
    );
    let inner_decor = format!(
        "{COLOR_GREEN}{}{COLOR_GREEN:#}",
        "-".repeat(term_size.columns as usize)
    );

    println!(
        "{} matched info{}:",
        compile_infos.len(),
        if compile_infos.len() > 1 { "s" } else { "" }
    );

    println!("{}", outer_decor);
    for (idx, item) in compile_infos.iter().enumerate() {
        println!("Product   : {}", item.product_name);
        println!("Model     : {}", item.product_model);
        println!("Family    : {}", item.product_family);
        println!("Platform  : {}", item.platform_model);
        println!("Target    : {}", item.make_target);
        println!("Directory : {}", item.make_directory);
        println!("Command   : {}", item.make_command);

        if idx < compile_infos.len() - 1 {
            println!("{}", inner_decor);
        }
    }
    println!("{}", outer_decor);

    println!(
        r#"{COLOR_YELLOW}Run make command under the project root, i.e. "{}"{COLOR_YELLOW:#}"#,
        utils::SvnInfo::new()?.working_copy_root_path().display(),
    );

    anyhow::Ok(())
}

fn dump_tsv(infos: &[CompileInfo]) -> anyhow::Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(std::io::stdout());

    writer.write_record([
        "Product",
        "Model",
        "Family",
        "Platform",
        "Target",
        "Directory",
        "Command",
    ])?;
    for info in infos.iter() {
        writer.write_record([
            &info.product_name,
            &info.product_model,
            &info.product_family,
            &info.platform_model,
            &info.make_target,
            &info.make_directory,
            &info.make_command,
        ])?;
    }
    writer.flush()?;

    anyhow::Ok(())
}

/// Dump the make information to the screen.
pub fn dump_mkinfo(infos: &[CompileInfo], format: DumpFormat) -> anyhow::Result<()> {
    match format {
        DumpFormat::Csv => dump_csv(infos),
        DumpFormat::Json => dump_json(infos),
        DumpFormat::List => dump_list(infos),
        DumpFormat::Tsv => dump_tsv(infos),
    }
}
