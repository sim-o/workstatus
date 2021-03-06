extern crate toml;

use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub title: String,
    pub gitlab_url: String,
    pub token: String,
    pub project: Vec<Project>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Project {
    pub title: String,
    pub name: String,
    pub ignore_users: Vec<String>,
    pub branch_users: Vec<String>,
}

pub fn read_config() -> Result<Config, Box<dyn Error>> {
    let path = Path::new("config.toml");
    let mut file = File::open(path)?;
    let mut config = String::new();
    file.read_to_string(&mut config)?;
    Ok(toml::from_str(config.as_str())?)
}
