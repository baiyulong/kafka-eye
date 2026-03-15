use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub clusters: Vec<ClusterConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClusterConfig {
    pub name: String,
    pub brokers: String,
    pub auth: AuthConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum AuthConfig {
    None,
    SaslPlain {
        username: String,
        password: String,
    },
    SaslScram256 {
        username: String,
        password: String,
    },
    SaslScram512 {
        username: String,
        password: String,
    },
    Ssl {
        ca_cert: Option<String>,
        client_cert: Option<String>,
        client_key: Option<String>,
    },
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            clusters: Vec::new(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig::None
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .expect("could not determine config directory")
            .join("kafka-eye")
            .join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents =
            fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
        let config: Self =
            toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        let contents =
            toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(&path, contents)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }
}
