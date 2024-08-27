use std::env;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::bail;
use anyhow::{self, Context};
use bitflags::bitflags;
use chrono::Local;
use clap::ValueEnum;
use crossterm::{style::Stylize, terminal};
use regex::Regex;
use serde_json::{json, Value};

use crate::utils;
use crate::utils::SvnInfo;

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
    plat_model: String,
    prod_family: Option<String>,
    make_goal: String,
    make_dirc: String,
}

#[derive(Debug)]
pub struct PrintInfo {
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
  goal     : "{}",
  dirc     : "{}",
}}"#,
            self.plat_model, self.make_goal, self.make_dirc,
        )
    }
}

impl fmt::Display for PrintInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"CompileInfo {{
  platform_model: "{}",
  make_goal     : "{}",
  make_dirc     : "{}",
  make_comm     : "{}"
}}"#,
            self.platform_model, self.make_target, self.make_directory, self.make_command,
        )
    }
}

/// Generate the make information for the given platform.
/// This function must run under project root.
pub fn gen_mkinfo(nickname: &str, makeflag: MakeFlag) -> anyhow::Result<Vec<PrintInfo>> {
    let svninfo = utils::SvnInfo::new()?;

    let proj_root = Path::new(
        svninfo
            .working_copy_root_path()
            .context("Error fetching project root")?,
    );
    if env::current_dir()?.as_path() != proj_root {
        bail!(
            r#"Error location! Please run this command under the project root, i.e. "{}"."#,
            proj_root.display()
        );
    }

    let product_info_path = proj_root.join("src/libplatform/hs_platform.c");
    if !product_info_path.is_file() {
        bail!(r#"File "{}" not available"#, product_info_path.display());
    }

    let plat_table = proj_root.join("scripts/platform_table");
    if !plat_table.is_file() {
        bail!(r#"File "{}" not available"#, plat_table.display());
    }

    let repo_branch = svninfo.branch_name().context("Failed to fetch branch")?;
    let repo_revision = svninfo.revision().context("Failed to fetch revision")?;
    let newer_mkfile = (repo_branch.as_str() == "MX_MAIN" && repo_revision >= 293968)
        || (repo_branch.as_str() == "HAWAII_REL_R11" && repo_revision >= 295630);

    // Find out all matched records in src/libplatform/hs_platform.c
    let product_info_file = fs::File::open(&product_info_path).context(format!(
        "Error opening file {}",
        product_info_path.display()
    ))?;
    let product_info_reader = BufReader::with_capacity(1024 * 512, product_info_file);
    let product_info_pattern = Regex::new(&format!(r#"(?i)^[[:blank:]]*\{{[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:digit:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*{})"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*(?:"([^"]*)"|(NULL))[[:blank:]]*\}}[[:blank:]]*,.*$"#, nickname)).context("Error building regex pattern of product info")?;
    let mut products: Vec<ProductInfo> = Vec::new();
    for line in product_info_reader.lines() {
        let line = line.context("Error reading product inventory")?;
        if let Some(captures) = product_info_pattern.captures(&line) {
            products.push(ProductInfo {
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
    }

    // Fetch makeinfo for each product
    let makeinfo_text = fs::read_to_string(&plat_table)
        .context(format!(r#"Error reading "{}""#, plat_table.display()))?;
    let makeinfo_pattern =
        Regex::new(r#"(?m)^[[:blank:]]*([[:word:]]+),([-[:word:]]+),[^,]*,[[:blank:]]*"[[:blank:]]*(?:cd[[:blank:]]+)?([-[:word:]/]+)",[[:space:]]*[[:digit:]]+(?:[[:space:]]*,[[:space:]]*([[:word:]]+))?.*$"#)
            .context("Error building regex pattern for makeinfo")?;
    let mut mkinfos: Vec<MakeInfo> = Vec::new();
    for item in makeinfo_pattern.captures_iter(&makeinfo_text) {
        mkinfos.push(MakeInfo {
            plat_model: item.get(1).unwrap().as_str().to_string(),
            prod_family: item.get(4).map(|v| v.as_str().to_string()),
            make_goal: item.get(2).unwrap().as_str().to_string(),
            make_dirc: item.get(3).unwrap().as_str().to_string(),
        })
    }

    // Normalize the image name
    let pattern_nonalnum =
        Regex::new(r#"[^[:alnum:]]+"#).context("Error building non-alnum regex pattern")?;
    let mut imagename_infix = String::new();

    // Extracting patterns like R10 or R10_F from branch name
    let branch_name = &repo_branch;
    let nickname_pattern = Regex::new(r"HAWAII_([-[:word:]]+)")
        .context("Error building regex pattern for nickname")
        .unwrap();
    let captures = nickname_pattern.captures(branch_name);
    let branch_nickname = pattern_nonalnum
        .replace_all(
            &match captures {
                Some(v) => v
                    .get(1)
                    .map_or(branch_name.to_owned(), |x| x.as_str().to_string()),
                None => branch_name.to_string(),
            },
            "",
        )
        .to_string();
    imagename_infix.push_str(&branch_nickname);

    let mut imagename_suffix = String::new();

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
    imagename_suffix.push_str(&Local::now().format("%m%d").to_string());

    // Username
    let username = utils::get_current_username().context("Failed to get username")?;
    imagename_suffix.push('-');
    imagename_suffix.push_str(&username);

    let mut printinfos: Vec<PrintInfo> = Vec::new();
    for product in products.iter() {
        let imagename_prodname = pattern_nonalnum.replace_all(&product.shortname, "");
        for mkinfo in mkinfos
            .iter()
            .filter(|x| x.plat_model == product.platform_code)
            .filter(|x| {
                !newer_mkfile
                    || (x.prod_family.is_some()
                        && x.prod_family.as_ref().unwrap() == &product.family)
            })
        {
            let mut make_goal = mkinfo.make_goal.clone();
            if makeflag.contains(MakeFlag::INET_V6) {
                make_goal.push_str("-ipv6");
            }

            let imagename_makegoal = pattern_nonalnum
                .replace_all(&mkinfo.make_goal, "")
                .to_uppercase();
            let imagename = format!(
                "{}-{}-{}-{}",
                imagename_prodname, imagename_infix, imagename_makegoal, imagename_suffix
            );
            let make_comm = format!(
                "hsdocker7 make -C {} -j16 {} HS_BUILD_COVERITY={} ISBUILDRELEASE={} HS_BUILD_UNIWEBUI={} HS_SHELL_PASSWORD={} IMG_NAME={} &> build.log",
                mkinfo.make_dirc, make_goal,
                if makeflag.contains(MakeFlag::COVERITY) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::R_BUILD) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::WITH_UI) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::WITH_PW) { 1 } else { 0 },
                imagename,
            );
            printinfos.push(PrintInfo {
                product_name: product.longname.clone(),
                product_model: product.model.clone(),
                product_family: product.family.clone(),
                platform_model: mkinfo.plat_model.clone(),
                make_target: make_goal,
                make_directory: mkinfo.make_dirc.clone(),
                make_command: make_comm,
            });
        }
    }

    anyhow::Ok(printinfos)
}

/// Dump mkinfo records as csv
fn dump_csv(infos: &[PrintInfo]) -> anyhow::Result<()> {
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

fn dump_json(printinfos: &[PrintInfo]) -> anyhow::Result<()> {
    let mut out: Value = json!([]);
    for printinfo in printinfos.iter() {
        out.as_array_mut().unwrap().push(json!({
            "ProductName": printinfo.product_name,
            "ProductModel": printinfo.product_name,
            "ProductFamily": printinfo.product_family,
            "Platform": printinfo.platform_model,
            "MakeTarget": printinfo.make_target,
            "MakePath": printinfo.make_directory,
            "MakeCommand": printinfo.make_command,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&out)?);

    anyhow::Ok(())
}

fn dump_list(printinfos: &[PrintInfo]) -> anyhow::Result<()> {
    // Style control
    let width = terminal::window_size()?.columns;

    if printinfos.is_empty() {
        println!("No matched makeinfo.");
        return anyhow::Ok(());
    }

    // Decorations
    let head_decor = "=".repeat(width as usize).dark_green().to_string();
    let data_decor = "-".repeat(width as usize).dark_green().to_string();

    let mut out = String::new();
    out.push_str(&format!(
        "{} matched makeinfo{}:\n",
        printinfos.len(),
        if printinfos.len() > 1 { "s" } else { "" }
    ));

    out.push_str(&format!("{}\n", head_decor));
    for (idx, item) in printinfos.iter().enumerate() {
        out.push_str(&format!(
            "ProductName   : {}\nProductModel  : {}\nProductFamily : {}\nPlatform      : {}\nMakeTarget    : {}\nMakeDirectory : {}\nMakeCommand   : {}\n",
            item.product_name, item.product_model, item.product_family, item.platform_model, item.make_target, item.make_directory, item.make_command
        ));

        if idx < printinfos.len() - 1 {
            out.push_str(&format!("{}\n", data_decor));
        }
    }
    out.push_str(&format!("{}\n", head_decor));

    out.push_str(
        &format!(
            r#"Run the make command under the project root, i.e. "{}"
"#,
            SvnInfo::new()?.working_copy_root_path().unwrap()
        )
        .dark_yellow()
        .to_string(),
    );

    print!("{}", out);

    anyhow::Ok(())
}

fn dump_tsv(infos: &[PrintInfo]) -> anyhow::Result<()> {
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
pub fn dump_mkinfo(infos: &[PrintInfo], format: DumpFormat) -> anyhow::Result<()> {
    match format {
        DumpFormat::Csv => dump_csv(infos),
        DumpFormat::Json => dump_json(infos),
        DumpFormat::List => dump_list(infos),
        DumpFormat::Tsv => dump_tsv(infos),
    }
}
