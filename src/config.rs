use std::fs;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::utils::SvnInfo;

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
}

impl MkinfoConf {
    #[allow(dead_code)]
    pub(crate) fn new() -> MkinfoConf {
        MkinfoConf { image_server: None }
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
    pub(crate) fn new() -> RuaConf {
        RuaConf {
            clean: None,
            mkinfo: None,
            review: None,
            compdb: None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn load() -> Result<Option<RuaConf>> {
        let svninfo = SvnInfo::new()?;

        let proj_conf_file = svninfo.working_copy_root_path().join(".rua/config.toml");
        if proj_conf_file.is_file() {
            return Ok(toml::from_str(
                &fs::read_to_string(proj_conf_file.as_path())
                    .context(format!("Can not read: {}", proj_conf_file.display()))?,
            )
            .context(format!(
                "Failed to parse config file: {}",
                proj_conf_file.display()
            ))?);
        }

        let user_conf_file = home::home_dir()
            .context("Unable to get home directory")?
            .join(".config/rua/config.toml");
        if user_conf_file.is_file() {
            return Ok(toml::from_str(
                &fs::read_to_string(user_conf_file.as_path())
                    .context(format!("Can not read: {}", user_conf_file.display()))?,
            )
            .context(format!(
                "Failed to parse file: {}",
                user_conf_file.display()
            ))?);
        }

        Ok(None)
    }
}
