use std::fmt::Display;
use std::fs;
use std::path;
use std::path::Path;

use addr2line::{self, fallible_iterator::FallibleIterator};
use anyhow::{self, Context};
use clap::ValueEnum;
use regex::Regex;
use serde_json::{self, json, Value};

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

/// Process the perf-annotated data
pub fn proc_perfanno<P: AsRef<Path>>(
    data_file: P,
    binary_file: P,
    daemon_name: &str,
) -> anyhow::Result<Value> {
    let text = fs::read_to_string(&data_file).context(anyhow::anyhow!(
        "Can't read file: {}",
        data_file.as_ref().display()
    ))?;
    let headline_pattern = Regex::new(
        r#"Samples[[:blank:]]*\|[[:blank:]]*.*?of (.*?) for.*?\(([[:digit:]]+)[[:blank:]]*samples"#,
    )
    .context("Failed to build pattern for headline")?;
    let dataline_pattern = Regex::new(r#"([[:digit:]]+)[[:blank:]]*:[[:blank:]]*([[:alnum:]]+)[[:blank:]]*:[[:blank:]]*(.*?)[[:blank:]]*$"#).context("Failed to build pattern for dataline")?;
    let mut json_data = json!({
        "counter": 0,
        "mods": {},
        "num_funcs": 0,
        "num_lines": 0,
        "num_mods": 0,
    });
    let mut curr_modkey: Option<String> = None;

    // Extract data from profiling output
    for line in text.lines() {
        // Check whether is a header line
        if let Some(captures) = headline_pattern.captures(line) {
            curr_modkey = Some(captures.get(1).unwrap().as_str().to_string());
            let counter: u64 = captures.get(2).unwrap().as_str().parse()?;

            // Top-level data
            json_data["num_funcs"] = json!(
                json_data["num_funcs"]
                    .as_u64()
                    .context("Can't cast as u64")?
                    + 1u64
            );
            json_data["counter"] = json!(
                json_data["counter"]
                    .as_u64()
                    .context("Can't cast as u64")?
                    + counter
            );
            if json_data["mods"]
                .get(curr_modkey.as_ref().context("None is encountered")?)
                .is_none()
            {
                json_data["mods"]
                    .as_object_mut()
                    .context("Can't cast as mutable object")?
                    .insert(
                        curr_modkey.as_ref().context("None is encountered")?.clone(),
                        json!({ "counter": 0, "funcs": [], "num_funcs": 0, "num_lines": 0 }),
                    );
                json_data["num_mods"] = json!(
                    json_data["num_mods"]
                        .as_u64()
                        .context("Can't cast as u64")?
                        + 1u64
                );
            }

            // mod-level data
            let curr_modval = json_data["mods"]
                [curr_modkey.as_ref().context("None is encountered")?]
            .as_object_mut()
            .context("Can't cast as object")?;
            curr_modval["counter"] = json!(
                curr_modval["counter"]
                    .as_u64()
                    .context("Can't cast as u64")?
                    + counter
            );
            curr_modval["num_funcs"] = json!(
                curr_modval["num_funcs"]
                    .as_u64()
                    .context("Can't cast as u64")?
                    + 1u64
            );
            curr_modval["funcs"]
                .as_array_mut()
                .context("Can't cast as mutable array")?
                .push(json!({"counter": 0, "lines": []}));
        } else if let Some(captures) = dataline_pattern.captures(line) {
            let counter: u64 = captures.get(1).unwrap().as_str().parse()?;
            let address = captures.get(2).unwrap().as_str();
            let instruction = captures.get(3).unwrap().as_str();

            // top-level data
            json_data["num_lines"] = json!(
                json_data["num_lines"]
                    .as_u64()
                    .context("Can't cast as u64")?
                    + 1u64
            );

            // mod-level data
            let curr_modval = json_data["mods"]
                [curr_modkey.as_ref().context("None is encountered")?]
            .as_object_mut()
            .context("Not an object")?;
            curr_modval["num_lines"] = json!(
                curr_modval["num_lines"]
                    .as_u64()
                    .context("Can't cast as u64")?
                    + 1u64
            );

            // func-level data
            let num_funcs = curr_modval["funcs"]
                .as_array()
                .context("Can't cast as array")?
                .len();
            let curr_func = curr_modval["funcs"]
                .get_mut(num_funcs - 1)
                .context("Failed to get the value at the index")?
                .as_object_mut()
                .context("Can't cast as mutable object")?;
            curr_func["counter"] = json!(
                curr_func["counter"]
                    .as_u64()
                    .context("Can't cast as u64")?
                    + counter
            );
            let curr_lines = curr_func["lines"]
                .as_array_mut()
                .context("Can't cast as mutable array")?;
            curr_lines.push(json!({
                "address": address,
                "counter": counter,
                "instruction": instruction,
                "frames": json!([])
            }));
        }
    }

    // Get function name and location for each line
    let loader = addr2line::Loader::new(binary_file.as_ref().as_os_str())
        .expect("Failed to create the loader");
    let modval = json_data["mods"]
        .get_mut(daemon_name)
        .context("Daemon not found")?;
    let funcs = modval["funcs"]
        .as_array_mut()
        .context("Can't cast as mutable array")?;
    for func in funcs
        .iter_mut()
        .map(|x| x.as_object_mut().expect("Can't cast as mutable object"))
    {
        let lines = func["lines"]
            .as_array_mut()
            .context("Can't cast as mutable array")?;
        for line in lines
            .iter_mut()
            .map(|x| x.as_object_mut().expect("Can't cast as mutable object"))
        {
            let addr =
                u64::from_str_radix(line["address"].as_str().context("Can't cast as &str")?, 16)
                    .context("Can't convert address string into u64")?;
            for item in loader
                .find_frames(addr)
                .expect("Frames not found for the given address")
                .iterator()
            {
                let frame = item?;
                let funcname = frame
                    .function
                    .map_or("??".to_string(), |x| x.name.to_string_lossy().to_string());
                let location = frame.location.map_or("?:?".to_string(), |x| {
                    format!(
                        "{}:{}",
                        path::Path::new(x.file.expect("Failed to get source file"))
                            .file_name()
                            .expect("File path terminates in ..")
                            .to_str()
                            .expect("Invalid UTF-8 encoded string"),
                        x.line.expect("Failed to get line number")
                    )
                });
                line["frames"]
                    .as_array_mut()
                    .context("Can't cast as mutable array")?
                    .push(json!({ "funcname": funcname, "location": location }));
            }
        }
    }

    Ok(json_data)
}

pub fn dump_perfdata(data: &Value, format: DumpFormat) -> anyhow::Result<()> {
    match format {
        DumpFormat::Json => {
            // json
            println!(
                "{}",
                serde_json::to_string_pretty(data).context("Failed to prettify JSON string")?
            );
            Ok(())
        }
        DumpFormat::Table => {
            // table
            let summary_decor: String = "S".repeat(100);
            let spacer: String = " ".repeat(6);
            let top_counter = data["counter"]
                .as_u64()
                .context("Can't cast as u64")?;
            let top_num_mods = data["num_mods"]
                .as_u64()
                .context("Can't cast as u64")?;
            let top_num_funcs = data["num_funcs"]
                .as_u64()
                .context("Can't cast as u64")?;
            let top_num_lines = data["num_lines"]
                .as_u64()
                .context("Can't cast as u64")?;

            // Print text title
            println!("{}", summary_decor);
            println!(
                "[SUMMARY]{0}Samples:{1}{0}Daemons:{2}{0}Funcs:{3}{0}Lines:{4}",
                spacer, top_counter, top_num_mods, top_num_funcs, top_num_lines,
            );
            println!("{}", summary_decor);
            print!("\n\n");

            let module_decor: String = "M".repeat(100);
            let mut mod_count: usize = 0;
            for (modk, modv) in data["mods"]
                .as_object()
                .context("Can't cast as object")?
                .iter()
            {
                mod_count += 1;
                let mod_counter = modv["counter"]
                    .as_u64()
                    .context("Can't cast as u64")?;
                let mod_num_funcs = modv["num_funcs"]
                    .as_u64()
                    .context("Can't cast as u64")?;
                let mod_num_lines = modv["num_lines"]
                    .as_u64()
                    .context("Can't cast as u64")?;

                // Module-level title
                println!("{}", module_decor);
                println!(
                    "[{0}]{spacer}Percent:{1:.2}%{spacer}Samples:{2}{spacer}Funcs:{3}{spacer}Lines:{4}",
                    modk,
                    mod_counter as f64 / top_counter as f64 * 100f64,
                    format_args!("{}/{}", mod_counter, top_counter),
                    format_args!("{}/{}", mod_num_funcs, top_num_funcs),
                    format_args!("{}/{}", mod_num_lines, top_num_lines),
                );
                println!("{}\n\n", module_decor);

                let table_borderline = "=".repeat(100);
                let table_centerline: String = format!(
                    "{1:>.8}{0}{1:>.13}{0}{1:>.12}{0}{1:.30}{0}{1:.25}",
                    "-+-",
                    "-".repeat(100),
                );
                let spacer_2 = " ".repeat(3);
                for (func_idx, func) in modv["funcs"]
                    .as_array()
                    .context("Can't cast as array")?
                    .iter()
                    .enumerate()
                {
                    let func_counter = func["counter"]
                        .as_u64()
                        .context("Can't cast as u64")?;
                    let func_counter_str = format!("{}/{}", func_counter, top_counter);
                    let modfunc_str = format!("[{}][Func#{}]", modk, func_idx + 1);
                    let lines = func["lines"].as_array().context("Can't cast as array")?;

                    println!("{}", table_borderline);
                    println!(
                        "{1:>8}{0}{2:>13}{0}{3:>12.12}{0}{4:30}{0}Func&Location",
                        spacer_2, "Percent", "Samples", "Address", "Instruction",
                    );
                    println!("{}", table_centerline);
                    println!(
                        "{1:>8.4}{0}{2:>13}{0}{3:>12}{0}{4:30.30}{0}",
                        spacer_2,
                        func_counter as f64 / top_counter as f64 * 100f64,
                        func_counter_str,
                        "[TOTAL]",
                        modfunc_str,
                    );

                    for line in lines.iter() {
                        let address = line["address"]
                            .as_str()
                            .context("Can't cast as &str")?;
                        let counter = line["counter"]
                            .as_u64()
                            .context("Can't cast as u64")?;
                        let counter_str = format!("{}/{}", counter, top_counter);
                        let share = counter as f64 / top_counter as f64 * 100f64;
                        let instruction = line["instruction"]
                            .as_str()
                            .context("Can't cast as &str")?;
                        let mut location = String::new();
                        for (idx, item) in line["frames"]
                            .as_array()
                            .context("Can't cast as array")?
                            .iter()
                            .rev()
                            .enumerate()
                        {
                            let frame = item.as_object().context("Can't cast as object")?;
                            let funcname = frame["funcname"]
                                .as_str()
                                .context("Can't cast as &str")?;
                            let fileloca = frame["location"]
                                .as_str()
                                .context("Can't cast as &str")?;
                            if idx > 0 {
                                location.push_str("->");
                            }
                            location.push_str(&format!("{}@{}", funcname, fileloca));
                        }

                        println!(
                            "{1:>8.4}{0}{2:>13}{0}{3:>12}{0}{4:30.30}{0}{5}",
                            spacer_2, share, counter_str, address, instruction, location
                        );
                    }

                    println!("{}", table_borderline);

                    if func_idx < (mod_num_funcs - 1) as usize {
                        print!("\n\n");
                    }
                }

                if mod_count < top_num_mods as usize {
                    print!("\n\n");
                }
            }
            Ok(())
        }
    }
}
