use std::{path::PathBuf, str::FromStr, sync::LazyLock};

use anyhow::Context;
use config::Config;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::utils::{SvnInfo, normalize_path};

pub(crate) const PROJ_LEVEL_RUA_DIR: LazyLock<PathBuf> = LazyLock::new(|| normalize_path(".rua"));
pub(crate) const CLANGD_CACHE: LazyLock<PathBuf> = LazyLock::new(|| normalize_path(".cache"));
pub(crate) const COMPDB_FILE: LazyLock<PathBuf> =
    LazyLock::new(|| normalize_path("compile_copmmands.json"));
pub(crate) const COMPDB_STORE: LazyLock<PathBuf> =
    LazyLock::new(|| normalize_path(".rua/compdbs.db3"));
pub(crate) const DEFAULT_BEAR: LazyLock<PathBuf> =
    LazyLock::new(|| normalize_path("/devel/sw/bear/bin/bear"));
pub(crate) const DEFAULT_INTERCEPT_BUILD: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from_str("/devel/sw/llvm/bin/intercept-build")
        .expect("Construct DEFAULT_INTERCEPT_BUILD failed")
});

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
}

impl CompdbConf {
    #[allow(dead_code)]
    pub(crate) fn new() -> CompdbConf {
        CompdbConf {
            defines: None,
            engine: None,
            bear_path: None,
            intercept_build_path: None,
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
    pub(crate) fn new() -> anyhow::Result<RuaConf> {
        let svninfo = SvnInfo::new()?;
        let s = Config::builder()
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
                    svninfo
                        .working_copy_root_path()
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
