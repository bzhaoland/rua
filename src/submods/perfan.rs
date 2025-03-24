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
    funcname: String,
    location: String,
}

#[derive(Deserialize, Serialize)]
struct ProfileInfoLine {
    counter_sample: u64, // Total number of samples of each line
    address: String,
    instruction: String,
    frames: Vec<Frame>,
}

#[derive(Deserialize, Serialize)]
struct ProfileInfoFunc {
    lines: Vec<ProfileInfoLine>,
    counter_sample: u64, // Quick access to number of samples in the function
}

#[derive(Deserialize, Serialize)]
struct ProfileInfoMod {
    funcs: Vec<ProfileInfoFunc>,
    counter_line: u64,   // Quick access to number of lines sampled in the module
    counter_sample: u64, // Total number of samples in the module
}

#[derive(Deserialize, Serialize)]
pub(crate) struct ProfileInfo {
    mods: IndexMap<String, ProfileInfoMod>,
    counter_func: u64,   // Quick access to number of functions sampled
    counter_line: u64,   // Quick access to number of lines sampled
    counter_sample: u64, // Quick access to num of samples
}

pub(crate) fn proc_perfanno<P: AsRef<Path>>(
    data_file: P,
    elfs: Vec<P>,
) -> anyhow::Result<ProfileInfo> {
    let text = fs::read_to_string(&data_file).context(anyhow::anyhow!(
        "Can't read file: {}",
        data_file.as_ref().display()
    ))?;

    // Regex pattern for headlines and datalines
    let headline_pattern = Regex::new(
        r#"Samples[[:blank:]]*\|[[:blank:]]*.*?of (.*?) for.*?\(([[:digit:]]+)[[:blank:]]*samples"#,
    )
    .context("Failed to build regex for headline")?;
    let dataline_pattern = Regex::new(r#"([[:digit:]]+)[[:blank:]]*:[[:blank:]]*([[:alnum:]]+)[[:blank:]]*:[[:blank:]]*(.*?)[[:blank:]]*$"#).context("Failed to build regex for dataline")?;

    let mut profile_info = ProfileInfo {
        counter_sample: 0,
        mods: IndexMap::new(),
        counter_func: 0,
        counter_line: 0,
    };
    let mut curr_mod = &mut ProfileInfoMod {
        funcs: Vec::new(),
        counter_line: 0,
        counter_sample: 0,
    };
    // Parsing profiling text
    for line in text.lines() {
        // Check whether is a header line
        if let Some(captures) = headline_pattern.captures(line) {
            let modk = captures.get(1).unwrap().as_str().to_string();
            let counter: u64 = captures.get(2).unwrap().as_str().parse()?;

            // Top-level data
            profile_info.counter_sample += counter;
            profile_info.counter_func += 1;

            // Mod-level data
            curr_mod = profile_info
                .mods
                .entry(modk.clone())
                .or_insert(ProfileInfoMod {
                    funcs: Vec::new(),
                    counter_line: 0,
                    counter_sample: 0,
                });
            curr_mod.counter_sample += counter;
            curr_mod.funcs.push(ProfileInfoFunc {
                counter_sample: counter,
                lines: Vec::new(),
            });
        } else if let Some(captures) = dataline_pattern.captures(line) {
            profile_info.counter_line += 1;
            let counter: u64 = captures.get(1).unwrap().as_str().parse()?;
            let address = captures.get(2).unwrap().as_str();
            let instruction = captures.get(3).unwrap().as_str();

            // Func-level
            let curr_func = curr_mod.funcs.last_mut().unwrap();
            curr_func.lines.push(ProfileInfoLine {
                counter_sample: counter,
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
        let daemon_name = elf
            .file_stem()
            .context("Failed to extract the file stem")?
            .to_string_lossy()
            .to_string();
        let module = profile_info
            .mods
            .get_mut(daemon_name.as_str())
            .context(format!("Can not find {}", daemon_name))?;
        for func in module.funcs.iter_mut() {
            for line in func.lines.iter_mut() {
                let addr = u64::from_str_radix(&line.address, 16)
                    .context("Can't convert address string into u64")?;
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
                        funcname: function_str,
                        location: location_str,
                    });
                }
            }
        }
    }

    Ok(profile_info)
}

pub(crate) fn tablize_perfdata(data: &ProfileInfo) -> Result<String> {
    let table_width = Term::stdout().size_checked().unwrap_or((24, 110)).1 as usize;
    let table_line = LINE_H.repeat(table_width);
    let col_width_addr = {
        let mut width = 0;
        for m in data.mods.values() {
            for f in m.funcs.iter() {
                for l in f.lines.iter() {
                    width = cmp::max(width, l.address.chars().count());
                }
            }
        }
        width
    };
    let col_width_count = {
        let mut width = 0;
        for m in data.mods.values() {
            for f in m.funcs.iter() {
                for l in f.lines.iter() {
                    width = cmp::max(width, l.counter_sample.to_string().chars().count());
                }
            }
        }
        width + data.counter_sample.to_string().chars().count() + 1
    };
    let mut output = String::new();

    // Print text title
    let info = format!(
        "{0}#samples:{1}{0}#daemons:{2}{0}#funcs:{3}{0}#lines:{4}{0}",
        LINE_V,
        data.counter_sample,
        data.mods.len(),
        data.counter_func,
        data.counter_line,
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
            "{0}{1}{0}percentage:{2:.2}%{0}#samples:{3}/{4}{0}#funcs:{5}/{6}{0}#lines:{7}/{8}{0}",
            LINE_V,
            modk,
            modv.counter_sample as f64 / data.counter_sample as f64 * 100f64,
            modv.counter_sample,
            data.counter_sample,
            modv.funcs.len(),
            data.counter_func,
            modv.counter_line,
            data.counter_line,
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
                    "{1:>9.4}%{0}{2:>col_width_count$}{0}{3:>col_width_addr$.col_width_addr$}{0}[{4}]\n",
                    spacer_2,
                    func.counter_sample as f64 / data.counter_sample as f64 * 100f64,
                    format!("{}/{}", func.counter_sample, data.counter_sample),
                    "",
                    modk,
                )
                .as_str(),
            );
            for line in func.lines.iter() {
                let mut location = String::new();
                for (idx, frame) in line.frames.iter().rev().enumerate() {
                    let funcname = frame.funcname.as_str();
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
                        line.counter_sample as f64 / data.counter_sample as f64 * 100f64,
                        format!("{}/{}", line.counter_sample, data.counter_sample),
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

pub(crate) fn dump_perfdata(data: &ProfileInfo, format: DumpFormat) -> Result<()> {
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
