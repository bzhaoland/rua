use std::cmp;
use std::fmt::Display;
use std::fs;
use std::path;
use std::path::Path;

use addr2line::{self, fallible_iterator::FallibleIterator};
use anyhow::{self, Context, Result};
use clap::ValueEnum;
use console::Term;
use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::utils::symbols::{DIAMOND, LINE_H, LINE_HD, LINE_V};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DumpFormat {
    Json,
    Table,
}

impl Display for DumpFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DumpFormat::Json => write!(f, "Json"),
            DumpFormat::Table => write!(f, "Table"),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Frame {
    function: String,
    location: String,
}

#[derive(Deserialize, Serialize)]
struct ProfileLine {
    counter_s: u64, // The number of samples for each line (instruction)
    address: String,
    instruction: String,
    frames: Vec<Frame>,
}

#[derive(Deserialize, Serialize)]
struct ProfileFunc {
    lines: Vec<ProfileLine>,
    counter_s: u64, // Short-path to get the number of samples of the function
    name: String,
}

#[derive(Deserialize, Serialize)]
struct ProfileMod {
    funcs: Vec<ProfileFunc>,
    counter_l: u64, // Short-path to get the number of lines sampled of the module
    counter_s: u64, // Short-path to get the number of samples of the module
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Profile {
    mods: IndexMap<String, ProfileMod>,
    counter_f: u64, // Short-path to get the number of functions
    counter_l: u64, // Short-path to get the number of lines
    counter_s: u64, // Short-path to get the number of samples
}

pub(crate) fn proc_perfanno<P: AsRef<Path>>(data_file: P, elfs: Vec<P>) -> anyhow::Result<Profile> {
    let text = fs::read_to_string(&data_file).context(anyhow::anyhow!(
        "Can't read file: {}",
        data_file.as_ref().display()
    ))?;

    // Regex pattern for headlines and datalines
    let regex_headerline = Regex::new(
        r#"Samples[[:blank:]]*\|[[:blank:]]*.*?of (.*?) for.*?\(([[:digit:]]+)[[:blank:]]*samples"#,
    )
    .context("Failed to build regex for headline")?;
    let regex_funcline = Regex::new(r#"[[:blank:]]+:[[:blank:]]+[[:xdigit:]]+(<[[:word:]]+>):"#)
        .context("Faild to build regex for funcline")?;
    let regex_dataline = Regex::new(r#"([[:digit:]]+)[[:blank:]]*:[[:blank:]]*([[:xdigit:]]+)[[:blank:]]*:[[:blank:]]*(.*?)[[:blank:]]*$"#).context("Failed to build regex for dataline")?;

    let mut profile = Profile {
        mods: IndexMap::new(),
        counter_f: 0,
        counter_l: 0,
        counter_s: 0,
    };
    let mut profile_mod_curr = &mut ProfileMod {
        funcs: Vec::new(),
        counter_l: 0,
        counter_s: 0,
    };
    let mut profile_func_curr = &mut ProfileFunc {
        lines: Vec::new(),
        counter_s: 0,
        name: String::new(),
    };
    // Parsing profiling text
    for line in text.lines() {
        // Check whether is a header line
        if let Some(captures) = regex_headerline.captures(line) {
            let mod_k = captures.get(1).unwrap().as_str().to_string();
            let counter: u64 = captures.get(2).unwrap().as_str().parse()?;

            // Top-level data
            profile.counter_f += 1;
            profile.counter_s += counter;

            // Module-level data
            profile_mod_curr = profile.mods.entry(mod_k.clone()).or_insert(ProfileMod {
                funcs: Vec::new(),
                counter_l: 0,
                counter_s: 0,
            });
            profile_mod_curr.counter_s += counter;
            profile_mod_curr.funcs.push(ProfileFunc {
                counter_s: counter,
                lines: Vec::new(),
                name: String::new(),
            });

            // Function reference
            profile_func_curr = profile_mod_curr.funcs.last_mut().unwrap();
        } else if let Some(captures) = regex_funcline.captures(line) {
            let funcname = captures.get(1).unwrap().as_str();
            profile_func_curr.name.push_str(funcname);
        } else if let Some(captures) = regex_dataline.captures(line) {
            profile.counter_l += 1;
            profile_mod_curr.counter_l += 1;
            let counter: u64 = captures.get(1).unwrap().as_str().parse()?;
            let address = captures.get(2).unwrap().as_str();
            let instruction = captures.get(3).unwrap().as_str();

            // Func-level
            profile_func_curr.lines.push(ProfileLine {
                counter_s: counter,
                address: address.to_string(),
                instruction: instruction.to_string(),
                frames: Vec::new(),
            });
        }
    }

    for elf in elfs.iter().map(AsRef::as_ref) {
        if !elf.is_file() {
            eprintln!("Warning: Elf {} does not exist, skipped", elf.display());
            continue;
        }
        let loader = addr2line::Loader::new(elf.as_os_str())
            .expect("Failed to create addr2line::loader object");
        let elf_name = elf
            .file_name()
            .context("Failed to extract the file stem")?
            .to_string_lossy()
            .to_string();
        let module = profile.mods.get_mut(&elf_name).context(format!(
            "Can not find {} module from profiling text",
            elf_name
        ))?;
        for func in module.funcs.iter_mut() {
            for line in func.lines.iter_mut() {
                let addr = u64::from_str_radix(&line.address, 16)
                    .context("Can't parse address string into u64")?;
                for item in loader
                    .find_frames(addr)
                    .unwrap_or_else(|_| panic!("Can not find frame by address {}", &line.address))
                    .iterator()
                {
                    let frame = item?;
                    let function = frame.function;
                    let function_str =
                        function.map_or("??".to_string(), |x| x.name.to_string_lossy().to_string());
                    let location = frame.location;
                    let location_str = location.map_or("?:?".to_string(), |x| {
                        format!(
                            "{}:{}",
                            path::Path::new(x.file.expect("Failed to get source file"))
                                .file_name()
                                .expect("File path terminates in ..")
                                .to_str()
                                .expect("Invalid UTF-8 encoded string"),
                            x.line.context("Failed to get line number").unwrap()
                        )
                    });
                    line.frames.push(Frame {
                        function: function_str,
                        location: location_str,
                    });
                }
            }
        }
    }

    Ok(profile)
}

pub(crate) fn tablize_perfdata(data: &Profile) -> Result<String> {
    let table_width = Term::stdout().size_checked().unwrap_or((24, 110)).1 as usize;
    let table_line = LINE_H.repeat(table_width);
    let (col_width_count, col_width_addr) = {
        let mut width_addr = 0;
        let mut width_count = 0;

        for m in data.mods.values() {
            for f in m.funcs.iter() {
                for l in f.lines.iter() {
                    width_addr = cmp::max(width_addr, l.address.chars().count());
                    width_count = cmp::max(width_count, l.counter_s.to_string().chars().count());
                }
            }
        }

        (
            width_count + data.counter_s.to_string().chars().count() + 1,
            width_addr,
        )
    };
    let mut output = String::new();

    // Print text title
    let info = format!(
        "{0}#samples:{1}{0}#daemons:{2}{0}#funcs:{3}{0}#lines:{4}{0}",
        LINE_V,
        data.counter_s,
        data.mods.len(),
        data.counter_f,
        data.counter_l,
    );
    let pad_width = table_width - info.chars().count();
    output.push_str(
        format!(
            "{}{}{}\n",
            LINE_HD.repeat(pad_width / 2),
            info,
            LINE_HD.repeat(pad_width - pad_width / 2)
        )
        .as_str(),
    );

    for (modk, modv) in data.mods.iter() {
        // Module-level title
        let modinfo = format!(
            "{0}{1}{0}percentage:{2:.4}%{0}#samples:{3}/{4}{0}#funcs:{5}/{6}{0}#lines:{7}/{8}{0}",
            LINE_V,
            modk,
            modv.counter_s as f64 / data.counter_s as f64 * 100f64,
            modv.counter_s,
            data.counter_s,
            modv.funcs.len(),
            data.counter_f,
            modv.counter_l,
            data.counter_l,
        );
        let rest_len = table_width - modinfo.chars().count();
        output.push_str(
            format!(
                "\n\n{}{}{}\n",
                DIAMOND.repeat(rest_len / 2),
                modinfo,
                DIAMOND.repeat(rest_len - rest_len / 2)
            )
            .as_str(),
        );

        let spacer_2 = " ".repeat(3);
        for func in modv.funcs.iter() {
            output.push_str(
                format!(
                    "\n{1:>10}{0}{2:>col_width_count$}{0}{3:>col_width_addr$.col_width_addr$}{0}{4:35}{0}Location\n{5}\n",
                    spacer_2, "Percentage", "Count", "Address", "Instruction", table_line,
                )
                .as_str(),
            );
            output.push_str(
                format!(
                    "{1:>9.4}%{0}{2:>col_width_count$}{0}{3:>col_width_addr$.col_width_addr$}{0}[{4:35}]{0}{5}\n",
                    spacer_2,
                    func.counter_s as f64 / data.counter_s as f64 * 100f64,
                    format!("{}/{}", func.counter_s, data.counter_s),
                    "",
                    modk,
                    func.name
                )
                .as_str(),
            );
            for line in func.lines.iter() {
                let mut location = String::new();
                for (idx, frame) in line.frames.iter().rev().enumerate() {
                    let funcname = frame.function.as_str();
                    let fileloca = frame.location.as_str();
                    if idx > 0 {
                        location.push_str("->");
                    }
                    location.push_str(&format!("{}@{}", funcname, fileloca));
                }
                output.push_str(
                    format!(
                        "{1:>9.4}%{0}{2:>col_width_count$}{0}{3:>col_width_addr$.col_width_addr$}{0}{4:35.35}{0}{5}\n",
                        spacer_2,
                        line.counter_s as f64 / data.counter_s as f64 * 100f64,
                        format!("{}/{}", line.counter_s, data.counter_s),
                        line.address,
                        line.instruction,
                        location
                    )
                    .as_str(),
                );
            }
        }
    }
    Ok(output)
}

pub(crate) fn dump_perfdata(data: &Profile, format: DumpFormat) -> Result<()> {
    match format {
        DumpFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(data).context("Failed to prettify JSON string")?
            );
            Ok(())
        }
        DumpFormat::Table => {
            println!("{}", tablize_perfdata(data)?);
            Ok(())
        }
    }
}
