extern crate toml;

use std::error::Error;
use std::path::Path;
use std::fs::File;
use std::io::Read;

pub struct Config {
    pub gitlab_url: String,
    pub token: String,
    pub project_name: String,
    pub ignore_users: Vec<String>,
}

pub fn read_config() -> Result<Config, Box<dyn Error>> {
    let path = Path::new("config.toml");
    let mut file = File::open(path)?;
    let mut config = String::new();
    file.read_to_string(&mut config);
    toml::from_str(config.as_str())
}