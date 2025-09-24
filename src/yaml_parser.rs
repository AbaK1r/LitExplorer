// src/yaml_parser.rs
use std::path::Path;
use std::collections::HashMap;
use anyhow::{Context, Result};
use crate::models::{ParameterValue, BasicParameterValue};
use serde_yaml;

/// 解析单个hparams.yaml文件到HashMap<String, ParameterValue>
// ————————————————————————————————————————————————————————————————————————
// 核心解析函数
// ————————————————————————————————————————————————————————————————————————
pub fn parse_hparams_file(file_path: &Path) -> Result<HashMap<String, ParameterValue>> {
    let contents = std::fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read hparams file: {}", file_path.display()))?;

    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML from file: {}", file_path.display()))?;

    let mut result = HashMap::new();
    flatten_yaml_value(&yaml_value, &mut result, String::new())?;
    Ok(result)
}

// ————————————————————————————————————————————————————————————————————————
// 递归扁平化函数：处理路径拼接
// ————————————————————————————————————————————————————————————————————————
fn flatten_yaml_value(
    value: &serde_yaml::Value,
    output: &mut HashMap<String, ParameterValue>,
    path: String,
) -> Result<()> {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (key, val) in map {
                let key_str = key.as_str()
                    .ok_or_else(|| anyhow::anyhow!("Non-string key in mapping: {:?}", key))?;
                let new_path = if path.is_empty() { key_str.to_string() } else { format!("{}-{}", path, key_str) };
                flatten_yaml_value(val, output, new_path)?;
            }
        }

        serde_yaml::Value::Sequence(seq) => {
            // Check if all items are simple (leaf) values
            if seq.iter().all(|v| matches!(v, serde_yaml::Value::String(_) | serde_yaml::Value::Number(_) | serde_yaml::Value::Bool(_))) {
                let list: Result<Vec<ParameterValue>> = seq
                    .iter().map(|v| base_value_to_parameter_value(v)).collect();
                output.insert(path, ParameterValue::List(list?));
            } else {
                // Recurse into complex list items (e.g., maps or nested lists)
                for (i, item) in seq.iter().enumerate() {
                    let item_path = format!("{}-{}", path, i);
                    flatten_yaml_value(item, output, item_path)?;
                }
            }
        }

        serde_yaml::Value::Tagged(tagged) => {
            // Ignore YAML tags, just recurse into the value
            flatten_yaml_value(&tagged.value, output, path)?;
        }

        serde_yaml::Value::Null => {
            // Skip null values (or you could insert a Null variant if needed)
        }

        _ => {
            // Leaf node: string, number, bool
            output.insert(path, base_value_to_parameter_value(value)?);
        }
    }
    Ok(())
}

// ————————————————————————————————————————————————————————————————————————
// 将 serde_yaml::Value 转换为 ParameterValue（支持递归）
// ————————————————————————————————————————————————————————————————————————
fn base_value_to_parameter_value(value: &serde_yaml::Value) -> Result<ParameterValue> {
    match value {
        serde_yaml::Value::String(s) => Ok(ParameterValue::Basic(BasicParameterValue::String(s.clone()))),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(ParameterValue::Basic(BasicParameterValue::Int(i)))
            } else if let Some(f) = n.as_f64() {
                Ok(ParameterValue::Basic(BasicParameterValue::Float(f)))
            } else {
                Err(anyhow::anyhow!("Unsupported number format in YAML"))
            }
        }
        serde_yaml::Value::Bool(b) => Ok(ParameterValue::Basic(BasicParameterValue::Bool(*b))),
        _ => Err(anyhow::anyhow!("Unexpected YAML value type: {:?}", value)),
    }
}

/// 批量解析多个hparams.yaml文件
pub fn parse_multiple_hparams_files(file_paths: &[std::path::PathBuf]) -> Result<Vec<(std::path::PathBuf, HashMap<String, ParameterValue>)>> {
    let mut results = Vec::new();

    for file_path in file_paths {
        match parse_hparams_file(file_path) {
            Ok(hparams) => {
                results.push((file_path.clone(), hparams));
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e);
                // 可以选择跳过错误文件或返回错误
                // 这里我们选择跳过并继续处理其他文件
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{print_hparams_pretty, ParameterValue, BasicParameterValue};

    #[test]
    fn test_parse_hparams_file() {
        let yaml_content = r#"
seed: 172
call_back_monitor: VAL--acc
csdp: null
feats_size: 1536
random_patch_share: 0.7
trainer:
  num_sanity_val_steps: 0
  accelerator: gpu
  devices:
  - 1
  benchmark: true
  check_val_every_n_epoch: 2
  precision: 32-true
employees:
  - id: 1
    name: John Doe
    department: Engineering
    skills:
      - Python
      - JavaScript
      - Docker
    contact:
      email: john@company.com
      phone: "+1-555-0101"
  
  - id: 2
    name: Jane Smith
    department: Marketing
    skills:
      - SEO
      - Content Writing
      - Analytics
    contact:
      email: jane@company.com
      phone: "+1-555-0102"
"#;

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_hparams.yaml");
        std::fs::write(&test_file, yaml_content).unwrap();

        let result = parse_hparams_file(&test_file);
        assert!(result.is_ok(), "Failed to parse YAML: {:?}", result.err());
        let hparams = result.unwrap();

        // 打印便于调试（CI 中也可保留）
        print_hparams_pretty(&hparams).unwrap();

        // ——————————————————————————————————————
        // ✅ 关键断言：验证扁平化结果
        // ——————————————————————————————————————

        // 基本字段
        assert_eq!(
            hparams.get("seed"),
            Some(&ParameterValue::Basic(BasicParameterValue::Int(172)))
        );
        assert_eq!(
            hparams.get("feats_size"),
            Some(&ParameterValue::Basic(BasicParameterValue::Int(1536)))
        );
        assert_eq!(
            hparams.get("random_patch_share"),
            Some(&ParameterValue::Basic(BasicParameterValue::Float(0.7)))
        );
        assert_eq!(
            hparams.get("call_back_monitor"),
            Some(&ParameterValue::Basic(BasicParameterValue::String("VAL--acc".to_string())))
        );

        // null 字段应被忽略（根据你的 flatten 逻辑）
        assert!(!hparams.contains_key("csdp"));

        // trainer 下的字段
        assert_eq!(
            hparams.get("trainer-num_sanity_val_steps"),
            Some(&ParameterValue::Basic(BasicParameterValue::Int(0)))
        );
        assert_eq!(
            hparams.get("trainer-accelerator"),
            Some(&ParameterValue::Basic(BasicParameterValue::String("gpu".to_string())))
        );
        assert_eq!(
            hparams.get("trainer-benchmark"),
            Some(&ParameterValue::Basic(BasicParameterValue::Bool(true)))
        );
        assert_eq!(
            hparams.get("trainer-check_val_every_n_epoch"),
            Some(&ParameterValue::Basic(BasicParameterValue::Int(2)))
        );
        assert_eq!(
            hparams.get("trainer-precision"),
            Some(&ParameterValue::Basic(BasicParameterValue::String("32-true".to_string())))
        );

        // trainer-devices 是简单列表 → 应为 List
        assert_eq!(
            hparams.get("trainer-devices"),
            Some(&ParameterValue::List(vec![
                ParameterValue::Basic(BasicParameterValue::Int(1))
            ]))
        );

        // employees[0] 字段
        assert_eq!(
            hparams.get("employees-0-id"),
            Some(&ParameterValue::Basic(BasicParameterValue::Int(1)))
        );
        assert_eq!(
            hparams.get("employees-0-name"),
            Some(&ParameterValue::Basic(BasicParameterValue::String("John Doe".to_string())))
        );
        assert_eq!(
            hparams.get("employees-0-department"),
            Some(&ParameterValue::Basic(BasicParameterValue::String("Engineering".to_string())))
        );
        assert_eq!(
            hparams.get("employees-0-contact-email"),
            Some(&ParameterValue::Basic(BasicParameterValue::String("john@company.com".to_string())))
        );

        // employees[0].skills 是字符串列表 → 应为 List
        assert_eq!(
            hparams.get("employees-0-skills"),
            Some(&ParameterValue::List(vec![
                ParameterValue::Basic(BasicParameterValue::String("Python".to_string())),
                ParameterValue::Basic(BasicParameterValue::String("JavaScript".to_string())),
                ParameterValue::Basic(BasicParameterValue::String("Docker".to_string())),
            ]))
        );

        // employees[1] 字段
        assert_eq!(
            hparams.get("employees-1-id"),
            Some(&ParameterValue::Basic(BasicParameterValue::Int(2)))
        );
        assert_eq!(
            hparams.get("employees-1-name"),
            Some(&ParameterValue::Basic(BasicParameterValue::String("Jane Smith".to_string())))
        );
        assert_eq!(
            hparams.get("employees-1-skills"),
            Some(&ParameterValue::List(vec![
                ParameterValue::Basic(BasicParameterValue::String("SEO".to_string())),
                ParameterValue::Basic(BasicParameterValue::String("Content Writing".to_string())),
                ParameterValue::Basic(BasicParameterValue::String("Analytics".to_string())),
            ]))
        );

        // 确保没有多余字段（可选，用于严格验证）
        let expected_keys: std::collections::HashSet<&str> = [
            "seed",
            "call_back_monitor",
            "feats_size",
            "random_patch_share",
            "trainer-num_sanity_val_steps",
            "trainer-accelerator",
            "trainer-devices",
            "trainer-benchmark",
            "trainer-check_val_every_n_epoch",
            "trainer-precision",
            "employees-0-id",
            "employees-0-name",
            "employees-0-department",
            "employees-0-skills",
            "employees-0-contact-email",
            "employees-0-contact-phone",
            "employees-1-id",
            "employees-1-name",
            "employees-1-department",
            "employees-1-skills",
            "employees-1-contact-email",
            "employees-1-contact-phone",
        ].iter().cloned().collect();

        let actual_keys: std::collections::HashSet<&String> = hparams.keys().collect();
        let actual_key_strs: std::collections::HashSet<&str> = actual_keys.iter().map(|s| s.as_str()).collect();

        assert_eq!(actual_key_strs, expected_keys, "Key sets do not match");

        // 清理
        std::fs::remove_file(&test_file).unwrap();
    }
}