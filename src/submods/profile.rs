use std::fs;
use std::path::Path;
use std::process;

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{self, json, Value};

#[allow(dead_code)]
pub enum DumpFormat {
    Table,
    Json,
}

/// Process the profiling result measured by cycles.
pub fn proc_perfdata<P: AsRef<Path>>(
    data_file: P,
    binary_file: P,
    daemon_name: &str,
) -> Result<Value> {
    let text = fs::read_to_string(&data_file).with_context(|| {
        format!(
            "Error reading file {}",
            data_file.as_ref().to_str().unwrap()
        )
    })?;
    let headline_pattern =
        Regex::new(r#"Samples\s*\|\s*.*?of (.*?) for.*?\((\d+)\s*samples"#).unwrap();
    let dataline_pattern = Regex::new(r#"(\d+)\s*:\s*([0-9a-zA-Z]+)\s*:\s*(.*?)\s*$"#).unwrap();
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
        if let Some(captures) = headline_pattern.captures(line) {
            curr_modkey = Some(captures.get(1).unwrap().as_str().to_string());
            let counter: u64 = captures.get(2).unwrap().as_str().parse().unwrap();

            // top-level data
            json_data["num_funcs"] = json!(json_data["num_funcs"].as_u64().unwrap() + 1u64);
            json_data["counter"] = json!(json_data["counter"].as_u64().unwrap() + counter);
            if json_data["mods"]
                .get(curr_modkey.as_ref().unwrap())
                .is_none()
            {
                json_data["mods"].as_object_mut().unwrap().insert(
                    curr_modkey.as_ref().unwrap().clone(),
                    json!({ "counter": 0, "funcs": [], "num_funcs": 0, "num_lines": 0 }),
                );
                json_data["num_mods"] = json!(json_data["num_mods"].as_u64().unwrap() + 1u64);
            }

            // mod-level data
            let curr_modval = json_data["mods"][curr_modkey.as_ref().unwrap()]
                .as_object_mut()
                .unwrap();
            curr_modval["counter"] = json!(curr_modval["counter"].as_u64().unwrap() + counter);
            curr_modval["num_funcs"] = json!(curr_modval["num_funcs"].as_u64().unwrap() + 1u64);
            curr_modval["funcs"]
                .as_array_mut()
                .unwrap()
                .push(json!({"counter": 0, "lines": []}));
        }
        if let Some(captures) = dataline_pattern.captures(line) {
            let counter: u64 = captures.get(1).unwrap().as_str().parse().unwrap();
            let address = captures.get(2).unwrap().as_str();
            let instruction = captures.get(3).unwrap().as_str();

            // top-level data
            json_data["num_lines"] = json!(json_data["num_lines"].as_u64().unwrap() + 1u64);

            // mod-level data
            let curr_modval = json_data["mods"][curr_modkey.as_ref().unwrap()]
                .as_object_mut()
                .unwrap();
            curr_modval["num_lines"] = json!(curr_modval["num_lines"].as_u64().unwrap() + 1u64);

            // func-level data
            let num_funcs = curr_modval["funcs"].as_array().unwrap().len();
            let curr_func = curr_modval["funcs"]
                .get_mut(num_funcs - 1)
                .unwrap()
                .as_object_mut()
                .unwrap();
            curr_func["counter"] = json!(curr_func["counter"].as_u64().unwrap() + counter);
            let curr_lines = curr_func["lines"].as_array_mut().unwrap();
            curr_lines.push(json!({
                "address": address,
                "counter": counter,
                "instruction": instruction,
                "location": Value::Null,
                "funcname": Value::Null,
            }));
        }
    }

    // Get function name and location for each line
    let mut addrs: Vec<String> = Vec::new();
    let modval = json_data["mods"]
        .get_mut(daemon_name)
        .expect("Daemon not found");
    let funcs = modval["funcs"].as_array_mut().unwrap();
    for func in funcs.iter_mut() {
        let lines = func["lines"].as_array_mut().unwrap();
        for line in lines.iter_mut() {
            let addr = line.as_object().unwrap()["address"]
                .as_str()
                .unwrap()
                .to_string();
            addrs.push(addr);
        }
    }

    let out = process::Command::new("addr2line")
        .arg("-Cfs")
        .arg("-e")
        .arg(binary_file.as_ref().as_os_str())
        .args(&addrs)
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut olines_iter = stdout.lines();

    for func in funcs.iter_mut() {
        let lines = func["lines"].as_array_mut().unwrap();
        for line in lines.iter_mut() {
            let funcname = olines_iter.next().unwrap();
            let location = olines_iter.next().unwrap();
            line["funcname"] = json!(funcname);
            line["location"] = json!(location);
        }
    }

    Ok(json_data)
}

pub fn dump_perfdata(data: &Value, format: DumpFormat) -> Result<()> {
    match format {
        DumpFormat::Json => {
            // json
            println!("{}", serde_json::to_string_pretty(data).unwrap());
            Ok(())
        }
        DumpFormat::Table => {
            // table
            let decor: String = "#".repeat(100);
            let long_pad: String = " ".repeat(8);
            let top_counter = data["counter"].as_u64().unwrap();
            let top_num_mods = data["num_mods"].as_u64().unwrap();
            let top_num_funcs = data["num_funcs"].as_u64().unwrap();
            let top_num_lines = data["num_lines"].as_u64().unwrap();

            // Print text title
            println!("{}\n", decor);
            println!(
                "[SUMMARY]{0}Samples:{1}{0}NumDaemons:{2}{0}NumFuncs:{3}{0}NumLines:{4}",
                long_pad, top_counter, top_num_mods, top_num_funcs, top_num_lines,
            );
            println!("{}\n\n", decor);

            let mod_decor: String = "M".repeat(100);
            let mut mod_count: usize = 0;
            for (modk, modv) in data["mods"].as_object().unwrap().iter() {
                mod_count += 1;
                let mod_counter = modv["counter"].as_u64().unwrap();
                let mod_num_funcs = modv["num_funcs"].as_u64().unwrap();
                let mod_num_lines = modv["num_lines"].as_u64().unwrap();

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
                for (func_idx, func) in modv["funcs"].as_array().unwrap().iter().enumerate() {
                    let func_counter = func["counter"].as_u64().unwrap();
                    let func_counter_str = format!("{}/{}", func_counter, top_counter);
                    let modfunc_str = format!("[{}][Func#{}]", modk, func_idx + 1);
                    let lines = func["lines"].as_array().unwrap();

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
                        let address = line["address"].as_str().unwrap();
                        let counter = line["counter"].as_u64().unwrap();
                        let counter_str = format!("{}/{}", counter, top_counter);
                        let share = counter as f64 / top_counter as f64 * 100f64;
                        let instruction = line["instruction"].as_str().unwrap();
                        let funcname = line["funcname"].as_str().unwrap_or("??");
                        let location = line["location"].as_str().unwrap_or("??:0");
                        let location = format!("{}@{}", funcname, location);

                        println!(
                            "{1:>8.4}{0}{2:>13}{0}{3:>12}{0}{4:30.30}{0}{5}",
                            short_pad, share, counter_str, address, instruction, location
                        );
                    }

                    println!("{}", table_borderline);

                    if func_idx < (mod_num_funcs - 1) as usize {
                        println!();
                        println!();
                    }
                }

                if mod_count < top_num_mods as usize {
                    println!();
                    println!();
                }
            }
            Ok(())
        }
    }
}
