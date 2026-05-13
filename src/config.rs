use std::path::PathBuf;

use anyhow::Context;
use config::Config;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::utils::RepoInfo;

pub(crate) const PROJ_RUA_DIR: &str = ".rua";
pub(crate) const CLANGD_CACHE: &str = ".cache";
pub(crate) const COMPDB_FILE: &str = "compile_commands.json";
pub(crate) const COMPDB_STORE: &str = ".rua/compdb.store";
pub(crate) const DEFAULT_BEAR: &str = "/devel/sw/bear/bin/bear";
pub(crate) const DEFAULT_INTERCEPT_BUILD: &str = "/devel/sw/llvm/bin/intercept-build";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CleanConf {
    pub ignores: Option<Vec<String>>,
}

impl CleanConf {
    #[allow(dead_code)]
    pub(crate) fn new() -> CleanConf {
        CleanConf { ignores: None }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MkinfoConf {
    pub(crate) image_server: Option<String>,
    pub(crate) defines: Option<IndexMap<String, String>>,
}

impl MkinfoConf {
    #[allow(dead_code)]
    pub(crate) fn new() -> MkinfoConf {
        MkinfoConf {
            image_server: None,
            defines: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ReviewConf {
    pub(crate) template_file: Option<String>,
}

impl ReviewConf {
    #[allow(dead_code)]
    pub(crate) fn new() -> ReviewConf {
        ReviewConf {
            template_file: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompdbConf {
    pub(crate) defines: Option<IndexMap<String, String>>,
    pub(crate) engine: Option<String>,
    pub(crate) bear_path: Option<String>,
    pub(crate) intercept_build_path: Option<String>,
    pub(crate) merge: Option<Vec<String>>,
}

impl CompdbConf {
    #[allow(dead_code)]
    pub(crate) fn new() -> CompdbConf {
        CompdbConf {
            defines: None,
            engine: None,
            bear_path: None,
            intercept_build_path: None,
            merge: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RuaConf {
    pub(crate) clean: Option<CleanConf>,
    pub(crate) mkinfo: Option<MkinfoConf>,
    pub(crate) review: Option<ReviewConf>,
    pub(crate) compdb: Option<CompdbConf>,
}

impl RuaConf {
    #[allow(dead_code)]
    pub(crate) fn new(repo_info: &RepoInfo) -> anyhow::Result<RuaConf> {
        let s = Config::builder()
            .add_source(
                config::File::with_name(
                    home::home_dir()
                        .context("Failed to get home dir")?
                        .join(".rua/config.toml")
                        .to_str()
                        .unwrap(),
                )
                .required(false),
            )
            .add_source(
                config::File::with_name(
                    home::home_dir()
                        .context("Failed to get home dir")?
                        .join(".config/rua/config.toml")
                        .to_str()
                        .unwrap(),
                )
                .required(false),
            )
            .add_source(
                config::File::with_name(
                    PathBuf::from(repo_info.work_dir())
                        .join(".rua/config.toml")
                        .as_path()
                        .to_str()
                        .unwrap(),
                )
                .required(false),
            )
            .build()?;

        Ok(s.try_deserialize().unwrap())
    }
}
