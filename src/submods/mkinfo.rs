use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{self, Context};
use bitflags::bitflags;
use chrono::Local;
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

/// Structure to hold product information.
#[derive(Debug)]
#[allow(dead_code)]
struct ProductInfo {
    platform_model: String,
    product_model: String,
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
            self.plat_model, self.make_goal, self.make_dirc, self.make_comm,
        )
    }
}

/// Generate the make information for the given platform.
/// This function must run under project root.
pub fn gen_mkinfo(nickname: &str, makeflag: MakeFlag) -> anyhow::Result<Vec<PrintInfo>> {
    if !utils::is_at_proj_root()? {
        anyhow::bail!("Location error! Please run under the project root.");
    }

    let plat_registry = Path::new("./src/libplatform/hs_platform.c");
    if !plat_registry.is_file() {
        anyhow::bail!(r#"File "{}"not found"#, plat_registry.to_string_lossy());
    }

    let plat_table = Path::new("./scripts/platform_table");
    if !plat_table.is_file() {
        anyhow::bail!(r#"File "{}" not found"#, plat_table.to_string_lossy());
    }

    // Find all matched record(s) from src/libplatform/hs_platform.c
    let platinfo_text = fs::read_to_string(plat_registry).context(format!(
        r#"Error reading file "{}""#,
        plat_registry.to_string_lossy()
    ))?;
    let platinfo_pattern = Regex::new(
        &format!(r#"(?im)^[[:blank:]]*\{{[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:digit:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*([[:word:]]+)[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*{})"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*"([^"]*)"[[:blank:]]*,[[:blank:]]*(?:"([^"]*)"|(NULL))[[:blank:]]*\}}[[:blank:]]*,.*$"#, nickname)).context("Error building regex pattern for platinfo")?;
    let mut products: Vec<ProductInfo> = Vec::new();
    for (
        _,
        [platmodel, prodmodel, nameid, oemid, family, shortname, longname, descr, snmpoid, mut iconpath],
    ) in platinfo_pattern
        .captures_iter(&platinfo_text)
        .map(|c| c.extract())
    {
        iconpath = iconpath.trim();
        products.push(ProductInfo {
            platform_model: platmodel.to_string(),
            product_model: prodmodel.to_string(),
            name_id: nameid.parse::<usize>()?,
            oem_id: oemid.to_string(),
            family: family.to_string(),
            shortname: shortname.to_string(),
            longname: longname.to_string(),
            snmp_descr: descr.to_string(),
            snmp_oid: snmpoid.to_string(),
            icon_path: if iconpath.is_empty() || iconpath == "NULL" {
                None
            } else {
                Some(iconpath.to_string())
            },
        })
    }

    // Fetch makeinfo for each product
    let makeinfo_text = fs::read_to_string(plat_table).context(format!(
        r#"Error reading "{}""#,
        plat_table.to_string_lossy()
    ))?;
    let makeinfo_pattern =
        Regex::new(r#"(?im)^[[:blank:]]*([[:word:]]+),([-\w]+),[^,]*,[[:blank:]]*"[[:blank:]]*(?:cd[[:blank:]]+)?([-\w/]+)",.*$"#)
            .context("Error building regex pattern for makeinfo")?;
    let mut mkinfos: Vec<MakeInfo> = Vec::new();
    for (_, [plat_model, make_goal, make_dirc]) in makeinfo_pattern
        .captures_iter(&makeinfo_text)
        .map(|c| c.extract())
    {
        mkinfos.push(MakeInfo {
            plat_model: plat_model.to_string(),
            make_goal: make_goal.to_string(),
            make_dirc: make_dirc.to_string(),
        })
    }

    let pattern_nonalnum = Regex::new(r#"[^[:alnum:]]+"#).context("Error building non-alnum regex pattern")?;
    let mut image_name_infix = String::new();

    // Extracting patterns like R10 or R10_F from branch name
    let branch_name = utils::get_svn_branch()?;
    let branch_nickname = match &branch_name {
        Some(name) => {
            let nickname_pattern = Regex::new(r"HAWAII_([\w-]+)")
                .context("Error building regex pattern for nickname")
                .unwrap();
            let captures = nickname_pattern.captures(name);
            match captures {
                Some(v) => v.get(1).map_or(name.to_owned(), |x| {
                    pattern_nonalnum.replace_all(x.as_str(), "").to_string()
                }),
                None => name.to_owned(),
            }
        }
        None => "UB".to_string(),
    };
    image_name_infix.push_str(&branch_nickname);

    let mut image_name_suffix = String::new();

    // When IPv6 is enabled
    if makeflag.contains(MakeFlag::INET_V6) {
        image_name_suffix.push_str("V6-");
    }

    // Build mode and date
    image_name_suffix.push(if makeflag.contains(MakeFlag::R_BUILD) {
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
    for prod in products.iter() {
        let prodname = pattern_nonalnum.replace_all(&prod.shortname, "");
        for mkinfo in mkinfos.iter() {
            if mkinfo.plat_model != prod.platform_model {
                continue;
            }

            let mut make_goal = mkinfo.make_goal.clone();
            if makeflag.contains(MakeFlag::INET_V6) {
                make_goal.push_str("-ipv6");
            }

            let image_name_goal = mkinfo.make_goal.replace('-', "").to_uppercase();
            let image_name = format!(
                "{}-{}-{}-{}",
                prodname, image_name_infix, image_name_goal, image_name_suffix
            );
            let make_comm = format!(
                "hsdocker7 make -C {} -j8 {} HS_BUILD_COVERITY={} ISBUILDRELEASE={} HS_BUILD_UNIWEBUI={} HS_SHELL_PASSWORD={} IMG_NAME={} &> build.log",
                mkinfo.make_dirc, make_goal,
                if makeflag.contains(MakeFlag::COVERITY) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::R_BUILD) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::WITH_UI) { 1 } else { 0 },
                if makeflag.contains(MakeFlag::WITH_PW) { 1 } else { 0 },
                image_name,
            );

            printinfos.push(PrintInfo {
                prod_name: prod.longname.clone(),
                plat_model: mkinfo.plat_model.clone(),
                make_goal,
                make_dirc: mkinfo.make_dirc.clone(),
                make_comm,
            });
        }
    }

    anyhow::Ok(printinfos)
}

/// Dump mkinfo records as csv
fn dump_csv(infos: &[PrintInfo]) -> anyhow::Result<()> {
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

    anyhow::Ok(())
}

fn dump_json(infos: &[PrintInfo]) -> anyhow::Result<()> {
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

    anyhow::Ok(())
}

fn dump_list(infos: &[PrintInfo]) -> anyhow::Result<()> {
    // Style control
    let width = terminal::window_size()?.columns;

    if infos.is_empty() {
        println!("No matched makeinfo.");
        return anyhow::Ok(());
    }
    let head_decor = "=".repeat(width as usize).dark_green().to_string();
    let data_decor = "-".repeat(width as usize).dark_green().to_string();

    let mut out = String::new();
    out.push_str(&format!(
        "{} matched makeinfo{}:\n",
        infos.len(),
        if infos.len() > 1 { "s" } else { "" }
    ));

    out.push_str(&format!("{}\n", head_decor.as_str().green()));
    for (idx, item) in infos.iter().enumerate() {
        out.push_str(&format!(
            "Product  : {}\nPlatform : {}\nTarget   : {}\nPath     : {}\nCommand  : {}\n",
            item.prod_name, item.plat_model, item.make_goal, item.make_dirc, item.make_comm
        ));

        if idx < infos.len() - 1 {
            out.push_str(&format!("{}\n", data_decor.as_str()));
        }
    }
    out.push_str(&format!("{}\n", head_decor.as_str()));

    out.push_str(
        &format!(
            r#"Run command under the project root, e.g. "{}"
"#,
            utils::get_proj_root()?.to_string_lossy()
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
