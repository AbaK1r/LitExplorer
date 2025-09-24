use serde::{Deserialize, Deserializer};

/// 应用程序配置结构
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub ignored_parameters: IgnoredConfig,
    pub tolerance: ToleranceConfig,
    pub grouping: GroupingConfig,
    pub diff: DiffConfig,
    pub tui: TuiConfig,
    pub keybindings: KeybindingsConfig,
    pub test_script: TestScriptConfig,
}

/// 通用配置
#[derive(Debug, Deserialize, Default)]
pub struct GeneralConfig {
    pub log_dir: String,
    pub hparams_file: String,
    pub cache_enabled: bool,
}

/// 忽略参数配置
#[derive(Debug, Deserialize, Default)]
pub struct IgnoredConfig {
    pub parameters: Vec<String>,
}

/// 容差配置
#[derive(Debug, Deserialize, Default)]
pub struct ToleranceConfig {
    pub float_tolerance: f64,
    pub int_tolerance: i64,
    pub string_case_sensitive: bool,
}

/// 分组配置
#[derive(Debug, Deserialize, Default)]
pub struct GroupingConfig {
    pub group_by_all_parameters: bool,
    pub grouping_parameters: Option<Vec<String>>,
    pub similarity_threshold: usize,
}

/// 差异比较配置
#[derive(Debug, Deserialize, Default)]
pub struct DiffConfig {
    pub show_detailed_diff: bool,
    pub diff_format: String,
    pub highlight_diff_keys: bool,
}

/// TUI界面配置
#[derive(Debug, Deserialize, Default)]
pub struct TuiConfig {
    pub color_theme: String,
    pub colors: ColorConfig,
    pub layout: String,
    pub show_help_bar: bool,
    pub auto_expand_groups: bool,
}

/// 颜色配置
#[derive(Debug, Deserialize, Default)]
pub struct ColorConfig {
    pub same_experiment: String,
    pub similar_experiment: String,
    pub selected: String,
    pub background: String,
    pub text: String,
}

/// 键盘绑定配置
#[derive(Debug, Deserialize, Default)]
pub struct KeybindingsConfig {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub select: String,
    pub confirm: String,
    pub quit: String,
    pub help: String,
    pub filter: String,
}

/// 测试脚本配置
#[derive(Debug, Deserialize, Default)]
pub struct TestScriptConfig {
    pub path: String,
    pub default_args: DefaultArgsConfig,
    pub prompt_for_args: bool,
    pub fixed_args: Vec<String>,
}

/// 默认参数配置
#[derive(Debug, Deserialize, Default)]
pub struct DefaultArgsConfig {
    #[serde(default, deserialize_with = "crate::models::utils::deserialize_optional_string")]
    pub filter: Option<String>,
    #[serde(default, deserialize_with = "crate::models::utils::deserialize_optional_string")]
    pub sort_key: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml;
    
    #[test]
    fn test_config_deserialization() {
        let yaml = r#"
        general:
          log_dir: logs
          hparams_file: hparams.yaml
          cache_enabled: true
        ignored_parameters:
          parameters: [param1, param2]
        tolerance:
          float_tolerance: 0.001
          int_tolerance: 0
          string_case_sensitive: true
        grouping:
          group_by_all_parameters: false
          grouping_parameters: [param3, param4]
          similarity_threshold: 80
        diff:
          show_detailed_diff: true
          diff_format: unified
          highlight_diff_keys: true
        tui:
          color_theme: default
          colors:
            same_experiment: green
            similar_experiment: yellow
            selected: blue
            background: black
            text: white
          layout: compact
          show_help_bar: true
          auto_expand_groups: false
        keybindings:
          up: k
          down: j
          left: h
          right: l
          select: space
          confirm: enter
          quit: q
          help: ?
          filter: |
            /
        test_script:
          path: test.sh
          default_args:
            filter: ""
            sort_key: ""
          prompt_for_args: true
          fixed_args: [--verbose]
        "#;
        
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize config");
        
        // 验证基本字段
        assert_eq!(config.general.log_dir, "logs");
        assert_eq!(config.general.cache_enabled, true);
        assert_eq!(config.tolerance.float_tolerance, 0.001);
    }
}