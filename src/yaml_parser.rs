// src/yaml_parser.rs
use std::fs::File;
use std::io::Read;
use std::path::Path;
use anyhow::{Context, Result};
use crate::models::ParameterValue;
use serde_yaml;

/// 解析单个hparams.yaml文件到HashMap
pub fn parse_hparams_file(file_path: &Path) -> Result<ParameterValue> {
    // 打开文件
    let mut file = File::open(file_path)
        .with_context(|| format!("Failed to open hparams file: {}", file_path.display()))?;

    // 读取文件内容
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read hparams file: {}", file_path.display()))?;

    // 解析YAML内容
    let hparams: ParameterValue = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML from file: {}", file_path.display()))?;

    Ok(hparams)
}

/// 批量解析多个hparams.yaml文件
pub fn parse_multiple_hparams_files(file_paths: &[std::path::PathBuf]) -> Result<Vec<(std::path::PathBuf, ParameterValue)>> {
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
    use crate::models::ParameterValue;

    #[test]
    fn test_parse_hparams_file() {
        // 创建一个包含嵌套结构的YAML内容用于测试
        let yaml_content = r#"
seed: 172
call_back_monitor: VAL--acc
call_back_mode: max
csdp: null
sdp: null
model_name: snuffy_multiclass
feats_size: 1536
random_patch_share: 0.7
big_lambda: 2000
c_dropout: 0.15
num_classes: 2
dataset: rxa
fold: 6
train_batch_size: 1
val_batch_size: 1
num_workers: 8
weight_decay: 0.0001
lr: 0.0003
lr_scheduler: step
lr_step_size: 5
lr_gamma: 0.98
early_stop: false
trainer:
  num_sanity_val_steps: 0
  max_epochs: 110
  accelerator: gpu
  devices:
  - 1
  benchmark: true
  check_val_every_n_epoch: 2
  precision: 32-true
  log_every_n_steps: 50
  enable_progress_bar: false
  enable_model_summary: true
  accumulate_grad_batches: 4

"#;

        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_hparams.yaml");
        std::fs::write(&test_file, yaml_content).unwrap();

        // 解析文件
        let result = parse_hparams_file(&test_file);
        assert!(result.is_ok());

        let hparams = result.unwrap();
        println!("Hparams: \n{}", hparams.to_json_pretty(4));
        if let ParameterValue::Map(map) = &hparams {
            assert_eq!(map.get("model_name"), Some(&ParameterValue::String("snuffy_multiclass".to_string())));
            assert_eq!(map.get("lr"), Some(&ParameterValue::Float(0.0003)));
            assert_eq!(map.get("seed"), Some(&ParameterValue::Int(172)));

            // 测试嵌套Map
            if let Some(ParameterValue::Map(trainer)) = map.get("trainer") {
                assert_eq!(trainer.get("max_epochs"), Some(&ParameterValue::Int(110)));
                assert_eq!(trainer.get("accelerator"), Some(&ParameterValue::String("gpu".to_string())));

                // 测试嵌套List
                if let Some(ParameterValue::List(devices)) = trainer.get("devices") {
                    assert_eq!(devices.len(), 1);
                    assert_eq!(devices[0], ParameterValue::Int(1));
                } else {
                    panic!("Expected list for devices");
                }
            } else {
                panic!("Expected map for trainer");
            }
        } else {
            panic!("Expected map for hparams");
        }

        // 清理临时文件
        std::fs::remove_file(&test_file).unwrap();
    }
}