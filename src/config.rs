use std::fs;

use anyhow::{Context, Result};
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

    #[allow(dead_code)]
    pub(crate) fn merge(mut self, other: &Self) -> Self {
        self.ignores = self.ignores.or_else(|| other.ignores.clone());
        self
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

    #[allow(dead_code)]
    pub(crate) fn merge(mut self, other: &Self) -> Self {
        self.image_server = self.image_server.or_else(|| other.image_server.clone());
        self
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

    #[allow(dead_code)]
    pub(crate) fn merge(mut self, other: &Self) -> Self {
        self.template_file = self.template_file.or_else(|| other.template_file.clone());
        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompdbConf {
    pub(crate) engine: Option<String>,
    pub(crate) bear_path: Option<String>,
    pub(crate) intercept_build_path: Option<String>,
}

impl CompdbConf {
    #[allow(dead_code)]
    pub(crate) fn new() -> CompdbConf {
        CompdbConf {
            engine: None,
            bear_path: None,
            intercept_build_path: None,
        }
    }

    pub(crate) fn merge(mut self, other: &Self) -> Self {
        self.engine = self.engine.or_else(|| other.engine.clone());
        self.bear_path = self.bear_path.or_else(|| other.bear_path.clone());
        self.intercept_build_path = self
            .intercept_build_path
            .or_else(|| other.bear_path.clone());
        self
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

    pub(crate) fn merge(mut self, other: &RuaConf) -> Result<Self> {
        if self.clean.is_none() {
            self.clean = other.clean.clone();
        } else if other.clean.is_some() {
            self.clean = Some(self.clean.unwrap().merge(other.clean.as_ref().unwrap()));
        }

        if self.mkinfo.is_none() {
            self.mkinfo = other.mkinfo.clone();
        } else if other.clean.is_some() {
            self.mkinfo = Some(self.mkinfo.unwrap().merge(other.mkinfo.as_ref().unwrap()));
        }

        if self.review.is_none() {
            self.review = other.review.clone();
        } else if other.review.is_some() {
            self.review = Some(self.review.unwrap().merge(other.review.as_ref().unwrap()));
        }

        if self.compdb.is_none() {
            self.compdb = other.compdb.clone();
        } else if other.review.is_some() {
            self.compdb = Some(self.compdb.unwrap().merge(other.compdb.as_ref().unwrap()));
        }

        Ok(self)
    }

    #[allow(dead_code)]
    pub(crate) fn load() -> Result<Option<RuaConf>> {
        let svninfo = SvnInfo::new()?;

        let proj_conf_file = svninfo.working_copy_root_path().join(".rua/config.toml");
        let user_conf_file = home::home_dir()
            .context("Unable to get home directory")?
            .join(".config/rua/config.toml");

        let proj_conf: Option<RuaConf> = if proj_conf_file.is_file() {
            toml::from_str(
                &fs::read_to_string(proj_conf_file.as_path())
                    .context(format!("Can't read file: {}", proj_conf_file.display()))?,
            )
            .context(format!(
                "Failed to parse config file: {}",
                proj_conf_file.display()
            ))?
        } else {
            None
        };
        let user_conf: Option<RuaConf> = if user_conf_file.is_file() {
            toml::from_str(
                &fs::read_to_string(user_conf_file.as_path())
                    .context(format!("Failed to read: {}", user_conf_file.display()))?,
            )
            .context(format!(
                "Failed to parse file: {}",
                user_conf_file.display()
            ))?
        } else {
            None
        };

        let conf = match (proj_conf, user_conf) {
            (Some(x), Some(y)) => Some(x.merge(&y)?),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        };

        Ok(conf)
    }
}
