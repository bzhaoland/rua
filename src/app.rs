use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs, io};

use anstyle::{Ansi256Color, Color, Style};
use anyhow::{Result, bail};
use clap::builder::styling;
use clap::{CommandFactory, Parser, Subcommand};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use rusqlite::Connection;

use crate::cli::clean::CleanArgs;
use crate::cli::compdb::CompdbCmd;
use crate::cli::mkinfo::MkinfoArgs;
use crate::cli::perfan::PerfanArgs;
use crate::cli::review::ReviewArgs;
use crate::cli::shinit::ShinitArgs;
use crate::cli::showcc::ShowccArgs;
use crate::cli::silist::SilistArgs;
use crate::cli::update::UpdateArgs;
use crate::config::{CLANGD_CACHE, COMPDB_FILE, COMPDB_STORE, PROJ_RUA_DIR, RuaConf};
use crate::core::clean;
use crate::core::compdb::{self, CompdbEngine};
use crate::core::mkinfo::{self, GenBy, MakeOpts};
use crate::core::perfan;
use crate::core::review;
use crate::core::shinit;
use crate::core::showcc;
use crate::core::silist;
use crate::core::update;
use crate::utils;
use crate::utils::progress_bar::{TICK_CHARS, TICK_INTERVAL};

const STYLE_YELLOW_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi256(Ansi256Color(3))))
    .bold();
const STYLE_GREEN: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(2))));
const STYLE_CYAN: Style = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(6))));
const STYLES: styling::Styles = styling::Styles::styled()
    .header(STYLE_YELLOW_BOLD)
    .usage(STYLE_YELLOW_BOLD)
    .literal(STYLE_GREEN)
    .placeholder(STYLE_CYAN);

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum Comm {
    Clean(CleanArgs),

    /// Manipulate compilation database.
    Compdb {
        #[clap(subcommand)]
        compdb_comm: CompdbCmd,
    },

    /// Get all matched makeinfos for product
    ///
    /// Note: R6+ releases are supported by mkinfo.
    #[command(
        after_help = format!(r#"{0}Examples:{0:#}
  rua mkinfo A1000      # Makeinfo for A1000 without extra features
  rua mkinfo -6 A1000   # Makeinfo for A1000 with IPv6 enabled
  rua mkinfo -6w 'X\d+' # Makeinfos for X-series products with IPv6 and WebUI enabled using regex pattern
  rua mkinfo --by-target a-dnv  # Makeinfos for a-dnv target"#, STYLE_YELLOW_BOLD)
    )]
    Mkinfo(MkinfoArgs),

    /// Extensively map instructions to file locations (inline expanded)
    Perfan(PerfanArgs),

    /// Start a new review request or refresh the existing one if review-id provided
    Review(ReviewArgs),

    /// Show all possible compile commands for filename (based on compilation database)
    Showcc(ShowccArgs),

    /// Generate a filelist for Source Insight (deprecated)
    Silist(SilistArgs),

    /// Generate completion for the given shell
    #[command(after_help = format!(r#"{0}Note:{0:#}
  eval "$(rua init bash)"  # Append this line to ~/.bashrc
  eval "$(rua init zsh)"   # Append this line to ~/.zshrc"#, STYLE_YELLOW_BOLD))]
    Init(ShinitArgs),

    /// Update rua
    Update(UpdateArgs),
}

#[derive(Clone, Debug, Parser)]
#[command(
    name = "rua",
    author = "bzhao",
    version = "1.4.0",
    styles = STYLES,
    about = "A toolbox for developers of StoneOS and its derivatives",
    after_help = r#"Contact bzhao@hillstonenet.com if encountered bugs"#
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Comm,

    #[arg(short = 'd', long = "debug", help = "Enable debug option")]
    debug: bool,
}

pub(crate) fn run_app(args: &Cli) -> Result<()> {
    match args.command.clone() {
        Comm::Clean(CleanArgs { dirs, ignores }) => {
            let conf = RuaConf::new()?;
            let mut ignore_set: Vec<Regex> = Vec::new();
            ignore_set.push(Regex::new(format!("^{}$", COMPDB_FILE).as_str()).unwrap());
            ignore_set.push(Regex::new(format!("^{}$", CLANGD_CACHE).as_str()).unwrap());
            ignore_set.push(Regex::new(format!(r#"^{}(?:/.*)?$"#, PROJ_RUA_DIR).as_str()).unwrap());

            if let Some(v) = ignores {
                for item in v {
                    ignore_set.push(Regex::new(format!("^{}$", item).as_str()).unwrap());
                }
            }
            if let Some(v) = conf.clean {
                if let Some(x) = v.ignores {
                    for item in x {
                        ignore_set.push(Regex::new(format!("^{}$", item).as_str()).unwrap());
                    }
                }
            }

            clean::clean_build(dirs.as_ref(), &ignore_set)
        }
        Comm::Compdb { compdb_comm } => {
            let conf = RuaConf::new()?;
            let rua_cache = Path::new(COMPDB_STORE);
            if !rua_cache.is_file() {
                print!("The compilation database store does not exist, create it? [Y/n]: ");
                io::stdout().flush()?;
                let mut input_buf = String::new();
                io::stdin().read_line(&mut input_buf)?;
                let input = input_buf.trim();
                match input.trim().to_lowercase().as_str() {
                    "y" | "yes" | "" => {
                        fs::create_dir_all(".rua")?;
                    }
                    _ => return Ok(()),
                }
            }

            let conn = Connection::open(COMPDB_STORE)?;
            compdb::create_tables(&conn)?;

            match compdb_comm {
                CompdbCmd::Gen {
                    product_dir,
                    make_target,
                    defines,
                    engine,
                    bear_path,
                    intercept_build_path,
                    merge_seq: to_merge,
                } => {
                    let compdb_conf = conf.compdb;

                    // Get bear path from config or argument
                    let mut final_bear_path = None;
                    if let Some(v) = compdb_conf.as_ref() {
                        if let Some(x) = v.bear_path.as_ref() {
                            final_bear_path = Some(Path::new(x))
                        }
                    }
                    if let Some(v) = bear_path.as_ref() {
                        final_bear_path = Some(Path::new(v));
                    }

                    // Get intercept-build path from config or argument
                    let mut final_intercept_build_path = None;
                    if let Some(v) = compdb_conf.as_ref() {
                        if let Some(x) = v.intercept_build_path.as_ref() {
                            final_intercept_build_path = Some(Path::new(x));
                        }
                    }
                    if let Some(v) = intercept_build_path.as_ref() {
                        final_intercept_build_path = Some(Path::new(v));
                    }

                    let mut final_engine = None;
                    if let Some(v) = compdb_conf.as_ref() {
                        if let Some(x) = v.engine.as_ref() {
                            final_engine = match x.as_str() {
                                "built-in" => Some(CompdbEngine::BuiltIn),
                                "bear" => Some(CompdbEngine::Bear),
                                "intercept-build" => Some(CompdbEngine::InterceptBuild),
                                y => bail!("Invalid engine specified in config: {}", y),
                            };
                        }
                    }
                    if let Some(v) = engine {
                        final_engine = Some(v);
                    }

                    let svninfo = utils::SvnInfo::new()?;

                    // Add defines from config and cli
                    let mut defines_map: IndexMap<String, String> = IndexMap::new();
                    if let Some(c) = compdb_conf.as_ref() {
                        if let Some(x) = c.defines.as_ref() {
                            defines_map.extend(x.clone());
                        }
                    }
                    for item in defines.iter() {
                        if let Some((k, v)) = item.split_once("=") {
                            defines_map.insert(k.to_string(), v.to_string());
                        } else {
                            bail!("Invalid key-value pair: {}", item);
                        }
                    }

                    let mut merge_list: Vec<PathBuf> = Vec::new();
                    if let Some(c) = compdb_conf.as_ref() {
                        if let Some(list) = c.merge.as_ref() {
                            for item in list.iter().map(PathBuf::from) {
                                merge_list.push(item);
                            }
                        }
                    }
                    if let Some(list) = to_merge {
                        for item in list.iter().map(PathBuf::from) {
                            merge_list.push(item);
                        }
                    }

                    let compdb_options = compdb::CompdbOptions {
                        defines: defines_map,
                        engine: final_engine,
                        bear_path: final_bear_path.map(|x| x.to_path_buf()),
                        intercept_build_path: final_intercept_build_path.map(|x| x.to_path_buf()),
                        to_merge: merge_list,
                    };
                    compdb::gen_compdb(&svninfo, &product_dir, &make_target, compdb_options)?;

                    // Archive the newly generated compilation database
                    let pb = ProgressBar::no_length().with_style(ProgressStyle::with_template(
                        "Archiving the newly generated compilation database...{msg}",
                    )?);
                    pb.tick();
                    let rows = compdb::archive_compdb(
                        &conn,
                        svninfo.branch_name(),
                        svninfo.revision(),
                        make_target.as_str(),
                        "compile_commands.json",
                    )?;
                    if rows == 0 {
                        eprintln!();
                        bail!(
                            "\rFailed to archive the newly generated compilation database to store"
                        );
                    }
                    pb.finish_with_message("ok");

                    // Get the generation id and insert it into the history table
                    if let Some(generation) = compdb::get_biggest_generation(&conn)? {
                        compdb::set_current_generation(&conn, generation)?;
                    }
                    Ok(())
                }
                CompdbCmd::Ls => compdb::list_generations(&conn),
                CompdbCmd::Use { generation } => {
                    compdb::use_generation(&conn, generation)?;
                    Ok(())
                }
                CompdbCmd::Del {
                    some,
                    old,
                    new,
                    all,
                } => {
                    let mut stderr_ = io::stderr();
                    if some.is_some() {
                        let generations = some.unwrap();
                        let generations_string = generations
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<String>>()
                            .join(" ");
                        let many = generations.len() > 1;
                        eprint!(
                            "Removing generation{} {}...",
                            if many { "s" } else { "" },
                            generations_string
                        );
                        stderr_.flush()?;
                        compdb::remove_generation(&conn, compdb::DelOpt::Generations(generations))?;
                        eprintln!(
                            "\rRemoving generation{} {}...ok",
                            if many { "s" } else { "" },
                            generations_string
                        );
                    } else if old.is_some() {
                        let n = old.unwrap();
                        eprint!(
                            "Removing {} oldest generation{}...",
                            n,
                            if n > 1 { "s" } else { "" }
                        );
                        stderr_.flush()?;
                        compdb::remove_generation(&conn, compdb::DelOpt::Oldest(n))?;
                        eprintln!(
                            "\rRemoving {} oldest generation{}...ok",
                            n,
                            if n > 1 { "s" } else { "" }
                        );
                    } else if new.is_some() {
                        let n = new.unwrap();
                        eprint!(
                            "Removing {} newest generation{}...",
                            n,
                            if n > 1 { "s" } else { "" }
                        );
                        stderr_.flush()?;
                        compdb::remove_generation(&conn, compdb::DelOpt::Newest(n))?;
                        eprintln!(
                            "\rRemoving {} newest generation{}...ok",
                            n,
                            if n > 1 { "s" } else { "" }
                        );
                    } else if all {
                        eprint!("Removing all generations...");
                        stderr_.flush()?;
                        compdb::remove_generation(&conn, compdb::DelOpt::All)?;
                        eprintln!("\rRemoving all generations...ok");
                    };
                    Ok(())
                }
                CompdbCmd::Add {
                    target,
                    revision,
                    compdb_path,
                } => {
                    let svninfo = utils::SvnInfo::new()?;
                    let compdb_path = compdb_path
                        .as_ref()
                        .map_or_else(|| COMPDB_FILE, |x| x.as_str());
                    eprint!("Archiving compilation database for {}...", target);
                    io::stderr().flush()?;
                    let revision = revision.unwrap_or_else(|| svninfo.revision());
                    compdb::archive_compdb(
                        &conn,
                        svninfo.branch_name(),
                        revision,
                        target.as_str(),
                        compdb_path,
                    )?;
                    eprintln!("\rArchiving compilation database for {}...ok", target);
                    let file = Path::new(compdb_path);
                    let file_name = file.file_name();
                    let parent_dir = file.parent().unwrap();
                    let current_dir = env::current_dir().unwrap();
                    if file_name.is_some_and(|x| x == "compile_commands.json")
                        && parent_dir == current_dir
                    {
                        let generation = compdb::get_biggest_generation(&conn)?.unwrap();
                        compdb::set_current_generation(&conn, generation)?;
                    }
                    Ok(())
                }
                CompdbCmd::Merge {
                    target,
                    revision,
                    files,
                } => {
                    let pbar = ProgressBar::no_length().with_style(
                        ProgressStyle::with_template("Merging compilation databases...{msg}")?
                            .tick_chars(TICK_CHARS),
                    );
                    pbar.enable_steady_tick(TICK_INTERVAL);
                    compdb::merge_compdb(files)?;
                    pbar.finish_with_message("ok");
                    let svninfo = utils::SvnInfo::new()?;
                    let revision = revision.unwrap_or_else(|| svninfo.revision());
                    pbar.set_style(ProgressStyle::with_template(
                        "Archiving the newly generated compilation database...{msg}",
                    )?);
                    pbar.enable_steady_tick(TICK_INTERVAL);
                    compdb::archive_compdb(
                        &conn,
                        svninfo.branch_name(),
                        revision,
                        target.as_str(),
                        COMPDB_FILE,
                    )?;
                    compdb::set_current_generation(
                        &conn,
                        compdb::get_biggest_generation(&conn)?.unwrap(),
                    )?;
                    pbar.finish_with_message("ok");
                    Ok(())
                }
                CompdbCmd::Name { generation, name } => {
                    eprint!(
                        "Naming compilation database generation {} {}...",
                        generation, name
                    );
                    io::stderr().flush()?;
                    let rows = compdb::name_generation(&conn, generation, name.as_str())?;
                    if rows == 0 {
                        eprintln!(
                            "\rNaming compilation database generation {} {}...err",
                            generation, name
                        );
                        bail!("No such generation");
                    }
                    eprintln!(
                        "\rNaming compilation database generation {} {}...ok",
                        generation, name
                    );
                    Ok(())
                }
                CompdbCmd::Remark { generation, remark } => {
                    eprint!(
                        "Remarking compilation database generation {}...",
                        generation
                    );
                    io::stderr().flush()?;
                    let rows = compdb::remark_generation(&conn, generation, remark.as_str())?;
                    if rows == 0 {
                        eprintln!(
                            "\rRemarking compilation database generation {}...",
                            generation
                        );
                        bail!("No such generation");
                    }
                    eprintln!(
                        "\rRemarking compilation database generation {}...ok",
                        generation
                    );
                    Ok(())
                }
            }
        }
        Comm::Showcc(ShowccArgs { comp_unit, comp_db }) => {
            let compilation_db = match comp_db {
                Some(v) => PathBuf::from_str(v.as_str())?,
                None => PathBuf::from_str("compile_commands.json")?,
            };
            showcc::show_compile_command(comp_unit.as_str(), compilation_db.as_path())
        }
        Comm::Silist(SilistArgs { prefix }) => silist::gen_silist(&prefix),
        Comm::Mkinfo(MkinfoArgs {
            ipv6,
            coverage,
            coverity,
            password,
            debug,
            webui,
            image_server,
            bins_without_strip,
            output_format,
            by_target,
            name: product_name_or_compile_target,
        }) => {
            let conf = RuaConf::new()?;
            let mkinfo_conf = conf.mkinfo;

            let mut final_image_server = None;
            if let Some(ref v) = mkinfo_conf {
                if let Some(x) = v.image_server.as_deref() {
                    final_image_server = match x {
                        "beijing" | "bj" | "b" => Some(mkinfo::ImageServer::B),
                        "suzhou" | "sz" | "s" => Some(mkinfo::ImageServer::S),
                        other => {
                            eprintln!(
                                r#"WARNING: Invalid config item: image_server = {:?}! Falling back to "Suzhou" as image server"#,
                                other
                            );
                            Some(mkinfo::ImageServer::S)
                        }
                    };
                }
            }
            if let Some(v) = image_server {
                final_image_server = Some(v);
            }

            let mut define_map = IndexMap::new();
            if let Some(ref v) = mkinfo_conf {
                if let Some(x) = v.defines.as_ref() {
                    for (key, val) in x.clone() {
                        define_map.insert(key, val);
                    }
                }
            }

            let mut makeflag = mkinfo::MakeFlag::empty();
            if !debug {
                makeflag |= mkinfo::MakeFlag::RELEASE;
            };
            if ipv6 {
                makeflag |= mkinfo::MakeFlag::IPV6;
            }
            if webui {
                makeflag |= mkinfo::MakeFlag::WEBUI;
            }
            if password {
                makeflag |= mkinfo::MakeFlag::SHELL_PASSWORD;
            }
            if coverage {
                makeflag |= mkinfo::MakeFlag::COVERAGE;
            }
            if coverity {
                makeflag |= mkinfo::MakeFlag::COVERITY;
            }

            let makeopts = MakeOpts {
                flag: makeflag,
                image_server: final_image_server,
                nostrip_bins: bins_without_strip,
                defines: define_map,
            };

            let mkinfos = mkinfo::gen_mkinfo(
                if by_target {
                    GenBy::Target(product_name_or_compile_target)
                } else {
                    GenBy::Nickname(product_name_or_compile_target)
                },
                makeopts,
            )?;

            mkinfo::dump_mkinfo(&mkinfos, output_format)
        }
        Comm::Perfan(PerfanArgs {
            file,
            elfs,
            format: outfmt,
        }) => {
            let data = perfan::proc_perfanno(&file, elfs.iter().collect::<Vec<&PathBuf>>())?;
            perfan::dump_perfdata(&data, outfmt)
        }
        Comm::Review(ReviewArgs {
            bug_id,
            review_id,
            files,
            diff_file,
            reviewers,
            branch_name,
            repo_name,
            revisions,
            template_file,
        }) => {
            let conf = RuaConf::new()?;
            let mut final_template_file = None;
            if let Some(review_conf) = conf.review.as_ref() {
                if let Some(v) = review_conf.template_file.as_ref() {
                    final_template_file = Some(v.to_owned());
                }
            }
            if let Some(v) = template_file.as_deref() {
                final_template_file = Some(v.to_string());
            }

            let options = review::ReviewOptions {
                bug_id,
                review_id,
                files,
                diff_file,
                reviewers,
                branch_name,
                repo_name,
                revisions,
                template_file: final_template_file,
            };
            tokio::runtime::Runtime::new()?.block_on(review::review(&options))
        }
        Comm::Init(ShinitArgs { shell }) => {
            shinit::gen_completion(&mut Cli::command(), shell);
            Ok(())
        }
        Comm::Update(UpdateArgs { pin }) => update::update(pin),
    }
}
