use std::fmt::Display;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::{fs::File, str::FromStr};

use anyhow::{anyhow, Ok, Result};
use chrono::Local;
use clap::ValueEnum;
use console::{Style, Term};
use regex::Regex;
use serde_json::{json, Value};

#[derive(Debug, PartialEq)]
pub enum InetVer {
    IPv4,
    IPv6,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
}

#[derive(Debug)]
pub struct MakeOpt {
    pub coverity: bool,
    pub inet_ver: InetVer,
    pub passwd: bool,
    pub buildmode: BuildMode,
    pub webui: bool,
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

/// Get current username through external command `id -un`.
/// Unfortunately, crates `whoami` and `users` both function uncorrectly,
/// they got nothing when call corresponding function to get current username.
/// Besides, method using `libc::getuid` + `libc::getpwid` wrapped in an unsafe
/// block functioned uncorrectly too in company's CentOS7 server. Maybe it is
/// because there is no `passwd` table available on the server.
fn get_current_username() -> Option<String> {
    let output = Command::new("id").arg("-un").output();

    if output.is_err() {
        return None;
    }

    let output = output.unwrap();
    if !output.status.success() {
        return None;
    }

    Some(
        String::from_utf8(output.stdout)
            .unwrap()
            .strip_suffix('\n')
            .unwrap()
            .to_string(),
    )
}

/// When `svn` utility is available and `svn info` ran successfully
fn get_current_branch() -> Option<String> {
    let output = Command::new("svn").arg("info").output();

    if output.is_err() {
        return None;
    }

    let output = output.unwrap();
    if !output.status.success() {
        return None;
    }

    let branch_pat = Regex::new("URL:[^\n]+/branches/([\\w\\-]+)\n").unwrap();
    let output = String::from_utf8(output.stdout).unwrap();
    let match_res = branch_pat.captures(&output);

    let captures = match_res.as_ref()?;
    let branch_fullname = captures.get(1)?.as_str().to_string();
    let pat = Regex::new(r"R\d+").unwrap();
    let ret = pat.find(&branch_fullname);
    return match ret {
        Some(v) => Some(v.as_str().to_string()),
        None => Some(branch_fullname),
    };
}

/// Generate the make information for the given platform.
pub fn gen_mkinfo(nick_name: &str, mkopt: &MakeOpt) -> Result<Vec<PrintInfo>> {
    let plat_regist_file = PathBuf::from_str("./src/libplatform/hs_platform.c").unwrap();
    let plat_mkinfo_file = PathBuf::from_str("./scripts/platform_table").unwrap();

    // Check current working directory
    if !(plat_regist_file.is_file() && plat_mkinfo_file.is_file()) {
        return Err(anyhow!(
            "Wrong location! Run this command under project root."
        ));
    }

    // Get all matched records from src/libplatform/hs_platform for the given platform name
    let platinfo_reader = BufReader::new(File::open(plat_regist_file).unwrap());
    let platinfo_pat_1 = Regex::new(
        &format!(r#"(?i)\{{\s*\w+\s*,\s*\w+\s*,\s*\d+\s*,\s*\w+\s*,\s*\w+\s*,\s*"[\w\-]+"\s*,\s*"[\w\-]+?-{}"\s*,\s*".+?"\s*,\s*"[\d.]+?"\s*,\s*(?:"(.*?)"|NULL)\s*\}}"#,
        nick_name)).unwrap();
    let platinfo_pat_2 = Regex::new(
        r#"(?i)\{\s*(\w+)\s*,\s*(\w+)\s*,\s*(\d+)\s*,\s*(\w+)\s*,\s*(\w+)\s*,\s*"([\w\-]+)"\s*,\s*"([\w\-]+)"\s*,\s*"(.+?)"\s*,\s*"([\d.]+?)"\s*,\s*(?:"(.*?)"|NULL)\s*\}"#).unwrap();
    let mut prods: Vec<ProdInfo> = Vec::new();
    for line in platinfo_reader.lines().map(|l| l.unwrap()) {
        if platinfo_pat_1.find(&line).is_none() {
            continue;
        }
        match platinfo_pat_2.captures(&line) {
            Some(v) => {
                prods.push(ProdInfo {
                    plat_model: v.get(1).unwrap().as_str().to_string(),
                    prod_model: v.get(2).unwrap().as_str().to_string(),
                    name_id: v.get(3).unwrap().as_str().parse::<usize>().unwrap(),
                    oem_id: v.get(4).unwrap().as_str().to_string(),
                    family: v.get(5).unwrap().as_str().to_string(),
                    prod_shortname: v.get(6).unwrap().as_str().to_string(),
                    prod_longname: v.get(7).unwrap().as_str().to_string(),
                    prod_descr: v.get(8).unwrap().as_str().to_string(),
                    snmp_oid: v.get(9).unwrap().as_str().to_string(),
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
    let makeinfo_reader = BufReader::new(File::open(plat_mkinfo_file).unwrap());
    let makeinfo_pat =
        Regex::new(r#"(?i)^\s*(\w+),([\w-]+),[^,]*,\s*"\s*(?:cd)?\s*([0-9A-Za-z_\-/]+)\s*","#)
            .unwrap();
    let mut mkinfos: Vec<MakeInfo> = Vec::new();
    for line in makeinfo_reader.lines().map(|l| l.unwrap()) {
        match makeinfo_pat.captures(&line) {
            Some(v) => {
                mkinfos.push(MakeInfo {
                    plat_model: v.get(1).unwrap().as_str().to_string(),
                    make_goal: v.get(2).unwrap().as_str().to_string(),
                    make_dirc: v.get(3).unwrap().as_str().to_string(),
                });
            }
            None => {
                continue;
            }
        }
    }

    let image_name_prefix = String::from("SG6000-");

    let mut image_name_suffix = get_current_branch().unwrap_or("UB".to_string());
    image_name_suffix.push('-');

    // When IPv6 is enabled
    if mkopt.inet_ver == InetVer::IPv6 {
        image_name_suffix.push_str("V6-");
    }

    // Date
    image_name_suffix.push(match &mkopt.buildmode {
        BuildMode::Debug => 'd',
        BuildMode::Release => 'r',
    });
    image_name_suffix.push_str(&Local::now().format("%m%d").to_string());

    // Username
    if let Some(username) = get_current_username() {
        image_name_suffix.push('-');
        image_name_suffix.push_str(&username);
    }

    let mut printinfos: Vec<PrintInfo> = Vec::new();

    for prod in prods.iter() {
        for mkinfo in mkinfos.iter() {
            if mkinfo.plat_model != prod.plat_model {
                continue;
            }

            let mut make_goal = mkinfo.make_goal.clone();
            if mkopt.inet_ver == InetVer::IPv6 {
                make_goal.push_str("-ipv6");
            }

            let mut image_name_goal = mkinfo.make_goal.replace('-', "").to_uppercase();
            image_name_goal.push('-');

            let image_name = format!(
                "{}{}{}",
                image_name_prefix, image_name_goal, image_name_suffix
            );
            let make_comm = format!(
                "hsdocker7 make -C {} -j8 {} HS_BUILD_COVERITY={} ISBUILDRELEASE={} HS_BUILD_UNIWEBUI={} HS_SHELL_PASSWORD={} IMG_NAME={} &> build.log",
                mkinfo.make_dirc, make_goal,
                match mkopt.coverity {
                    true  => 1,
                    false => 0,
                },
                match mkopt.buildmode {
                    BuildMode::Debug   => 0,
                    BuildMode::Release => 1,
                },
                match mkopt.webui {
                    false => 0,
                    true  => 1,
                },
                match mkopt.passwd {
                    false => 0,
                    true  => 1,
                },
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

    writer.write_record(["ProdName", "PlatModel", "MakeGoal", "MakePath", "MakeComm"])?;
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
            "ProdName": info.prod_name,
            "PlatModel": info.plat_model,
            "MakeGoal": info.make_goal,
            "MakePath": info.make_dirc,
            "MakeComm": info.make_comm,
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

    writer.write_record(["ProdName", "PlatModel", "MakeGoal", "MakePath", "MakeComm"])?;
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
