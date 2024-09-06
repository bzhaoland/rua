use std::env;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};

use anyhow::{self, bail, Context};
use bitflags::bitflags;
use clap::ValueEnum;
use crossterm::{style::Stylize, terminal};
use regex::Regex;
use serde_json::{json, Value};

use crate::utils;

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MakeFlag: u8 {
        const R_BUILD  = 0b00000001;  // Release build
        const INET_V6  = 0b00000010;  // Internet v6
        const WITH_UI  = 0b00000100;  // With WebUI
        const WITH_PW  = 0b00001000;  // With password
        const COVERITY = 0b00010000;
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
    shortname: String,
    longname: String,
    snmp_descr: String,
    snmp_oid: String,
    icon_path: Option<String>,
}

#[derive(Debug)]
pub struct MakeInfo {
    platform_model: String,
    product_family: Option<String>,
    make_target: String,
    make_directory: String,
}

#[derive(Debug)]
pub struct CompileInfo {
    product_name: String,
    product_model: String,
    product_family: String,
    platform_model: String,
    make_target: String,
    make_directory: String,
    make_command: String,
}

impl fmt::Display for MakeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"MakeInfo {{
  platform_model: "{}",
  make_target: "{}",
  make_directory: "{}",
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
  make_target: "{}",
  make_directory: "{}",
  make_command: "{}"
}}"#,
            self.platform_model, self.make_target, self.make_directory, self.make_command,
        )
    }
}

/// Generate the make information for the given platform.
/// This function must run under the project root which is a valid svn repo.
pub fn gen_mkinfo(nickname: &str, makeflag: MakeFlag) -> anyhow::Result<Vec<CompileInfo>> {
    let svninfo = utils::SvnInfo::new()?;

    // Check location
    let proj_root = svninfo.working_copy_root_path();
    if env::current_dir()?.as_path() != proj_root {
        bail!(
            r#"Error location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    // Check file existence
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
                shortname: captures.get(6).unwrap().as_str().to_string(),
                longname: captures.get(7).unwrap().as_str().to_string(),
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
    let mut mkinfos: Vec<MakeInfo> = Vec::new();
    while makeinfo_reader.read_line(&mut line)? != 0 {
        if let Some(captures) = makeinfo_pattern.captures(&line) {
            mkinfos.push(MakeInfo {
                platform_model: captures.get(1).unwrap().as_str().to_string(),
                product_family: captures.get(4).map(|v| v.as_str().to_string()),
                make_target: captures.get(2).unwrap().as_str().to_string(),
                make_directory: captures.get(3).unwrap().as_str().to_string(),
            })
        }
        line.clear()
    }
    mkinfos.shrink_to_fit();

    // Compose an image name using product-series/make-target/IPv6-tag/date/username
    let mut imagename_suffix = String::with_capacity(16);

    let pattern_nonalnum =
        Regex::new(r#"[^[:alnum:]]+"#).context("Error building regex pattern for nonalnum")?;

    // Use branch name abbreviation to compose the image name
    let nickname_pattern = Regex::new(r"HAWAII_([-[:word:]]+)")
        .context("Error building regex pattern for nickname")
        .unwrap();
    let captures = nickname_pattern.captures(repo_branch);
    let branch_nickname = pattern_nonalnum
        .replace_all(
            &match captures {
                Some(v) => v
                    .get(1)
                    .map_or(repo_branch.to_owned(), |x| x.as_str().to_string()),
                None => repo_branch.to_string(),
            },
            "",
        )
        .to_string();

    // IPv6 check
    if makeflag.contains(MakeFlag::INET_V6) {
        imagename_suffix.push_str("V6-");
    }

    // Building mode
    imagename_suffix.push(if makeflag.contains(MakeFlag::R_BUILD) {
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
    for product in product_info_list.iter() {
        let imagename_prodname = pattern_nonalnum.replace_all(&product.shortname, "");
        for mkinfo in mkinfos
            .iter()
            .filter(|x| x.platform_model == product.platform_code)
            .filter(|x| {
                !has_family_field_in_plattable
                    || (x.product_family.is_some()
                        && x.product_family.as_ref().unwrap() == &product.family)
            })
        {
            let mut make_goal = mkinfo.make_target.clone();
            if makeflag.contains(MakeFlag::INET_V6) {
                make_goal.push_str("-ipv6");
            }

            let imagename_makegoal = pattern_nonalnum
                .replace_all(&mkinfo.make_target, "")
                .to_uppercase();
            let imagename = format!(
                "{}-{}-{}-{}",
                imagename_prodname, branch_nickname, imagename_makegoal, imagename_suffix
            );
            let make_comm = format!(
                "hsdocker7 make -C {} -j16 {} HS_BUILD_COVERITY={} ISBUILDRELEASE={} HS_BUILD_UNIWEBUI={} HS_SHELL_PASSWORD={} IMG_NAME={} &> build.log",
                mkinfo.make_directory, make_goal,
                if makeflag.contains(MakeFlag::COVERITY) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::R_BUILD) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::WITH_UI) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::WITH_PW) { 1 } else { 0 },
                imagename,
            );
            compile_infos.push(CompileInfo {
                product_name: product.longname.clone(),
                product_model: product.model.clone(),
                product_family: product.family.clone(),
                platform_model: mkinfo.platform_model.clone(),
                make_target: make_goal,
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
        "ProductName",
        "ProductModel",
        "ProductFamily",
        "PlatformModel",
        "MakeTarget",
        "MakeDirectory",
        "MakeCommand",
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
            "ProductName": item.product_name,
            "ProductModel": item.product_name,
            "ProductFamily": item.product_family,
            "Platform": item.platform_model,
            "MakeTarget": item.make_target,
            "MakePath": item.make_directory,
            "MakeCommand": item.make_command,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&output)?);

    anyhow::Ok(())
}

fn dump_list(compile_infos: &[CompileInfo]) -> anyhow::Result<()> {
    // Style control
    let term_cols = terminal::window_size()?.columns;

    if compile_infos.is_empty() {
        println!("No matched info.");
        return anyhow::Ok(());
    }

    // Decorations
    let outer_decor = "=".repeat(term_cols as usize).dark_green().to_string();
    let inner_decor = "-".repeat(term_cols as usize).dark_green().to_string();

    let mut stdout_lock = io::stdout().lock();
    writeln!(
        stdout_lock,
        "{} matched info{}:",
        compile_infos.len(),
        if compile_infos.len() > 1 { "s" } else { "" }
    )?;

    writeln!(stdout_lock, "{}", outer_decor)?;
    for (idx, item) in compile_infos.iter().enumerate() {
        writeln!(stdout_lock, "ProductName   : {}", item.product_name)?;
        writeln!(stdout_lock, "ProductModel  : {}", item.product_model)?;
        writeln!(stdout_lock, "ProductFamily : {}", item.product_family)?;
        writeln!(stdout_lock, "Platform      : {}", item.platform_model)?;
        writeln!(stdout_lock, "MakeTarget    : {}", item.make_target)?;
        writeln!(stdout_lock, "MakeDirectory : {}", item.make_directory)?;
        writeln!(stdout_lock, "MakeCommand   : {}", item.make_command)?;

        if idx < compile_infos.len() - 1 {
            writeln!(stdout_lock, "{}", inner_decor)?;
        }
    }
    writeln!(stdout_lock, "{}", outer_decor)?;

    writeln!(
        stdout_lock,
        r#"Run the make command under the project root, i.e. "{}""#,
        utils::SvnInfo::new()?.working_copy_root_path().display()
    )?;
    stdout_lock.flush()?;

    anyhow::Ok(())
}

fn dump_tsv(infos: &[CompileInfo]) -> anyhow::Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(std::io::stdout());

    writer.write_record([
        "ProductName",
        "ProductModel",
        "ProductFamily",
        "Platform",
        "MakeTarget",
        "MakeDirectory",
        "MakeCommand",
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
