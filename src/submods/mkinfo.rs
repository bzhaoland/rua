use std::fmt::Display;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::{fs::File, str::FromStr};

use anyhow::{anyhow, bail, Context, Ok, Result};
use bitflags::bitflags;
use chrono::Local;
use clap::ValueEnum;
use console::{Style, Term};
use regex::Regex;
use serde_json::{json, Value};

use crate::utils;

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MakeFlag: u8 {
        const BUILD_R  = 0b00000001;
        const INET_V6  = 0b00000010;
        const WITH_UI  = 0b00000100;
        const WITH_PW  = 0b00001000;
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

impl Display for DumpFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DumpFormat::Csv => write!(f, "Csv"),
            DumpFormat::Json => write!(f, "Json"),
            DumpFormat::List => write!(f, "List"),
            DumpFormat::Tsv => write!(f, "Tsv"),
        }
    }
}

/// Structure to hold product information.
#[derive(Debug)]
#[allow(dead_code)]
struct ProdInfo {
    plat_model: String,
    prod_model: String,
    name_id: usize,
    oem_id: String,
    family: String,
    prod_shortname: String,
    prod_longname: String,
    prod_descr: String,
    snmp_oid: String,
    icon_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct MakeInfo {
    plat_model: String,
    make_goal: String,
    make_dirc: String,
}

#[derive(Debug)]
pub struct PrintInfo {
    prod_name: String,
    plat_model: String,
    make_goal: String,
    make_dirc: String,
    make_comm: String,
}

impl Display for MakeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl Display for PrintInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"CompileInfo {{
  platform_model: "{}",
  make_goal     : "{}",
  make_dirc     : "{}",
  make_comm     : "{}"
}}"#,
            self.plat_model, self.make_goal, self.make_dirc, self.make_comm,
        )
    }
}

/// Generate the make information for the given platform.
pub fn gen_mkinfo(nickname: &str, makeflag: MakeFlag) -> Result<Vec<PrintInfo>> {
    let plat_registry_file = PathBuf::from_str("./src/libplatform/hs_platform.c")
        .context(anyhow!("Error converting str to PathBuf"))?;
    let plat_mkinfo_file = PathBuf::from_str("./scripts/platform_table")
        .context(anyhow!("Error converting str to PathBuf"))?;

    // Check current working directory
    if !(plat_registry_file.is_file() && plat_mkinfo_file.is_file()) {
        bail!("Error location! Command should be run under project root.");
    }

    // Get all matched records from src/libplatform/hs_platform for the given platform name
    let platinfo_reader = BufReader::new(File::open(plat_registry_file).unwrap());
    let platinfo_pat_1 = Regex::new(
        &format!(r#"(?i)\{{\s*\w+\s*,\s*\w+\s*,\s*\d+\s*,\s*\w+\s*,\s*\w+\s*,\s*"[\w\-]+"\s*,\s*"[\w\-]+?-{}"\s*,\s*".+?"\s*,\s*"[\d.]+?"\s*,\s*(?:"(.*?)"|NULL)\s*\}}"#,
        nickname)).context("Error building regex pattern for product search with product name")?;
    let platinfo_pat_2 = Regex::new(
        r#"(?i)\{\s*(\w+)\s*,\s*(\w+)\s*,\s*(\d+)\s*,\s*(\w+)\s*,\s*(\w+)\s*,\s*"([\w\-]+)"\s*,\s*"([\w\-]+)"\s*,\s*"(.+?)"\s*,\s*"([\d.]+?)"\s*,\s*(?:"(.*?)"|NULL)\s*\}"#).context("Error building regex pattern for makeinfo search with matched platform id")?;
    let mut prods: Vec<ProdInfo> = Vec::new();
    for line in platinfo_reader.lines().map(|l| l.unwrap()) {
        if platinfo_pat_1.find(&line).is_none() {
            continue;
        }
        match platinfo_pat_2.captures(&line) {
            Some(v) => {
                prods.push(ProdInfo {
                    plat_model: v
                        .get(1)
                        .context("Error extracting the platform model part")?
                        .as_str()
                        .to_string(),
                    prod_model: v
                        .get(2)
                        .context("Error extracting the product model part")?
                        .as_str()
                        .to_string(),
                    name_id: v
                        .get(3)
                        .context("Error extracting the name part")?
                        .as_str()
                        .parse::<usize>()
                        .unwrap(),
                    oem_id: v
                        .get(4)
                        .context("Error extracting the oem-id part")?
                        .as_str()
                        .to_string(),
                    family: v
                        .get(5)
                        .context("Error extracting the product family part")?
                        .as_str()
                        .to_string(),
                    prod_shortname: v
                        .get(6)
                        .context("Error extracting the short name part")?
                        .as_str()
                        .to_string(),
                    prod_longname: v
                        .get(7)
                        .context("Error extracting the long name part")?
                        .as_str()
                        .to_string(),
                    prod_descr: v
                        .get(8)
                        .context("Error extracting the description part")?
                        .as_str()
                        .to_string(),
                    snmp_oid: v
                        .get(9)
                        .context("Error extracting the snmp-oid part")?
                        .as_str()
                        .to_string(),
                    icon_path: match v.get(10) {
                        Some(v) => {
                            if v.as_str() == "NULL" {
                                None
                            } else {
                                Some(PathBuf::from(v.as_str()))
                            }
                        }
                        None => None,
                    },
                });
            }
            None => {
                continue;
            }
        };
    }

    // Fetch makeinfo for each product
    let makeinfo_reader = BufReader::new(File::open(plat_mkinfo_file).context("File not found")?);
    let makeinfo_pat =
        Regex::new(r#"(?i)^\s*(\w+),([\w-]+),[^,]*,\s*"\s*(?:cd)?\s*([0-9A-Za-z_\-/]+)\s*","#)
            .context("Error composing a regex pattern")?;
    let mut mkinfos: Vec<MakeInfo> = Vec::new();
    for line in makeinfo_reader.lines().map(|l| l.unwrap()) {
        match makeinfo_pat.captures(&line) {
            Some(v) => {
                mkinfos.push(MakeInfo {
                    plat_model: v
                        .get(1)
                        .context("Error extracting the platform part")?
                        .as_str()
                        .to_string(),
                    make_goal: v
                        .get(2)
                        .context("Error extracting the make target part")?
                        .as_str()
                        .to_string(),
                    make_dirc: v
                        .get(3)
                        .context("Error extracting the directory part")?
                        .as_str()
                        .to_string(),
                });
            }
            None => {
                continue;
            }
        }
    }

    let mut image_name_prefix = String::from("SG6000-");

    // Extracting patterns like R10 or R10_F from branch name
    let branch_name = utils::get_svn_branch();
    let branch_nickname = match &branch_name {
        Some(name) => {
            let nickname_pattern = Regex::new(r"HAWAII_([\w-]+)")
                .context("Error building regex pattern for nickname")
                .unwrap();
            let captures = nickname_pattern.captures(name);
            match captures {
                Some(v) => v.get(1).map_or(name.to_owned(), |x| x.as_str().to_string()),
                None => name.to_owned(),
            }
        }
        None => "UB".to_string(),
    };
    image_name_prefix.push_str(&branch_nickname);

    let mut image_name_suffix = String::new();

    // When IPv6 is enabled
    if makeflag.contains(MakeFlag::INET_V6) {
        image_name_suffix.push_str("V6-");
    }

    // Build mode and date
    image_name_suffix.push(if makeflag.contains(MakeFlag::BUILD_R) {
        'r'
    } else {
        'd'
    });
    image_name_suffix.push_str(&Local::now().format("%m%d").to_string());

    // Username
    let username = utils::get_current_username().context("Failed to get username")?;
    image_name_suffix.push('-');
    image_name_suffix.push_str(&username);

    let mut printinfos: Vec<PrintInfo> = Vec::new();

    for prod in prods.iter() {
        for mkinfo in mkinfos.iter() {
            if mkinfo.plat_model != prod.plat_model {
                continue;
            }

            let mut make_goal = mkinfo.make_goal.clone();
            if makeflag.contains(MakeFlag::INET_V6) {
                make_goal.push_str("-ipv6");
            }

            let image_name_goal = mkinfo.make_goal.replace('-', "").to_uppercase();
            let image_name = format!(
                "{}-{}-{}",
                image_name_prefix, image_name_goal, image_name_suffix
            );
            let make_comm = format!(
                "hsdocker7 make -C {} -j8 {} HS_BUILD_COVERITY={} ISBUILDRELEASE={} HS_BUILD_UNIWEBUI={} HS_SHELL_PASSWORD={} IMG_NAME={} &> build.log",
                mkinfo.make_dirc, make_goal,
                if makeflag.contains(MakeFlag::COVERITY) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::BUILD_R) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::WITH_UI) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::WITH_PW) { 1 } else { 0 },
                image_name,
            );

            printinfos.push(PrintInfo {
                prod_name: prod.prod_longname.clone(),
                plat_model: mkinfo.plat_model.clone(),
                make_goal,
                make_dirc: mkinfo.make_dirc.clone(),
                make_comm,
            });
        }
    }

    Ok(printinfos)
}

/// Dump mkinfo records as csv
fn dump_csv(infos: &[PrintInfo]) -> Result<()> {
    let mut writer = csv::Writer::from_writer(std::io::stdout());

    writer.write_record(["Product", "Platform", "Target", "Path", "Command"])?;
    for info in infos.iter() {
        writer.write_record([
            info.prod_name.as_str(),
            info.plat_model.as_str(),
            info.make_goal.as_str(),
            info.make_dirc.as_str(),
            info.make_comm.as_str(),
        ])?;
    }
    writer.flush()?;

    Ok(())
}

fn dump_json(infos: &[PrintInfo]) -> Result<()> {
    let mut out: Value = json!([]);
    for info in infos.iter() {
        out.as_array_mut().unwrap().push(json!({
            "Product": info.prod_name,
            "Platform": info.plat_model,
            "Target": info.make_goal,
            "Path": info.make_dirc,
            "Command": info.make_comm,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&out)?);

    Ok(())
}

fn dump_list(infos: &[PrintInfo]) -> Result<()> {
    // Style control
    let color_grn = Style::new().green();
    let color_ylw = Style::new().yellow();
    let (_, width) = Term::stdout().size();

    if infos.is_empty() {
        println!("No matched makeinfo.");
        return Ok(());
    }
    let head_decor = "=".repeat(width as usize);
    let data_decor = "-".repeat(width as usize);

    let mut out = String::new();
    out.push_str(&format!(
        "{} matched makeinfo{}:\n",
        infos.len(),
        if infos.len() > 1 { "s" } else { "" }
    ));

    out.push_str(&format!("{}\n", color_grn.apply_to(&head_decor)));

    for (idx, item) in infos.iter().enumerate() {
        out.push_str(&format!(
            "Product  : {}\nPlatform : {}\nTarget   : {}\nPath     : {}\nCommand  : {}\n",
            item.prod_name, item.plat_model, item.make_goal, item.make_dirc, item.make_comm
        ));

        if idx < infos.len() - 1 {
            out.push_str(&format!("{}\n", color_grn.apply_to(&data_decor)));
        }
    }

    out.push_str(&format!("{}\n", color_grn.apply_to(&head_decor)));

    out.push_str(&format!(
        "{}\n",
        color_ylw.apply_to("Run compile command under project root, such as 'MX_MAIN'.")
    ));

    print!("{}", out);

    Ok(())
}

fn dump_tsv(infos: &[PrintInfo]) -> Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(std::io::stdout());

    writer.write_record(["Product", "Platform", "Target", "Path", "Command"])?;
    for info in infos.iter() {
        writer.write_record([
            info.prod_name.as_str(),
            info.plat_model.as_str(),
            info.make_goal.as_str(),
            info.make_dirc.as_str(),
            info.make_comm.as_str(),
        ])?;
    }
    writer.flush()?;

    Ok(())
}

/// Dump the make information to the screen.
pub fn dump_mkinfo(infos: &[PrintInfo], format: DumpFormat) -> Result<()> {
    match format {
        DumpFormat::Csv => dump_csv(infos),
        DumpFormat::Json => dump_json(infos),
        DumpFormat::List => dump_list(infos),
        DumpFormat::Tsv => dump_tsv(infos),
    }
}
