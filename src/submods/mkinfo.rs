use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};

use anyhow::{self, bail, Context};
use bitflags::bitflags;
use clap::ValueEnum;
use crossterm::terminal;
use regex::Regex;
use serde_json::{json, Value};

use crate::utils;

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MakeFlag: u8 {
        const RELEASE_BUILD         = 0b00000001;
        const ENABLE_IPV6           = 0b00000010;
        const PLEASE_WEBUI          = 0b00000100; // Not enforced
        const ENABLE_SHELL_PASSWORD = 0b00001000;
        const ENABLE_COVERITY       = 0b00010000;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DumpFormat {
    Csv,
    Json,
    List,
    Tsv,
}

impl fmt::Display for DumpFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DumpFormat::Csv => write!(f, "Csv"),
            DumpFormat::Json => write!(f, "Json"),
            DumpFormat::List => write!(f, "List"),
            DumpFormat::Tsv => write!(f, "Tsv"),
        }
    }
}

/// Structure holding product information.
#[derive(Debug)]
#[allow(dead_code)]
struct ProductInfo {
    platform_code: String,
    model: String,
    name_id: usize,
    oem_id: String,
    family: String,
    short_name: String,
    long_name: String,
    snmp_descr: String,
    snmp_oid: String,
    icon_path: Option<String>,
}

#[derive(Debug)]
pub struct MakeInfo {
    platform_model: String,
    product_family: Option<String>,
    make_goal: String,
    make_directory: String,
}

#[derive(Debug)]
pub struct CompileInfo {
    product_name: String,
    product_model: String,
    product_family: String,
    platform_model: String,
    make_goal: String,
    make_directory: String,
    make_command: String,
}

impl fmt::Display for MakeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"MakeInfo {{
  platform_model: "{}",
  make_goal: "{}",
  make_directory: "{}",
}}"#,
            self.platform_model, self.make_goal, self.make_directory,
        )
    }
}

impl fmt::Display for CompileInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"CompileInfo {{
  platform_model: "{}",
  make_goal: "{}",
  make_directory: "{}",
  make_command: "{}"
}}"#,
            self.platform_model, self.make_goal, self.make_directory, self.make_command,
        )
    }
}

const COLOR_ANSI_GRN: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));
const COLOR_ANSI_YLW: anstyle::Style =
    anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)));

/// Generate the make information for the given platform.
/// This function must run under the project root which is a valid svn repo.
pub fn gen_mkinfo(
    nickname: &str,
    makeflag: MakeFlag,
    imageserver: Option<&str>,
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

    // Check file
    let product_info_path = proj_root.join("src/libplatform/hs_platform.c");
    if !product_info_path.is_file() {
        bail!(r#"File "{}" not available"#, product_info_path.display());
    }
    let makeinfo_path = proj_root.join("scripts/platform_table");
    if !makeinfo_path.is_file() {
        bail!(r#"File "{}" not available"#, makeinfo_path.display());
    }

    let repo_branch = svninfo.branch_name();
    let repo_revision = svninfo.revision();
    let has_family_field_in_plattable =
        match repo_branch.chars().take(7).collect::<String>().as_str() {
            "MX_MAIN" if repo_revision >= 293968 => true,
            "HAWAII_" => {
                let hawaii_release_ver = Regex::new(r#"HAWAII_(?:REL_)?R([[:digit:]]+)"#)
                    .context("Error building pattern for release version")?
                    .captures(repo_branch)
                    .context("Error capturing release version from branch name")?
                    .get(1)
                    .unwrap()
                    .as_str()
                    .parse::<usize>()
                    .context("Error parsing release version as an integer")?;
                (hawaii_release_ver == 11 && repo_revision >= 295630) || hawaii_release_ver > 11
            }
            _ => false,
        };

    // Find out all matched records in src/libplatform/hs_platform.c
    let product_info_file = fs::File::open(&product_info_path).context(format!(
        "Error opening file {}",
        product_info_path.display()
    ))?;
    let mut product_info_reader = BufReader::with_capacity(1024 * 512, product_info_file);
    let product_info_pattern = Regex::new(&format!(r#"(?i)^[[:blank:]]*\{{[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:digit:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*{})"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*(?:"([^"]*)"|(NULL))[[:blank:]]*\}}"#, nickname)).context("Error building regex pattern of product info")?;
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

    // Fetch makeinfo for each product
    let makeinfo_file = fs::File::open(&makeinfo_path).context(format!(
        r#"Error opening file "{}""#,
        makeinfo_path.display()
    ))?;
    let mut makeinfo_reader = BufReader::with_capacity(1024 * 512, &makeinfo_file);
    let makeinfo_pattern =
        Regex::new(r#"^[[:blank:]]*([[:word:]]+),([-[:word:]]+),[^,]*,[[:blank:]]*"[[:blank:]]*(?:cd[[:blank:]]+)?([-[:word:]/]+)",[[:space:]]*[[:digit:]]+(?:[[:space:]]*,[[:space:]]*([[:word:]]+))?.*"#)
            .context("Error building regex pattern for makeinfo")?;
    let mut mkinfos: HashMap<String, Vec<MakeInfo>> = HashMap::with_capacity(256);
    while makeinfo_reader.read_line(&mut line)? != 0 {
        if let Some(captures) = makeinfo_pattern.captures(&line) {
            let makeinfo_item = MakeInfo {
                platform_model: captures.get(1).unwrap().as_str().to_string(),
                product_family: captures.get(4).map(|v| v.as_str().to_string()),
                make_goal: captures.get(2).unwrap().as_str().to_string(),
                make_directory: captures.get(3).unwrap().as_str().to_string(),
            };

            if !mkinfos.contains_key(&makeinfo_item.platform_model) {
                mkinfos.insert(makeinfo_item.platform_model.clone(), Vec::with_capacity(1));
            }

            let v = mkinfos.get_mut(&makeinfo_item.platform_model).unwrap();
            v.push(makeinfo_item);
        }

        line.clear()
    }
    mkinfos.shrink_to_fit();

    let mut compile_infos: Vec<CompileInfo> = Vec::new();
    for product in product_info_list.iter() {
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
            let mut make_goal = mkinfo.make_goal.clone();
            if makeflag.contains(MakeFlag::ENABLE_IPV6) {
                make_goal.push_str("-ipv6");
            }

            let make_comm = format!(
                "hsdocker7 make -C {} -j8 {} ISBUILDRELEASE={} NOTBUILDUNIWEBUI={} HS_SHELL_PASSWORD={} HS_BUILD_COVERITY={}{}",
                mkinfo.make_directory, make_goal,
                if makeflag.contains(MakeFlag::ENABLE_COVERITY) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::RELEASE_BUILD) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::PLEASE_WEBUI) { 0 } else { 1 },
                if makeflag.contains(MakeFlag::ENABLE_SHELL_PASSWORD) { 1 } else { 0 },
                imageserver.map_or(String::new(), |v| format!(" OS_IMAGE_FTP_IP={}", v)),
            );
            compile_infos.push(CompileInfo {
                product_name: product.long_name.clone(),
                product_model: product.model.clone(),
                product_family: product.family.clone(),
                platform_model: mkinfo.platform_model.clone(),
                make_goal,
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
        "Goal",
        "Directory",
        "Command",
    ])?;
    for info in infos.iter() {
        writer.write_record([
            &info.product_name,
            &info.product_model,
            &info.product_family,
            &info.platform_model,
            &info.make_goal,
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
            "Goal": item.make_goal,
            "Directory": item.make_directory,
            "Command": item.make_command,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&output)?);

    anyhow::Ok(())
}

fn dump_list(compile_infos: &[CompileInfo]) -> anyhow::Result<()> {
    // Style control
    let decor_width = terminal::window_size()?.columns;

    if compile_infos.is_empty() {
        println!("No matched info.");
        return anyhow::Ok(());
    }

    // Decorations
    let outer_decor = format!(
        "{}{}{:#}",
        COLOR_ANSI_GRN,
        "=".repeat(decor_width as usize),
        COLOR_ANSI_GRN
    );
    let inner_decor = format!(
        "{}{}{:#}",
        COLOR_ANSI_GRN,
        "-".repeat(decor_width as usize),
        COLOR_ANSI_GRN
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
        println!("Goal      : {}", item.make_goal);
        println!("Directory : {}", item.make_directory);
        println!("Command   : {}", item.make_command);

        if idx < compile_infos.len() - 1 {
            println!("{}", inner_decor);
        }
    }
    println!("{}", outer_decor);

    println!(
        r#"{}Run make command under the project root, i.e. "{}"{:#}"#,
        COLOR_ANSI_YLW,
        utils::SvnInfo::new()?.working_copy_root_path().display(),
        COLOR_ANSI_YLW
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
        "Goal",
        "Directory",
        "Command",
    ])?;
    for info in infos.iter() {
        writer.write_record([
            &info.product_name,
            &info.product_model,
            &info.product_family,
            &info.platform_model,
            &info.make_goal,
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
