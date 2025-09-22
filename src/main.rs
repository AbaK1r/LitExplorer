// src/main.rs
mod models;
mod config;
mod file_utils;
mod yaml_parser;

use models::*;
use config::load_config;
use anyhow::Result;
use crate::file_utils::find_hparams_files;

fn main() -> Result<()> {
    // 加载配置文件
    let config = load_config("lightning_explorer.toml")?;
    println!("Configuration loaded successfully!");
    println!("Log directory: {}", config.general.log_dir);
    // 查找所有hparams.yaml文件
    let hparams_files = find_hparams_files(&config.general.log_dir, &config.general.hparams_file)?;
    println!("Found {} hparams files:", hparams_files.len());
    for (i, file_path) in hparams_files.iter().enumerate() {
        let version = file_utils::extract_version_number_safe(file_path)?;
        println!("  {}. version_{}: {}", i + 1, version, file_path.display());
    }

    Ok(())
}