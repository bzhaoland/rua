use std::{env, fs};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CleanConf {
    pub ignores: Option<Vec<String>>,
}

impl CleanConf {
    #[allow(dead_code)]
    pub fn new() -> CleanConf {
        CleanConf { ignores: None }
    }

    #[allow(dead_code)]
    pub fn merge(mut self, other: &Self) -> Self {
        if self.ignores.is_none() {
            self.ignores = other.ignores.clone();
        }
        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MkinfoConf {
    pub image_server: Option<String>,
}

impl MkinfoConf {
    #[allow(dead_code)]
    pub fn new() -> MkinfoConf {
        MkinfoConf { image_server: None }
    }

    #[allow(dead_code)]
    pub fn merge(mut self, other: &Self) -> Self {
        if self.image_server.is_none() {
            self.image_server = other.image_server.clone()
        }
        self
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuaConf {
    pub clean: Option<CleanConf>,
    pub mkinfo: Option<MkinfoConf>,
}

impl RuaConf {
    #[allow(dead_code)]
    pub fn new() -> RuaConf {
        RuaConf {
            clean: None,
            mkinfo: None,
        }
    }

    pub fn merge(mut self, other: &RuaConf) -> Result<Self> {
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

        Ok(self)
    }
}

pub fn load_config() -> Result<Option<RuaConf>> {
    let proj_conf_file = env::current_dir()?.join(".rua/config.toml");
    let user_conf_file = home::home_dir()
        .context("Unable to get home directory")?
        .join(".config/rua/config.toml");

    let proj_conf: Option<RuaConf> = if proj_conf_file.is_file() {
        Some(toml::from_str(&fs::read_to_string(proj_conf_file)?)?)
    } else {
        None
    };
    let user_conf: Option<RuaConf> = if user_conf_file.is_file() {
        Some(toml::from_str(&fs::read_to_string(user_conf_file)?)?)
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
