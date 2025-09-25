use crate::models::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_config(config_path: &str) -> Result<Config> {
    // 检查配置文件是否存在，如果不存在则创建默认配置
    if !Path::new(config_path).exists() {
        create_default_config(config_path)?;
        println!("Created default config file at {}", config_path);
    }

    // 读取配置文件内容
    let config_content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path))?;

    // 解析TOML配置
    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to parse config file: {}", config_path))?;

    Ok(config)
}

fn create_default_config(config_path: &str) -> Result<()> {
    let default_config = r#"[general]
log_dir = "lightning_logs"
hparams_file = "hparams.yaml"
cache_enabled = true

[ignored_parameters]
parameters = [
    "fold",
    "devices",
    "random_seed",
    "timestamp",
    "worker_id",
    "run_id",
    "version",
]

[tolerance]
float_tolerance = 0.001
int_tolerance = 0
string_case_sensitive = false

[grouping]
main_key = ["model_name", "dataset"]
group_by_all_parameters = true
grouping_parameters = [
    "model_type",
    "dataset",
    "learning_rate",
]
similarity_threshold = 2

[diff]
show_detailed_diff = true
diff_format = "key: value1 vs value2"
highlight_diff_keys = true

[tui]
color_theme = "default"
colors = { same_experiment = "green", similar_experiment = "yellow", selected = "blue", background = "black", text = "white" }
layout = "list"
show_help_bar = true
auto_expand_groups = false

[keybindings]
up = "up"
down = "down"
left = "left"
right = "right"
select = "space"
confirm = "enter"
quit = "q"
help = "h"
filter = "/"

[test_script]
path = "test.py"
default_args = { filter = "", sort_key = "fold" }
prompt_for_args = true
fixed_args = []
"#;

    fs::write(config_path, default_config)
        .with_context(|| format!("Failed to create default config file: {}", config_path))?;

    Ok(())
}
