use std::path::Path;
use std::str::FromStr;
use std::{fs, path::PathBuf};

use addr2line::{self, fallible_iterator::FallibleIterator};
use anyhow::{self, Context};
use regex as re;
use serde_json::{self, json, Value};

#[allow(dead_code)]
pub enum DumpFormat {
    Table,
    Json,
}

/// Process the perf-annotated data
pub fn proc_perfanno<P: AsRef<Path>>(
    data_file: P,
    binary_file: P,
    daemon_name: &str,
) -> anyhow::Result<Value> {
    let text = fs::read_to_string(&data_file).context(anyhow::Error::msg(format!(
        "Error reading file {}",
        data_file.as_ref().to_string_lossy()
    )))?;
    let headline_pattern = re::Regex::new(r#"Samples\s*\|\s*.*?of (.*?) for.*?\((\d+)\s*samples"#)?;
    let dataline_pattern = re::Regex::new(r#"(\d+)\s*:\s*([0-9a-zA-Z]+)\s*:\s*(.*?)\s*$"#)?;
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
            curr_modkey = Some(
                captures
                    .get(1)
                    .context("Error extracting the module name part from the captures")?
                    .as_str()
                    .to_string(),
            );
            let counter: u64 = captures
                .get(2)
                .context(
                    "Error extracting the number of samples of current module from the captures",
                )?
                .as_str()
                .parse()?;

            // Top-level data
            json_data["num_funcs"] = json!(
                json_data["num_funcs"]
                    .as_u64()
                    .context("Error casting to a 64-bit unsigned integer")?
                    + 1u64
            );
            json_data["counter"] = json!(
                json_data["counter"]
                    .as_u64()
                    .context("Error casting to a 64-bit unsigned integer")?
                    + counter
            );
            if json_data["mods"]
                .get(curr_modkey.as_ref().context("None is encountered")?)
                .is_none()
            {
                json_data["mods"].as_object_mut().unwrap().insert(
                    curr_modkey.as_ref().context("None is encountered")?.clone(),
                    json!({ "counter": 0, "funcs": [], "num_funcs": 0, "num_lines": 0 }),
                );
                json_data["num_mods"] = json!(
                    json_data["num_mods"]
                        .as_u64()
                        .context("Error casting to a 64-bit unsigned integer")?
                        + 1u64
                );
            }

            // mod-level data
            let curr_modval = json_data["mods"]
                [curr_modkey.as_ref().context("None is encountered")?]
            .as_object_mut()
            .context("Error casting to an object")?;
            curr_modval["counter"] = json!(
                curr_modval["counter"]
                    .as_u64()
                    .context("Error casting to a 64-bit unsigned integer")?
                    + counter
            );
            curr_modval["num_funcs"] = json!(
                curr_modval["num_funcs"]
                    .as_u64()
                    .context("Error casting to a 64-bit unsigned integer")?
                    + 1u64
            );
            curr_modval["funcs"]
                .as_array_mut()
                .expect("Error casting to a mutable array")
                .push(json!({"counter": 0, "lines": []}));
        } else if let Some(captures) = dataline_pattern.captures(line) {
            let counter: u64 = captures
                .get(1)
                .context("Error extracting the counter part from the captures")?
                .as_str()
                .parse()?;
            let address = captures
                .get(2)
                .context("Error extracting the address part from the captures")?
                .as_str();
            let instruction = captures
                .get(3)
                .context("Error extracting the instruction part from the captures")?
                .as_str();

            // top-level data
            json_data["num_lines"] = json!(
                json_data["num_lines"]
                    .as_u64()
                    .context("Error casting to an 64-bit unsigned integer")?
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
                    .context("Error casting to an 64-bit unsigned integer")?
                    + 1u64
            );

            // func-level data
            let num_funcs = curr_modval["funcs"]
                .as_array()
                .context("Error casting to an array")?
                .len();
            let curr_func = curr_modval["funcs"]
                .get_mut(num_funcs - 1)
                .context("Error getting the value at the index")?
                .as_object_mut()
                .context("Error casting to a mutable object")?;
            curr_func["counter"] = json!(
                curr_func["counter"]
                    .as_u64()
                    .context("Error casting to an 64-bit unsigned integer")?
                    + counter
            );
            let curr_lines = curr_func["lines"]
                .as_array_mut()
                .context("Error casting to an array")?;
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
        .expect("Error creating the loader");
    let modval = json_data["mods"]
        .get_mut(daemon_name)
        .context("Daemon not found")?;
    let funcs = modval["funcs"]
        .as_array_mut()
        .context("Error casting to a mutable array")?;
    for func in funcs.iter_mut().map(|x| {
        x.as_object_mut()
            .expect("Error casting to a mutable object")
    }) {
        let lines = func["lines"]
            .as_array_mut()
            .context("Error casting a mutable array")?;
        for line in lines.iter_mut().map(|x| {
            x.as_object_mut()
                .expect("Error casting to a mutable object")
        }) {
            let addr = u64::from_str_radix(
                line["address"]
                    .as_str()
                    .context("Error casting to a string slice")?,
                16,
            )
            .context("Error converting string to a 64-bit unsigned integer")?;
            println!("{:?}", addr);
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
                        PathBuf::from_str(x.file.expect("Source file not found"))
                            .expect("Error creating a PathBuf from a string slice")
                            .file_name()
                            .expect("File path terminates in ..")
                            .to_str()
                            .expect("Invalid UTF-8 encoded string"),
                        x.line.expect("Line number not found")
                    )
                });
                line["frames"]
                    .as_array_mut()
                    .context("Error casting to a mutable array")?
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
                serde_json::to_string_pretty(data)
                    .context("Error serializing to a prettyt-printed string")?
            );
            Ok(())
        }
        DumpFormat::Table => {
            // table
            let decor: String = "#".repeat(100);
            let long_pad: String = " ".repeat(8);
            let top_counter = data["counter"]
                .as_u64()
                .context("Error casting to a 64-bit unsigned integer")?;
            let top_num_mods = data["num_mods"]
                .as_u64()
                .context("Error casting to a 64-bit unsigned integer")?;
            let top_num_funcs = data["num_funcs"]
                .as_u64()
                .context("Error casting to a 64-bit unsigned integer")?;
            let top_num_lines = data["num_lines"]
                .as_u64()
                .context("Error casting to a 64-bit unsigned integer")?;

            // Print text title
            println!("{}", decor);
            println!(
                "[SUMMARY]{0}Samples:{1}{0}NumDaemons:{2}{0}NumFuncs:{3}{0}NumLines:{4}",
                long_pad, top_counter, top_num_mods, top_num_funcs, top_num_lines,
            );
            println!("{}", decor);
            print!("\n\n");

            let mod_decor: String = "M".repeat(100);
            let mut mod_count: usize = 0;
            for (modk, modv) in data["mods"]
                .as_object()
                .context("Error casting to an object")?
                .iter()
            {
                mod_count += 1;
                let mod_counter = modv["counter"]
                    .as_u64()
                    .context("Error casting to a 64-bit unsigned integer")?;
                let mod_num_funcs = modv["num_funcs"]
                    .as_u64()
                    .context("Error casting to a 64-bit unsigned integer")?;
                let mod_num_lines = modv["num_lines"]
                    .as_u64()
                    .context("Error casting to a 64-bit unsigned integer")?;

                // Module-level title
                println!("{}", mod_decor);
                println!(
                    "[{0}]{long_pad} Percent:{1:.4}%{long_pad}Samples:{2}{long_pad}NumFuncs:{3}{long_pad}NumLines:{4}",
                    modk,
                    mod_counter as f64 / top_counter as f64 * 100f64,
                    format_args!("{}/{}", mod_counter, top_counter),
                    format_args!("{}/{}", mod_num_funcs, top_num_funcs),
                    format_args!("{}/{}", mod_num_lines, top_num_lines),
                );
                println!("{}\n\n", mod_decor);

                let table_borderline = "=".repeat(100);
                let table_centerline: String = format!(
                    "{1:>.8}{0}{1:>.13}{0}{1:>.12}{0}{1:.30}{0}{1:.25}",
                    "-+-",
                    "-".repeat(100),
                );
                let short_pad = " ".repeat(3);
                for (func_idx, func) in modv["funcs"]
                    .as_array()
                    .context("Error casting to an array")?
                    .iter()
                    .enumerate()
                {
                    let func_counter = func["counter"]
                        .as_u64()
                        .context("Error casting to a 64-bit unsigned integer")?;
                    let func_counter_str = format!("{}/{}", func_counter, top_counter);
                    let modfunc_str = format!("[{}][Func#{}]", modk, func_idx + 1);
                    let lines = func["lines"]
                        .as_array()
                        .context("Error casting to an array")?;

                    println!("{}", table_borderline);
                    println!(
                        "{1:>8}{0}{2:>13}{0}{3:>12.12}{0}{4:30}{0}Func&Location",
                        short_pad, "Percent", "Samples", "Address", "Instruction",
                    );
                    println!("{}", table_centerline);
                    println!(
                        "{1:>8.4}{0}{2:>13}{0}{3:>12}{0}{4:30.30}{0}",
                        short_pad,
                        func_counter as f64 / top_counter as f64 * 100f64,
                        func_counter_str,
                        "[TOTAL]",
                        modfunc_str,
                    );

                    for line in lines.iter() {
                        let address = line["address"]
                            .as_str()
                            .context("Error casting to a string slice")?;
                        let counter = line["counter"]
                            .as_u64()
                            .context("Error casting to a 64-bit unsigned integer")?;
                        let counter_str = format!("{}/{}", counter, top_counter);
                        let share = counter as f64 / top_counter as f64 * 100f64;
                        let instruction = line["instruction"]
                            .as_str()
                            .context("Error casting to a string slice")?;
                        let mut location = String::new();
                        for (idx, item) in line["frames"]
                            .as_array()
                            .context("Error casting to an array")?
                            .iter()
                            .rev()
                            .enumerate()
                        {
                            let frame = item.as_object().unwrap();
                            let funcname = frame["funcname"].as_str().unwrap_or("??");
                            let fileloca = frame["location"].as_str().unwrap_or("?:?");
                            if idx > 1 {
                                location.push_str("->");
                            }
                            location.push_str(&format!("{}@{}", funcname, fileloca));
                        }

                        println!(
                            "{1:>8.4}{0}{2:>13}{0}{3:>12}{0}{4:30.30}{0}{5}",
                            short_pad, share, counter_str, address, instruction, location
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
