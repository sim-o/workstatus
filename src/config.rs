extern crate toml;

use std::error::Error;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config<'a> {
    pub gitlab_url: &'a String,
    pub token: &'a String,
    pub project_name: &'a String,
    pub ignore_users: &'a Vec<String>,
}

pub fn read_config<'a>() -> Result<Config<'a>, Box<dyn Error>> {
    let path = Path::new("config.toml");
    let mut file = File::open(path)?;
    let mut config = String::new();
    file.read_to_string(&mut config)?;
    Ok(toml::from_str(config.as_str())?)
}