// src/experiment_grouping.rs
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use anyhow::Result;
use crate::models::{VersionData, ExperimentGroup, Config, ParameterValue, BasicParameterValue, IgnoredConfig, ToleranceConfig, GroupingConfig};
use crate::yaml_parser::parse_multiple_hparams_files;
use crate::file_utils::extract_version_number_safe;
use serde_yaml::{Mapping, Value};

/// 递归处理YAML值并将嵌套map展开为扁平结构
fn flatten_yaml_value(value: &Value, prefix: &str, result: &mut HashMap<String, ParameterValue>) {
    match value {
        Value::Mapping(map) => {
            // 处理嵌套的map
            for (k, v) in map {
                if let Some(key_str) = k.as_str() {
                    let new_key = if prefix.is_empty() {
                        key_str.to_string()
                    } else {
                        format!("{}-{}", prefix, key_str)
                    };
                    flatten_yaml_value(v, &new_key, result);
                }
            }
        },
        Value::Sequence(seq) => {
            // 处理列表，将其中的基本类型转换为ParameterValue
            let mut param_list = Vec::new();
            for item in seq {
                match item {
                    Value::String(s) => param_list.push(ParameterValue::Basic(BasicParameterValue::String(s.clone()))),
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            param_list.push(ParameterValue::Basic(BasicParameterValue::Int(i)));
                        } else if let Some(f) = n.as_f64() {
                            param_list.push(ParameterValue::Basic(BasicParameterValue::Float(f)));
                        }
                    },
                    Value::Bool(b) => param_list.push(ParameterValue::Basic(BasicParameterValue::Bool(*b))),
                    _ => {
                        // 对于复杂类型，转换为字符串
                        param_list.push(ParameterValue::Basic(BasicParameterValue::String(format!("{:?}", item))));
                    }
                }
            }
            if !prefix.is_empty() {
                result.insert(prefix.to_string(), ParameterValue::List(param_list));
            }
        },
        Value::String(s) => {
            if !prefix.is_empty() {
                result.insert(prefix.to_string(), ParameterValue::Basic(BasicParameterValue::String(s.clone())));
            }
        },
        Value::Number(n) => {
            if !prefix.is_empty() {
                if let Some(i) = n.as_i64() {
                    result.insert(prefix.to_string(), ParameterValue::Basic(BasicParameterValue::Int(i)));
                } else if let Some(f) = n.as_f64() {
                    result.insert(prefix.to_string(), ParameterValue::Basic(BasicParameterValue::Float(f)));
                }
            }
        },
        Value::Bool(b) => {
            if !prefix.is_empty() {
                result.insert(prefix.to_string(), ParameterValue::Basic(BasicParameterValue::Bool(*b)));
            }
        },
        _ => {}
    }
}

/// 从文件路径列表创建VersionData列表
pub fn create_version_data_list(_config: &Config, hparams_files: &[PathBuf]) -> Result<Vec<VersionData>> {
    let mut versions = Vec::new();
    
    // 批量解析所有hparams文件
    let parsed_results = parse_multiple_hparams_files(hparams_files)?;
    
    // 处理每个解析结果，创建VersionData
    for (file_path, hparams) in parsed_results {
        // 提取版本号
        let version_num = extract_version_number_safe(&file_path)?;
        
        let mut hparams_map = HashMap::new();
        
        // 遍历已解析的参数，添加到hparams_map中
        for (key, value) in hparams {
            // 如果值是包含JSON字符串的BasicParameterValue，尝试解析它
            if let ParameterValue::Basic(BasicParameterValue::String(json_str)) = &value {
                // 尝试将JSON字符串解析为YAML值，以便处理嵌套结构
                if let Ok(parsed_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    // 将serde_json::Value转换为serde_yaml::Value
                    let yaml_value = convert_json_to_yaml(parsed_value);
                    // 处理嵌套结构并展开
                    flatten_yaml_value(&yaml_value, &key, &mut hparams_map);
                    // 跳过添加原始JSON字符串
                    continue;
                }
            }
            
            // 添加参数到映射
            hparams_map.insert(key.to_string(), value);
        }
        
        // 创建VersionData实例
        let version_data = VersionData {
            version_num,
            path: file_path.parent().unwrap().to_path_buf(), // 保存目录路径
            hparams: hparams_map,
        };
        
        versions.push(version_data);
    }
    
    // 按版本号排序
    versions.sort_by(|a, b| a.version_num.cmp(&b.version_num));
    
    Ok(versions)
}

/// 将serde_json::Value转换为serde_yaml::Value
fn convert_json_to_yaml(json_value: serde_json::Value) -> serde_yaml::Value {
    match json_value {
        serde_json::Value::Null => serde_yaml::Value::Null,
        serde_json::Value::Bool(b) => serde_yaml::Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_yaml::Value::Number(serde_yaml::Number::from(i))
            } else if let Some(f) = n.as_f64() {
                // YAML的Number不直接支持浮点数，我们使用字符串表示
                serde_yaml::Value::String(f.to_string())
            } else {
                serde_yaml::Value::String(n.to_string())
            }
        },
        serde_json::Value::String(s) => serde_yaml::Value::String(s),
        serde_json::Value::Array(arr) => serde_yaml::Value::Sequence(
            arr.into_iter().map(convert_json_to_yaml).collect()
        ),
        serde_json::Value::Object(obj) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (k, v) in obj {
                mapping.insert(
                    serde_yaml::Value::String(k),
                    convert_json_to_yaml(v)
                );
            }
            serde_yaml::Value::Mapping(mapping)
        }
    }
}

/// 过滤参数，排除被忽略的参数
fn filter_parameters(
    hparams: &HashMap<String, ParameterValue>, 
    ignored_params: &[String],
    grouping_params: &Option<Vec<String>>,
) -> HashMap<String, ParameterValue> {
    let mut filtered_params = HashMap::new();
    
    // 构建忽略参数的HashSet以便快速查找
    let ignored_set: HashSet<_> = ignored_params.iter().collect();
    
    // 检查是否指定了分组参数
    match grouping_params {
        Some(params) => {
            // 如果指定了分组参数，只包含这些参数
            for param_name in params {
                if let Some(value) = hparams.get(param_name) {
                    if !ignored_set.contains(param_name) {
                        filtered_params.insert(param_name.clone(), value.clone());
                    }
                }
            }
        },
        None => {
            // 如果没有指定分组参数，包含所有未被忽略的参数
            for (key, value) in hparams {
                if !ignored_set.contains(key) {
                    filtered_params.insert(key.clone(), value.clone());
                }
            }
        },
    }
    
    filtered_params
}

/// 计算参数集的哈希值，用于创建组ID
fn compute_params_hash(params: &HashMap<String, ParameterValue>, config: &Config) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    
    // 辅助函数：递归哈希单个ParameterValue
    fn hash_parameter_value(value: &ParameterValue, hasher: &mut std::collections::hash_map::DefaultHasher, config: &Config) {
        match value {
            ParameterValue::Basic(BasicParameterValue::String(s)) => {
                if config.tolerance.string_case_sensitive {
                    s.hash(hasher);
                } else {
                    s.to_lowercase().hash(hasher);
                }
            },
            ParameterValue::Basic(BasicParameterValue::Float(f)) => {
                // 对浮点数进行舍入，考虑容差
                let rounded = (f / config.tolerance.float_tolerance).round() * config.tolerance.float_tolerance;
                rounded.to_bits().hash(hasher);
            },
            ParameterValue::Basic(BasicParameterValue::Int(i)) => {
                // 对整数进行处理，考虑容差
                let adjusted = i - (i % (config.tolerance.int_tolerance + 1));
                adjusted.hash(hasher);
            },
            ParameterValue::Basic(BasicParameterValue::Bool(b)) => b.hash(hasher),
            ParameterValue::List(list) => {
                // 对列表进行哈希
                list.len().hash(hasher);
                for item in list {
                    // 递归处理列表中的每个ParameterValue
                    hash_parameter_value(item, hasher, config);
                }
            },
        }
    }
    
    // 将参数按键排序以获得一致的哈希
    let mut sorted_keys: Vec<_> = params.keys().collect();
    sorted_keys.sort();
    
    for key in sorted_keys {
        // 对键进行哈希
        key.hash(&mut hasher);
        
        // 对值进行哈希（使用equals_with_tolerance方法来考虑容差）
        let value = params.get(key).unwrap();
        hash_parameter_value(value, &mut hasher, config);
    }
    
    // 将哈希值转换为字符串
    format!("{:x}", hasher.finish())
}

/// 比较两个参数集，返回差异参数的数量
fn count_different_parameters(
    params1: &HashMap<String, ParameterValue>,
    params2: &HashMap<String, ParameterValue>,
    tolerance: &Config,
) -> usize {
    let mut diff_count = 0;
    
    // 检查第一个参数集中的所有参数
    for (key, value1) in params1 {
        if let Some(value2) = params2.get(key) {
            if !value1.equals_with_tolerance(value2, &tolerance.tolerance) {
                diff_count += 1;
            }
        } else {
            diff_count += 1;
        }
    }
    
    // 检查第二个参数集中独有的参数
    for (key, _) in params2 {
        if !params1.contains_key(key) {
            diff_count += 1;
        }
    }
    
    diff_count
}

/// 将版本数据分组为实验组
pub fn group_versions(config: &Config, versions: Vec<VersionData>) -> Result<Vec<ExperimentGroup>> {
    let mut groups: Vec<ExperimentGroup> = Vec::new();
    let mut ungrouped_versions: Vec<VersionData> = versions.clone();
    
    // 对每个未分组的版本进行分组
    while let Some(version) = ungrouped_versions.pop() {
        // 过滤版本的参数
        let filtered_params = filter_parameters(
            &version.hparams,
            &config.ignored_parameters.parameters,
            &config.grouping.grouping_parameters,
        );
        
        // 尝试将版本添加到现有组
        let mut added_to_existing = false;
        
        for group in &mut groups {
            // 计算与组基准参数的差异
            let diff_count = count_different_parameters(
                &filtered_params,
                &group.base_parameters,
                config,
            );
            
            // 如果差异在阈值范围内，则添加到该组
            if diff_count == 0 {
                group.member_versions.push(version.clone());
                added_to_existing = true;
                break;
            }
        }
        
        // 如果没有添加到现有组，则创建新组
        if !added_to_existing {
            let group_id = compute_params_hash(&filtered_params, config);
            
            let new_group = ExperimentGroup {
                group_id,
                base_parameters: filtered_params,
                member_versions: vec![version],
            };
            
            groups.push(new_group);
        }
    }
    
    // 对每个组的成员按版本号排序
    for group in &mut groups {
        group.member_versions.sort_by(|a, b| a.version_num.cmp(&b.version_num));
    }
    
    // 按组内版本数量排序（可选）
    groups.sort_by(|a, b| b.member_versions.len().cmp(&a.member_versions.len()));
    
    Ok(groups)
}

/// 查找相似的实验组
pub fn find_similar_groups(
    groups: &[ExperimentGroup],
    config: &Config,
) -> HashMap<String, Vec<String>> {
    let mut similar_groups: HashMap<String, Vec<String>> = HashMap::new();
    
    // 为每个组查找相似的组
    for i in 0..groups.len() {
        let group_id = &groups[i].group_id;
        similar_groups.entry(group_id.clone()).or_default();
        
        for j in 0..groups.len() {
            if i == j { continue; }
            
            let diff_count = count_different_parameters(
                &groups[i].base_parameters,
                &groups[j].base_parameters,
                config,
            );
            
            if diff_count <= config.grouping.similarity_threshold {
                similar_groups
                    .get_mut(group_id)
                    .unwrap()
                    .push(groups[j].group_id.clone());
            }
        }
    }
    
    similar_groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IgnoredConfig, ToleranceConfig, GroupingConfig};
    
    // 辅助函数：创建测试配置
    fn create_test_config() -> Config {
        Config {
            general: Default::default(),
            ignored_parameters: IgnoredConfig {
                parameters: vec!["fold".to_string(), "devices".to_string()],
            },
            tolerance: ToleranceConfig {
                float_tolerance: 0.001,
                int_tolerance: 0,
                string_case_sensitive: false,
            },
            grouping: GroupingConfig {
                group_by_all_parameters: true,
                grouping_parameters: None,
                similarity_threshold: 2,
            },
            diff: Default::default(),
            tui: Default::default(),
            keybindings: Default::default(),
            test_script: Default::default(),
        }
    }
    
    // 测试过滤参数功能
    #[test]
    fn test_filter_parameters() {
        let mut hparams = HashMap::new();
        hparams.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        hparams.insert("fold".to_string(), ParameterValue::Basic(BasicParameterValue::Int(1)));
        hparams.insert("devices".to_string(), ParameterValue::Basic(BasicParameterValue::Int(2)));
        hparams.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
        
        let ignored_params = vec!["fold".to_string(), "devices".to_string()];
        let grouping_params: Option<Vec<String>> = None;
        
        let filtered = filter_parameters(&hparams, &ignored_params, &grouping_params);
        
        assert!(filtered.contains_key("model"));
        assert!(filtered.contains_key("lr"));
        assert!(!filtered.contains_key("fold"));
        assert!(!filtered.contains_key("devices"));
    }
    
    // 测试递归过滤嵌套的参数（现在是扁平结构）
    #[test]
    fn test_filter_nested_parameters() {
        // 创建扁平结构，模拟展开后的嵌套Map
        let mut hparams = HashMap::new();
        hparams.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        hparams.insert("trainer-max_epochs".to_string(), ParameterValue::Basic(BasicParameterValue::Int(100)));
        hparams.insert("trainer-devices".to_string(), ParameterValue::Basic(BasicParameterValue::Int(2))); // 应该被忽略
        hparams.insert("config-optimizer".to_string(), ParameterValue::List(vec![
            ParameterValue::Basic(BasicParameterValue::String("adam".to_string())),
            ParameterValue::Basic(BasicParameterValue::String("sgd".to_string())),
        ]));
        hparams.insert("config-fold".to_string(), ParameterValue::Basic(BasicParameterValue::Int(3))); // 应该被忽略
        
        let ignored_params = vec!["trainer-devices".to_string(), "config-fold".to_string()];
        let grouping_params: Option<Vec<String>> = None;
        
        let filtered = filter_parameters(&hparams, &ignored_params, &grouping_params);
        
        // 验证参数
        assert!(filtered.contains_key("model"));
        assert!(filtered.contains_key("trainer-max_epochs"));
        assert!(filtered.contains_key("config-optimizer"));
        assert!(!filtered.contains_key("trainer-devices")); // 应该被忽略
        assert!(!filtered.contains_key("config-fold")); // 应该被忽略
    }
    
    // 测试指定分组参数时的过滤逻辑
    #[test]
    fn test_filter_with_grouping_parameters() {
        let mut hparams = HashMap::new();
        hparams.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        hparams.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
        hparams.insert("batch_size".to_string(), ParameterValue::Basic(BasicParameterValue::Int(32)));
        hparams.insert("fold".to_string(), ParameterValue::Basic(BasicParameterValue::Int(1)));
        
        let ignored_params = vec!["fold".to_string()];
        let grouping_params = Some(vec!["model".to_string(), "lr".to_string()]);
        
        let filtered = filter_parameters(&hparams, &ignored_params, &grouping_params);
        
        assert!(filtered.contains_key("model"));
        assert!(filtered.contains_key("lr"));
        assert!(!filtered.contains_key("batch_size")); // 不是分组参数，应该被过滤掉
        assert!(!filtered.contains_key("fold")); // 是忽略参数，应该被过滤掉
    }
    
    // 测试参数比较功能
    // #[test]
    // fn test_count_different_parameters() {
    //     let mut params1 = HashMap::new();
    //     params1.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
    //     params1.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
    //     params1.insert("batch_size".to_string(), ParameterValue::Basic(BasicParameterValue::Int(32)));
        
    //     let mut params2 = HashMap::new();
    //     params2.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
    //     params2.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.002))); // 不同的值
    //     params2.insert("batch_size".to_string(), ParameterValue::Basic(BasicParameterValue::Int(32)));
    //     params2.insert("epochs".to_string(), ParameterValue::Basic(BasicParameterValue::Int(100))); // 额外的参数
        
    //     let different_params = count_different_parameters(&params1, &params2, &ignored_params);
        
    //     assert_eq!(different_params.len(), 2);
    //     assert!(different_params.contains(&"lr".to_string()));
    //     assert!(different_params.contains(&"epochs".to_string()));
    // }
    
    // 测试扁平结构的嵌套参数比较
    #[test]
    fn test_count_different_parameters_nested() {
        let config = create_test_config();
        // 创建一个没有浮点数容差的配置，确保0.001和0.002被识别为不同的值
        let strict_config = Config {
            tolerance: ToleranceConfig {
                float_tolerance: 0.0,
                int_tolerance: 0,
                string_case_sensitive: false,
            },
            ..config
        };

        let mut params1 = HashMap::new();
        params1.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        params1.insert("trainer-lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
        params1.insert("trainer-epochs".to_string(), ParameterValue::Basic(BasicParameterValue::Int(100)));
        
        let mut params2 = HashMap::new();
        params2.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        params2.insert("trainer-lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.002))); // 不同的值
        params2.insert("trainer-epochs".to_string(), ParameterValue::Basic(BasicParameterValue::Int(100)));
        
        let different_params = count_different_parameters(&params1, &params2, &strict_config);
        
        assert_eq!(different_params, 1, "Only 'trainer-lr' parameter should be different");
    }

    // 测试参数哈希计算
    #[test]
    fn test_compute_params_hash() {
        let config = create_test_config();
        let mut params1 = HashMap::new();
        params1.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        params1.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
        params1.insert("batch_size".to_string(), ParameterValue::Basic(BasicParameterValue::Int(32)));
        
        // 创建一个顺序不同但内容相同的参数集
        let mut params2 = HashMap::new();
        params2.insert("batch_size".to_string(), ParameterValue::Basic(BasicParameterValue::Int(32)));
        params2.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        params2.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
        
        let hash1 = compute_params_hash(&params1, &config);
        let hash2 = compute_params_hash(&params2, &config);
        
        // 即使参数顺序不同，相同的参数集应该产生相同的哈希值
        assert_eq!(hash1, hash2);
    }
    
    // 测试版本分组功能
    // #[test]
    // fn test_group_versions() {
    //     // 创建几个测试版本数据
    //     let versions = create_version_data_list();
        
    //     // 分组配置
    //     let ignored_params = vec!["fold".to_string(), "devices".to_string()];
    //     let grouping_params: Option<Vec<String>> = None;
        
    //     let groups = group_versions(&versions, &ignored_params, &grouping_params);
        
    //     // 预期结果：版本1和版本3应该在同一组（因为它们的参数相同，除了被忽略的fold和devices）
    //     // 版本2应该在单独的一组
    //     assert_eq!(groups.len(), 2);
        
    //     // 检查分组逻辑
    //     let group_with_versions_1_and_3 = groups.iter()
    //         .find(|g| g.versions.len() == 2)
    //         .expect("Should have a group with 2 versions");
        
    //     assert_eq!(group_with_versions_1_and_3.versions.len(), 2);
        
    //     let group_with_version_2 = groups.iter()
    //         .find(|g| g.versions.len() == 1)
    //         .expect("Should have a group with 1 version");
        
    //     assert_eq!(group_with_version_2.versions.len(), 1);
    //     assert_eq!(group_with_version_2.versions[0].id, "version_2");
    // }
    
    // 测试相似实验组查找逻辑
    // #[test]
    // fn test_find_similar_groups() {
    //     // 创建一些测试版本数据
    //     let versions = create_version_data_list();
        
    //     // 分组配置
    //     let ignored_params = vec!["fold".to_string(), "devices".to_string()];
    //     let grouping_params: Option<Vec<String>> = None;
        
    //     let groups = group_versions(&versions, &ignored_params, &grouping_params);
        
    //     // 假设我们有一个参考配置，它与版本1和版本3的配置只有一个参数不同
    //     let mut reference_config = HashMap::new();
    //     reference_config.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
    //     reference_config.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.0015))); // 与版本1和3的lr=0.001不同
    //     reference_config.insert("batch_size".to_string(), ParameterValue::Basic(BasicParameterValue::Int(32)));
        
    //     let similar_groups = find_similar_groups(&groups, &reference_config); // 最多允许一个不同的参数，通过config中的similarity_threshold控制
        
    //     // 预期结果：与包含版本1和3的组相似
    //     assert_eq!(similar_groups.len(), 1);
        
    //     // 确保找到的是包含版本1和3的组
    //     let found_group = &similar_groups[0];
    //     assert_eq!(found_group.versions.len(), 2);
    // }
    
    // 测试相似组查找功能
    #[test]
    fn test_find_similar_groups1() {
        let config = create_test_config();
        
        // 创建几个VersionData实例用于member_versions
        let version1 = VersionData {
            path: "version_001".to_string().into(),
            version_num: 1,
            hparams: HashMap::new(),
        };
        
        let version2 = VersionData {
            path: "version_002".to_string().into(),
            version_num: 2,
            hparams: HashMap::new(),
        };
        
        let version3 = VersionData {
            path: "version_003".to_string().into(),
            version_num: 3,
            hparams: HashMap::new(),
        };
        
        // 创建几个组
        let mut group1 = ExperimentGroup {
            group_id: "group_1".to_string(),
            member_versions: vec![version1],
            base_parameters: HashMap::new(),
        };
        group1.base_parameters.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        group1.base_parameters.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
        group1.base_parameters.insert("batch_size".to_string(), ParameterValue::Basic(BasicParameterValue::Int(32)));
        
        let mut group2 = ExperimentGroup {
            group_id: "group_2".to_string(),
            member_versions: vec![version2],
            base_parameters: HashMap::new(),
        };
        group2.base_parameters.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())));
        group2.base_parameters.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
        group2.base_parameters.insert("batch_size".to_string(), ParameterValue::Basic(BasicParameterValue::Int(64))); // 与group1不同
        
        let mut group3 = ExperimentGroup {
            group_id: "group_3".to_string(),
            member_versions: vec![version3],
            base_parameters: HashMap::new(),
        };
            group3.base_parameters.insert("model".to_string(), ParameterValue::Basic(BasicParameterValue::String("rnn".to_string())));
        group3.base_parameters.insert("lr".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.002)));
        group3.base_parameters.insert("optimizer".to_string(), ParameterValue::Basic(BasicParameterValue::String("adam".to_string())));
        
        let groups = vec![group1, group2, group3];
        let similar_groups = find_similar_groups(&groups, &config);
        
        // 验证group1和group2是否相似
        assert!(similar_groups.get("group_1").is_some(), "Group 1 should have similar groups");
        let group1_similar = similar_groups.get("group_1").unwrap();
        assert!(group1_similar.contains(&"group_2".to_string()), "Group 1 should be similar to Group 2");
        
        // group3应该与其他组不相似
        if let Some(group3_similar) = similar_groups.get("group_3") {
            assert!(group3_similar.is_empty(), "Group 3 should not be similar to any other group");
        }
    }
    
    // 测试嵌套map展开功能
    #[test]
    fn test_nested_map_flattening() {
        // 创建包含嵌套map的YAML字符串
        let yaml_str = "{\"model_config\": {\"layers\": {\"hidden\": 128, \"output\": 10}, \"optimizer\": {\"type\": \"adam\", \"lr\": 0.001}}}";
        
        // 解析为Value
        if let Ok(value) = serde_yaml::from_str::<Value>(yaml_str) {
            let mut result = HashMap::new();
            flatten_yaml_value(&value, "", &mut result);
            
            // 验证嵌套map被正确展开
            assert!(result.contains_key("model_config-layers-hidden"));
            assert!(result.contains_key("model_config-layers-output"));
            assert!(result.contains_key("model_config-optimizer-type"));
            assert!(result.contains_key("model_config-optimizer-lr"));
            
            // 验证值的类型
            if let Some(ParameterValue::Basic(BasicParameterValue::Int(hidden))) = result.get("model_config-layers-hidden") {
                assert_eq!(*hidden, 128);
            } else {
                panic!("model_config-layers-hidden should be a Basic Int");
            }
            
            if let Some(ParameterValue::Basic(BasicParameterValue::Int(output))) = result.get("model_config-layers-output") {
                assert_eq!(*output, 10);
            } else {
                panic!("model_config-layers-output should be a Basic Int");
            }
            
            if let Some(ParameterValue::Basic(BasicParameterValue::String(optimizer_type))) = result.get("model_config-optimizer-type") {
                assert_eq!(*optimizer_type, "adam");
            } else {
                panic!("model_config-optimizer-type should be a Basic String");
            }
            
            if let Some(ParameterValue::Basic(BasicParameterValue::Float(lr))) = result.get("model_config-optimizer-lr") {
                assert_eq!(*lr, 0.001);
            } else {
                panic!("model_config-optimizer-lr should be a Basic Float");
            }
        } else {
            panic!("Failed to parse YAML string");
        }
    }
    
    // 测试List只包含基本类型的ParameterValue
    #[test]
    fn test_list_with_basic_types() {
        let mut params = HashMap::new();
        
        // 创建一个包含基本类型的List
        let optimizers = ParameterValue::List(vec![
            ParameterValue::Basic(BasicParameterValue::String("adam".to_string())),
            ParameterValue::Basic(BasicParameterValue::String("sgd".to_string())),
            ParameterValue::Basic(BasicParameterValue::String("rmsprop".to_string())),
        ]);
        
        params.insert("optimizers".to_string(), optimizers);
        
        // 检查参数是否正确
        if let ParameterValue::List(optimizers_list) = params.get("optimizers").unwrap() {
            assert_eq!(optimizers_list.len(), 3);
            
            if let ParameterValue::Basic(BasicParameterValue::String(optimizer_name)) = &optimizers_list[0] {
                assert_eq!(optimizer_name, "adam");
            } else {
                panic!("First optimizer should be a Basic String");
            }
        } else {
            panic!("optimizers should be a List");
        }
    }
}