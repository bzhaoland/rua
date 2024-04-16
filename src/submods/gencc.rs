use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
use std::string::String;

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::{self, json};

pub fn gencc(logfile: &dyn AsRef<Path>) -> Result<()> {
    let cc_pat = Regex::new(r"(\b[\w-]*(?:g?cc|g++|clang|clang++)\s+[^;]+?)\s*$").unwrap();
    let obj_pat = Regex::new(r#"\s-o\s*(\S+)"#).unwrap();
    let src_pat = Regex::new(r#"\S+\.c(?:c|pp|xx)?\b"#).unwrap();
    let mut logical_line = String::new(); // logical line
    let mut jcdb = json!([]);
    let logfile = File::open(logfile)
        .with_context(|| format!("Cannot open file {}", logfile.as_ref().to_str().unwrap()))?;
    let reader = BufReader::new(logfile);

    for physical_line in reader.lines().map(|l| l.unwrap()) {
        logical_line.push_str(&physical_line);

        // Continues with the next line
        let mut continu = false;
        for c in physical_line.chars().rev() {
            if c != '\\' {
                break;
            }
            continu = !continu;
        }
        if continu {
            logical_line.pop();
            continue;
        }

        let res = cc_pat.captures(&logical_line);
        if let Some(cccap) = res {
            let ccline = cccap.get(1).unwrap().as_str();
            let objfile = match obj_pat.captures(ccline) {
                Some(v) => PathBuf::from(v.get(1).unwrap().as_str()),
                None => {
                    logical_line.clear();
                    continue;
                }
            };
            let srcfile = match src_pat.find(ccline) {
                Some(v) => PathBuf::from(v.as_str()),
                None => {
                    logical_line.clear();
                    continue;
                }
            };

            let mut skip = false;
            let mut directory = PathBuf::new();

            for component in objfile.parent().unwrap().components() {
                if skip {
                    skip = false;
                    continue;
                }
                if component == Component::Normal("target".as_ref()) {
                    skip = true; // skip next component
                    continue;
                }

                directory.push(component);
            }

            jcdb.as_array_mut().unwrap().push(json!({
                "command": ccline,
                "directory": directory.to_str(),
                "file": srcfile,
            }));
        }

        logical_line.clear();
    }

    let outfile = File::create("compile_commands.json")
        .with_context(|| "Failed to create compile_commands.json file")?;
    serde_json::to_writer_pretty(outfile, &jcdb)
        .with_context(|| "Failed to save JSON compilation database")?;

    Ok(())
}
