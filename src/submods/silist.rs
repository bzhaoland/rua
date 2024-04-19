use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use anyhow::{Error, Result};
use colored::Colorize;
use regex::Regex;

pub fn gen_silist(product_dir: &str, make_target: &str, project_root: &str) -> Result<()> {
    let lastrules_file = "./scripts/last-rules.mk";
    let rules_file = "./scripts/rules.mk";
    let num_steps = 6usize;
    let mut curr_step;

    // Checking running directory, should run under project root
    curr_step = 1usize;
    print!("[{}/{}] CHECKING LOCATION...", curr_step, num_steps);
    io::stdout().flush()?;
    let mut location_ok = true;
    let attr = fs::metadata(lastrules_file);
    if attr.is_err() || attr.unwrap().is_file() != true {
        location_ok = false;
    }
    let attr = fs::metadata(rules_file);
    if attr.is_err() || attr.unwrap().is_file() != true {
        location_ok = false;
    }
    if location_ok == false {
        eprintln!("\r[{}/{}] CHECKING LOCATION...{}\x1B[0K", curr_step, num_steps, "FAILED".red());
        return Err(Error::msg("Run this command under project root."));
    }
    println!("\r[{}/{}] CHECKING LOCATION...{}\x1B[0K", curr_step, num_steps, "OK".green());

    // Inject hacked make rules
    curr_step = 2;
    print!("[{}/{}] INJECTING MAKEFILES...", curr_step, num_steps);
    io::stdout().flush()?;
    let recipe_pattern_c = Regex::new(r#"(\t\s*\$\(HS_CC\)\s+\$\(CFLAGS_GLOBAL_CP\)\s+\$\(CFLAGS_LOCAL_CP\)\s+-MMD\s+-c(\s+-E)?\s+-o\s+\$@\s+\$<\s*?\n?)"#).unwrap();
    let lastrules_orig = fs::read_to_string(lastrules_file)?;
    let lastrules_hack = recipe_pattern_c.replace_all(&lastrules_orig, "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(CC) $(CFLAGS_GLOBAL_CP) $(CFLAGS_LOCAL_CP) -MMD -c$2 -o $$@ $$< >>:file:>> $$<\n${1}").to_string();
    fs::write(lastrules_file, lastrules_hack)?;
    let recipe_pattern_cc = Regex::new(r#"(\t\s*\$\(COMPILE_CXX_CP_E\)(\s+-E)?\s*?\n?)"#).unwrap();
    let rules_orig = fs::read_to_string(rules_file)?;
    let rules_hack = recipe_pattern_cc.replace_all(
        &rules_orig,
        "\t##JCDB## >>:directory:>> $$(shell pwd | sed -z 's/\\n//g') >>:command:>> $$(COMPILE_CXX_CP)$2 >>:file:>> $$<\n${1}"
    ).to_string();
    fs::write(rules_file, rules_hack)?;
    println!("\r[{}/{}] INJECTING MAKEFILES...{}\x1B[0K", curr_step, num_steps, "OK".green());

    // Build the target (pseudo)
    curr_step = 3;
    print!("[{}/{}] BUILDING TARGET...", curr_step, num_steps);
    io::stdout().flush()?;
    let output = Command::new("hsdocker7")
        .args([
            "make",
            "-C",
            product_dir,
            make_target,
            "-j16",
            "-inwB",
            "HS_BUILD_COVERITY=0",
            "ISBUILDRELEASE=1",
            "HS_BUILD_UNIWEBUI=0",
            "HS_SHELL_PASSWORD=0",
            "IMG_NAME=RUAIHA",
        ])
        .output()
        .map_err(|e| {
            println!("\r[{}/{}] BUILDING TARGET...{}\x1B[0K", curr_step, num_steps, "FAILED".red());
            Error::msg(format!("Failed to execute `hsdocker7 make ...`: {}", &e.to_string()))
        })?;
    if output.status.success() != true {
        eprintln!("\r[{}/{}] BUILDING TARGET...{}\x1B[0K", curr_step, num_steps, "FAILED".red());
        return Err(Error::msg("error: failed to build target"));
    }
    println!("\r[{}/{}] BUILDING TARGET...{}\x1B[0K", curr_step, num_steps, "OK".green());

    // Restore the original make files
    curr_step = 4;
    print!("[{}/{} RESTORING MAKERULES...]", curr_step, num_steps);
    io::stdout().flush()?;
    fs::write(lastrules_file, lastrules_orig)?;
    fs::write(rules_file, rules_orig)?;
    println!("\r[{}/{}] RESTORING MAKERULES...{}\x1B[0K", curr_step, num_steps, "OK".green());

    // Parse the build log
    curr_step = 5;
    print!("[{}/{}] PARSING BUILD LOG...", curr_step, num_steps);
    io::stdout().flush()?;
    let output_str = String::from_utf8(output.stdout)?;
    let hackrule_pattern = Regex::new(
        r#"##JCDB##\s+>>:directory:>>\s+([^\n]+?)\s+>>:command:>>\s+([^\n]+?)\s+>>:file:>>\s+([^\n]+)\s*\n?"#,
    )?;
    let current_dir = env::current_dir()?;
    let project_root_on_winbuilder = PathBuf::from(project_root);
    let mut srcfiles: Vec<String> = Vec::new();
    let mut incdirs: Vec<String> = Vec::new();
    let incdir_pattern = Regex::new(r#"-I\s*(\S+)"#)?;
    for (_, [dirc, comm, file]) in hackrule_pattern.captures_iter(&output_str).map(|c| c.extract()) {
        let dirc = dirc.to_string();
        let file = PathBuf::from(&dirc).join(file).strip_prefix(&current_dir)?.to_owned();
        let file = project_root_on_winbuilder.join(file).to_string_lossy().to_string();
        srcfiles.push(file);

        for item in incdir_pattern.find_iter(comm) {
            // Check whether the incdir has already been cached
            let mut should_append = true;
            let incdir = PathBuf::from(&dirc).join(item.as_str());
            let mut tmpdir = incdir.as_path();

            while tmpdir != current_dir.as_path() {
                if incdirs.contains(&tmpdir.to_string_lossy().to_string()) {
                    should_append = false;
                    break;
                }

                let parent = tmpdir.parent();
                if parent.is_none() {
                    break;
                }

                tmpdir = parent.unwrap();
            }
            if should_append {
                let incdir_relative = incdir.strip_prefix(current_dir.as_path())?;
                let incdir_on_winbuilder = project_root_on_winbuilder.join(incdir_relative);
                incdirs.push(incdir_on_winbuilder.to_string_lossy().to_string());
            }
        }
    }
    println!("\r[{}/{}] PARSING BUILD LOG...{}\x1B[0K", curr_step, num_steps, "OK".green());

    // Generate FILELIST
    curr_step = 6;
    print!("[{}/{}] GENERATING FILELIST...", curr_step, num_steps);
    io::stdout().flush()?;
    let srcfiles_str = srcfiles.join("\r\n");
    let incdirs_str = incdirs.join("\r\n");
    let allfiles_str = format!("{}\r\n{}", srcfiles_str, incdirs_str);
    fs::write("filelist.txt", &allfiles_str)?;
    println!("\r[{}/{}] GENERATING FILELIST...{}\x1B[0K", curr_step, num_steps, "OK".green());

    Ok(())
}
