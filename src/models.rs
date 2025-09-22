use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Deserializer};
use std::fmt;
use anyhow::{Context, Result};

// ============ CONFIG 模块 ============
mod config {
    use super::*;

    #[derive(Debug, Deserialize)]
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

    #[derive(Debug, Deserialize)]
    pub struct GeneralConfig {
        pub log_dir: String,
        pub hparams_file: String,
        pub cache_enabled: bool,
    }

    #[derive(Debug, Deserialize)]
    pub struct IgnoredConfig {
        pub parameters: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ToleranceConfig {
        pub float_tolerance: f64,
        pub int_tolerance: i64,
        pub string_case_sensitive: bool,
    }

    #[derive(Debug, Deserialize)]
    pub struct GroupingConfig {
        pub group_by_all_parameters: bool,
        pub grouping_parameters: Option<Vec<String>>,
        pub similarity_threshold: usize,
    }

    #[derive(Debug, Deserialize)]
    pub struct DiffConfig {
        pub show_detailed_diff: bool,
        pub diff_format: String,
        pub highlight_diff_keys: bool,
    }

    #[derive(Debug, Deserialize)]
    pub struct TuiConfig {
        pub color_theme: String,
        pub colors: ColorConfig,
        pub layout: String,
        pub show_help_bar: bool,
        pub auto_expand_groups: bool,
    }

    #[derive(Debug, Deserialize)]
    pub struct ColorConfig {
        pub same_experiment: String,
        pub similar_experiment: String,
        pub selected: String,
        pub background: String,
        pub text: String,
    }

    #[derive(Debug, Deserialize)]
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

    #[derive(Debug, Deserialize)]
    pub struct TestScriptConfig {
        pub path: String,
        pub default_args: DefaultArgsConfig,
        pub prompt_for_args: bool,
        pub fixed_args: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct DefaultArgsConfig {
        #[serde(default, deserialize_with = "super::utils::deserialize_optional_string")]
        pub filter: Option<String>,
        #[serde(default, deserialize_with = "super::utils::deserialize_optional_string")]
        pub sort_key: Option<String>,
    }
}

// ============ PARAMETER VALUE 模块 ============
mod parameter_value {
    use super::*;
    use serde_yaml;

    #[derive(Debug, Clone, PartialEq)]
    pub enum ParameterValue {
        String(String),
        Float(f64),
        Int(i64),
        Bool(bool),
        Null,
        List(Vec<ParameterValue>),
        Map(HashMap<String, ParameterValue>),
    }

    impl<'de> Deserialize<'de> for ParameterValue {
        fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = serde_yaml::Value::deserialize(deserializer)
                .map_err(|e| serde::de::Error::custom(format!("YAML parse error: {}", e)))?;

            match value {
                serde_yaml::Value::String(s) => Ok(ParameterValue::String(s)),
                serde_yaml::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(ParameterValue::Int(i))
                    } else if let Some(f) = n.as_f64() {
                        Ok(ParameterValue::Float(f))
                    } else {
                        Err(serde::de::Error::custom("Unsupported number format"))
                    }
                }
                serde_yaml::Value::Bool(b) => Ok(ParameterValue::Bool(b)),
                serde_yaml::Value::Null => Ok(ParameterValue::Null),
                serde_yaml::Value::Sequence(seq) => {
                    let mut list = Vec::new();
                    for (i, item) in seq.into_iter().enumerate() {
                        let val = ParameterValue::deserialize(item)
                            .map_err(|e| serde::de::Error::custom(format!("Failed to deserialize list item at index {}: {}", i, e)))?;
                        list.push(val);
                    }
                    Ok(ParameterValue::List(list))
                }
                serde_yaml::Value::Mapping(map) => {
                    let mut hashmap = HashMap::new();
                    for (key, value) in map {
                        let key_str = key.as_str()
                            .ok_or_else(|| serde::de::Error::custom("Map key must be a string"))?
                            .to_string();
                        let param_value = ParameterValue::deserialize(value)
                            .map_err(|e| serde::de::Error::custom(format!("Failed to deserialize value for key '{}': {}", key_str, e)))?;
                        hashmap.insert(key_str, param_value);
                    }
                    Ok(ParameterValue::Map(hashmap))
                }
                _ => Err(serde::de::Error::custom("Unsupported value type")),
            }
        }
    }

    impl fmt::Display for ParameterValue {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ParameterValue::String(s) => write!(f, "\"{}\"", s),
                ParameterValue::Float(n) => write!(f, "{}", n),
                ParameterValue::Int(n) => write!(f, "{}", n),
                ParameterValue::Bool(b) => write!(f, "{}", b),
                ParameterValue::Null => write!(f, "null"),
                ParameterValue::List(list) => {
                    write!(f, "[")?;
                    for (i, item) in list.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", item)?;
                    }
                    write!(f, "]")
                }
                ParameterValue::Map(map) => {
                    write!(f, "{{")?;
                    for (i, (key, value)) in map.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "\"{}\": {}", key, value)?;
                    }
                    write!(f, "}}")
                }
            }
        }
    }

    impl ParameterValue {
        fn fmt_json_pretty_internal(&self, f: &mut String, indent_level: usize, indent_size: usize) {
            let indent = " ".repeat(indent_level * indent_size);
            let next_indent = " ".repeat((indent_level + 1) * indent_size);

            match self {
                ParameterValue::String(s) => {
                    // 转义字符串中的特殊字符
                    let escaped = s
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                        .replace('\r', "\\r")
                        .replace('\t', "\\t");
                    f.push_str(&format!("\"{}\"", escaped));
                }
                ParameterValue::Float(n) => {
                    f.push_str(&format!("{}", n));
                }
                ParameterValue::Int(n) => {
                    f.push_str(&format!("{}", n));
                }
                ParameterValue::Bool(b) => {
                    f.push_str(&format!("{}", b));
                }
                ParameterValue::Null => {
                    f.push_str("null");
                }
                ParameterValue::List(list) => {
                    if list.is_empty() {
                        f.push_str("[]");
                    } else {
                        f.push_str("[\n");
                        for (i, item) in list.iter().enumerate() {
                            f.push_str(&next_indent);
                            item.fmt_json_pretty_internal(f, indent_level + 1, indent_size);
                            if i < list.len() - 1 {
                                f.push_str(",\n");
                            } else {
                                f.push('\n');
                            }
                        }
                        f.push_str(&format!("{indent}]"));
                    }
                }
                ParameterValue::Map(map) => {
                    if map.is_empty() {
                        f.push_str("{}");
                    } else {
                        f.push_str("{\n");

                        // 对键进行排序以获得一致的输出
                        let mut entries: Vec<(&String, &ParameterValue)> = map.iter().collect();
                        entries.sort_by(|a, b| a.0.cmp(b.0));

                        for (i, (key, value)) in entries.iter().enumerate() {
                            // 转义键中的特殊字符
                            let escaped_key = key
                                .replace('\\', "\\\\")
                                .replace('"', "\\\"")
                                .replace('\n', "\\n")
                                .replace('\r', "\\r")
                                .replace('\t', "\\t");

                            f.push_str(&format!("{}\"{}\": ", next_indent, escaped_key));
                            value.fmt_json_pretty_internal(f, indent_level + 1, indent_size);
                            if i < entries.len() - 1 {
                                f.push_str(",\n");
                            } else {
                                f.push('\n');
                            }
                        }
                        f.push_str(&format!("{indent}}}"));
                    }
                }
            }
        }

        /// 将 ParameterValue 格式化为漂亮打印的 JSON 字符串
        ///
        /// # 参数
        /// - `indent_spaces`: 每级缩进的空格数
        ///
        /// # 示例
        /// ```
        /// let value = ParameterValue::Map(/* ... */);
        /// let json = value.to_json_pretty(2); // 使用2空格缩进
        /// println!("{}", json);
        /// ```
        pub fn to_json_pretty(&self, indent_spaces: usize) -> String {
            let mut result = String::new();
            self.fmt_json_pretty_internal(&mut result, 0, indent_spaces);
            result
        }
    }

    impl ParameterValue {
        pub fn equals_with_tolerance(&self, other: &Self, tolerance: &config::ToleranceConfig) -> bool {
            match (self, other) {
                (ParameterValue::String(a), ParameterValue::String(b)) => {
                    if tolerance.string_case_sensitive {
                        a == b
                    } else {
                        a.to_lowercase() == b.to_lowercase()
                    }
                }
                (ParameterValue::Float(a), ParameterValue::Float(b)) => {
                    // Handle NaN: if either is NaN, only equal if both are NaN
                    if a.is_nan() || b.is_nan() {
                        return a.is_nan() && b.is_nan();
                    }
                    (a - b).abs() <= tolerance.float_tolerance
                }
                (ParameterValue::Int(a), ParameterValue::Int(b)) => {
                    (a - b).abs() <= tolerance.int_tolerance
                }
                (ParameterValue::Bool(a), ParameterValue::Bool(b)) => a == b,
                (ParameterValue::Null, ParameterValue::Null) => true,
                (ParameterValue::List(a), ParameterValue::List(b)) => {
                    if a.len() != b.len() { return false; }
                    for (item_a, item_b) in a.iter().zip(b.iter()) {
                        if !item_a.equals_with_tolerance(item_b, tolerance) {
                            return false;
                        }
                    }
                    true
                }
                (ParameterValue::Map(a), ParameterValue::Map(b)) => {
                    if a.len() != b.len() { return false; }
                    for (key, value_a) in a {
                        if let Some(value_b) = b.get(key) {
                            if !value_a.equals_with_tolerance(value_b, tolerance) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    true
                }
                _ => false,
            }
        }

        pub fn to_simple_string(&self) -> String {
            match self {
                ParameterValue::String(s) => s.clone(),
                ParameterValue::Float(n) => format!("{:.6}", n),
                ParameterValue::Int(n) => n.to_string(),
                ParameterValue::Bool(b) => b.to_string(),
                ParameterValue::Null => "null".to_string(),
                ParameterValue::List(_) => "[list]".to_string(),
                ParameterValue::Map(_) => "{map}".to_string(),
            }
        }
    }
}

// ============ MODELS 模块 ============
mod models {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct VersionData {
        pub version_num: u32,
        pub path: PathBuf,
        pub hparams: HashMap<String, parameter_value::ParameterValue>,
    }

    #[derive(Debug, PartialEq)]
    pub struct ExperimentGroup {
        pub group_id: String,
        pub base_parameters: HashMap<String, parameter_value::ParameterValue>,
        pub member_versions: Vec<VersionData>,
    }
}

// ============ STATE 模块 ============
mod state {
    use super::*;

    #[derive(Debug)]
    pub struct AppState {
        pub all_versions: Vec<models::VersionData>,
        pub experiment_groups: Vec<models::ExperimentGroup>,
        pub config: config::Config,
    }
}

// ============ UTILS 模块 ============
mod utils {
    use super::*;
    use serde::de::Error as SerdeError;

    pub fn deserialize_optional_string<'de, D>(deserializer: D) -> std::result::Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        Ok(s.filter(|s| !s.is_empty()))
    }
}

// ============ PUBLIC RE-EXPORTS ============
pub use config::Config;
pub use parameter_value::ParameterValue;
pub use models::{VersionData, ExperimentGroup};
pub use state::AppState;