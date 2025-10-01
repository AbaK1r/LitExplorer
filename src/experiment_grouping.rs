// src/experiment_grouping.rs
use crate::file_utils::extract_version_number_safe;
use crate::models::{
    BasicParameterValue, Config, ExperimentGroup, GroupingConfig, IgnoredConfig, ParameterValue,
    ToleranceConfig, VersionData,
};
use crate::yaml_parser::parse_multiple_hparams_files;
use anyhow::Result;
use serde_yaml::{Mapping, Value};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// 从文件路径列表创建VersionData列表
/// 如果配置了main_key，则按main_key分组，并在每个分组内删除共有参数
/// 如果没有配置main_key，则在所有版本中删除共有参数
///
/// # 返回值
/// 返回版本数据列表和每个main_key分组内的相同hparams数据
pub fn create_version_data_list(
    config: &Config,
    hparams_files: &[PathBuf],
) -> Result<(
    Vec<VersionData>,
    HashMap<String, HashMap<String, ParameterValue>>,
)> {
    let mut versions = Vec::new();

    // 批量解析所有hparams文件
    let parsed_results = parse_multiple_hparams_files(hparams_files)?;

    // 处理每个解析结果，创建VersionData
    for (file_path, hparams) in parsed_results {
        // 提取版本号
        let version_num = extract_version_number_safe(&file_path)?;

        // 检查是否配置了main_key
        if let Some(main_keys) = &config.grouping.main_key {
            // 验证该版本是否包含所有配置的main_key
            for main_key in main_keys {
                if !hparams.contains_key(main_key) {
                    return Err(anyhow::anyhow!(
                        "Version {} is missing required main_key '{}'",
                        version_num,
                        main_key
                    ));
                }
            }
        }

        // 过滤参数，排除被忽略的参数和根据分组参数进行筛选
        let filtered_hparams = filter_parameters(
            &hparams,
            &config.ignored_parameters.parameters,
            &config.grouping.grouping_parameters,
        );

        // 创建VersionData实例
        let version_data = VersionData {
            version_num,
            path: file_path.parent().unwrap().to_path_buf(), // 保存目录路径
            hparams: filtered_hparams,
        };

        versions.push(version_data);
    }

    // 按版本号排序
    versions.sort_by(|a, b| a.version_num.cmp(&b.version_num));

    // 存储每个main_key分组内的相同hparams数据
    let mut group_common_hparams: HashMap<String, HashMap<String, ParameterValue>> = HashMap::new();

    // 检查是否配置了main_key
    if let Some(main_keys) = &config.grouping.main_key {
        // 按main_key值组合对版本进行分组
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();

        for (index, version) in versions.iter().enumerate() {
            // 获取所有main_key的值作为分组键
            let mut group_key_parts = Vec::new();
            for main_key in main_keys {
                if let Some(main_key_value) = version.hparams.get(main_key) {
                    group_key_parts.push(format!("{}={}", main_key, main_key_value));
                }
            }

            // 如果所有main_key都存在，则创建分组键
            if group_key_parts.len() == main_keys.len() {
                let group_key = group_key_parts.join(", ");
                groups.entry(group_key.clone()).or_default().push(index);
            }
        }

        // 对每个分组，找出并删除共有参数
        for (group_key, group_indices) in groups.iter() {
            if group_indices.is_empty() {
                continue;
            }

            // 只有当分组内有多个版本时才删除共有参数
            if group_indices.len() > 1 {
                // 找出该分组内所有版本共有的参数
                let mut common_params: HashMap<String, ParameterValue> = HashMap::new();
                let first_version_hparams = &versions[group_indices[0]].hparams;

                for (key, value) in first_version_hparams {
                    // 跳过所有main_key本身
                    if main_keys.contains(&key) {
                        continue;
                    }

                    let mut is_common = true;

                    // 检查分组内其他版本是否也有相同的键值对
                    for &index in &group_indices[1..] {
                        if let Some(other_value) = versions[index].hparams.get(key) {
                            if other_value != value {
                                is_common = false;
                                break;
                            }
                        } else {
                            is_common = false;
                            break;
                        }
                    }

                    if is_common {
                        common_params.insert(key.clone(), value.clone());
                    }
                }

                // 保存分组内的相同hparams数据
                group_common_hparams.insert(group_key.to_string(), common_params.clone());

                // 从分组内所有版本中删除共有的hparams键值对
                for &index in group_indices {
                    for key in &common_params {
                        versions[index].hparams.remove(key.0);
                    }
                }
            }
        }
    } else if config.grouping.grouping_parameters.is_none() {
        // 只有在没有指定分组参数时，才删除共有参数
        // 如果指定了分组参数，我们已经过滤了需要的参数，不应该再删除
        if !versions.is_empty() {
            // 首先，获取第一个版本的所有键值对
            let first_version_hparams = &versions[0].hparams;
            let mut common_params: HashMap<String, ParameterValue> = HashMap::new();

            for (key, value) in first_version_hparams {
                let mut is_common = true;

                // 检查其他版本是否也有相同的键值对
                for version in &versions[1..] {
                    if let Some(other_value) = version.hparams.get(key) {
                        if other_value != value {
                            is_common = false;
                            break;
                        }
                    } else {
                        is_common = false;
                        break;
                    }
                }

                if is_common {
                    common_params.insert(key.clone(), value.clone());
                }
            }

            // 从所有版本中删除共有的hparams键值对
            for version in &mut versions {
                for key in &common_params {
                    version.hparams.remove(key.0);
                }
            }
        }
    }

    Ok((versions, group_common_hparams))
}

/// 过滤参数，排除被忽略的参数
///
/// 此函数根据配置过滤参数映射，支持两种模式：
/// 1. 如果指定了分组参数列表，则只包含这些参数（同时排除被忽略的参数）
/// 2. 如果没有指定分组参数，则包含所有未被忽略的参数
///
/// # 参数
/// * `hparams` - 原始参数映射
/// * `ignored_params` - 需要排除的参数名列表
/// * `grouping_params` - 可选的分组参数列表，如果指定则只包含这些参数
///
/// # 返回值
/// * `HashMap<String, ParameterValue>` - 过滤后的参数映射
///
/// # 示例
/// ```ignore
/// let filtered = filter_parameters(&params, &["timestamp".to_string()], &Some(vec!["lr".to_string()]));
/// // 只返回"lr"参数（如果存在且未被忽略）
/// ```
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
        }
        None => {
            // 如果没有指定分组参数，包含所有未被忽略的参数
            for (key, value) in hparams {
                if !ignored_set.contains(key) {
                    filtered_params.insert(key.clone(), value.clone());
                }
            }
        }
    }

    filtered_params
}

/// 计算参数集的哈希值，用于创建组ID
///
/// 此函数根据参数值计算一致的哈希值，用于标识具有相同参数配置的实验组。
/// 哈希计算考虑了配置的容差设置，确保在容差范围内相等的参数产生相同的哈希值。
/// 参数按键排序以确保哈希的一致性，不受参数顺序影响。
///
/// # 参数
/// * `params` - 要计算哈希的参数映射
/// * `config` - 包含容差配置的配置对象
///
/// # 返回值
/// * `String` - 十六进制格式的哈希字符串
///
/// # 哈希逻辑
/// - 字符串：根据大小写敏感设置进行标准化
/// - 浮点数：根据容差进行舍入处理
/// - 整数：根据容差进行调整
/// - 布尔值：直接使用原始值
/// - 列表：递归处理每个元素并考虑长度
///
/// # 示例
/// ```ignore
/// let hash = compute_params_hash(&params, &config);
/// // 返回类似 "a1b2c3d4" 的哈希字符串
/// ```
fn compute_params_hash(params: &HashMap<String, ParameterValue>, config: &Config) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    // 辅助函数：递归哈希单个ParameterValue
    fn hash_parameter_value(
        value: &ParameterValue,
        hasher: &mut std::collections::hash_map::DefaultHasher,
        config: &Config,
    ) {
        match value {
            ParameterValue::Basic(BasicParameterValue::String(s)) => {
                if config.tolerance.string_case_sensitive {
                    s.hash(hasher);
                } else {
                    s.to_lowercase().hash(hasher);
                }
            }
            ParameterValue::Basic(BasicParameterValue::Float(f)) => {
                // 对浮点数进行舍入，考虑容差
                let rounded = (f / config.tolerance.float_tolerance).round()
                    * config.tolerance.float_tolerance;
                rounded.to_bits().hash(hasher);
            }
            ParameterValue::Basic(BasicParameterValue::Int(i)) => {
                // 对整数进行处理，考虑容差
                let adjusted = i - (i % (config.tolerance.int_tolerance + 1));
                adjusted.hash(hasher);
            }
            ParameterValue::Basic(BasicParameterValue::Bool(b)) => b.hash(hasher),
            ParameterValue::List(list) => {
                // 对列表进行哈希
                list.len().hash(hasher);
                for item in list {
                    // 递归处理列表中的每个ParameterValue
                    hash_parameter_value(item, hasher, config);
                }
            }
        }
    }

    // 确定要哈希的参数
    let params_to_hash: HashMap<String, ParameterValue> =
        if let Some(ref grouping_params) = config.grouping.grouping_parameters {
            // 如果指定了分组参数，只哈希这些参数
            grouping_params
                .iter()
                .filter_map(|param| {
                    params
                        .get(param)
                        .map(|value| (param.clone(), value.clone()))
                })
                .collect()
        } else {
            // 如果没有指定分组参数，哈希所有参数
            params.clone()
        };

    // 将参数按键排序以获得一致的哈希
    let mut sorted_keys: Vec<_> = params_to_hash.keys().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        // 对键进行哈希
        key.hash(&mut hasher);

        // 对值进行哈希（使用equals_with_tolerance方法来考虑容差）
        let value = params_to_hash.get(key).unwrap();
        hash_parameter_value(value, &mut hasher, config);
    }

    // 将哈希值转换为字符串
    format!("{:x}", hasher.finish())
}

/// 比较两个参数集，返回差异参数的数量
///
/// 此函数比较两个参数映射，计算在考虑容差设置的情况下有多少参数不同。
/// 差异计算包括：
/// 1. 第一个参数集中存在但第二个参数集中不存在的参数
/// 2. 两个参数集中都存在但值不同的参数（考虑容差）
/// 3. 第二个参数集中存在但第一个参数集中不存在的参数
///
/// # 参数
/// * `params1` - 第一个参数映射
/// * `params2` - 第二个参数映射  
/// * `tolerance` - 包含容差配置的配置对象
///
/// # 返回值
/// * `usize` - 差异参数的总数量
///
/// # 示例
/// ```ignore
/// let diff_count = count_different_parameters(&params1, &params2, &config);
/// // 返回差异参数的数量，如 3
/// ```
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
        // 尝试将版本添加到现有组
        let mut added_to_existing = false;

        for group in &mut groups {
            // 检查是否可以添加到该组
            let can_add_to_group = if let Some(ref grouping_params) =
                config.grouping.grouping_parameters
            {
                // 如果指定了分组参数，基于参数结构进行分组
                // 检查两个版本是否都有相同的参数结构（即分组参数都存在）
                grouping_params.iter().all(|param| {
                    version.hparams.contains_key(param) && group.base_parameters.contains_key(param)
                })
            } else {
                // 如果没有指定分组参数，检查所有参数是否完全相同
                count_different_parameters(&version.hparams, &group.base_parameters, config) == 0
            };

            // 如果可以添加到该组，则添加
            if can_add_to_group {
                group.member_versions.push(version.clone());
                added_to_existing = true;
                break;
            }
        }

        // 如果没有添加到现有组，则创建新组
        if !added_to_existing {
            let group_id = compute_params_hash(&version.hparams, config);

            let new_group = ExperimentGroup {
                group_id,
                base_parameters: version.hparams.clone(),
                member_versions: vec![version],
            };

            groups.push(new_group);
        }
    }

    // 对每个组的成员按版本号排序
    for group in &mut groups {
        group
            .member_versions
            .sort_by(|a, b| a.version_num.cmp(&b.version_num));
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
            if i == j {
                continue;
            }

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
    use crate::models::{GroupingConfig, IgnoredConfig, ToleranceConfig};

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
                main_key: None,
            },
            diff: Default::default(),
            tui: Default::default(),
            keybindings: Default::default(),
            test_script: Default::default(),
        }
    }

    // 辅助函数：创建带有main_key的测试配置
    fn create_test_config_with_main_key(main_key: Option<Vec<String>>) -> Config {
        let mut config = create_test_config();
        config.grouping.main_key = main_key;
        config
    }

    // 测试过滤参数功能
    #[test]
    fn test_filter_parameters() {
        let mut hparams = HashMap::new();
        hparams.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        hparams.insert(
            "fold".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(1)),
        );
        hparams.insert(
            "devices".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(2)),
        );
        hparams.insert(
            "lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.001)),
        );

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
        hparams.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        hparams.insert(
            "trainer-max_epochs".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(100)),
        );
        hparams.insert(
            "trainer-devices".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(2)),
        ); // 应该被忽略
        hparams.insert(
            "config-optimizer".to_string(),
            ParameterValue::List(vec![
                ParameterValue::Basic(BasicParameterValue::String("adam".to_string())),
                ParameterValue::Basic(BasicParameterValue::String("sgd".to_string())),
            ]),
        );
        hparams.insert(
            "config-fold".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(3)),
        ); // 应该被忽略

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
        hparams.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        hparams.insert(
            "lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.001)),
        );
        hparams.insert(
            "batch_size".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(32)),
        );
        hparams.insert(
            "fold".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(1)),
        );

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
        params1.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        params1.insert(
            "trainer-lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.001)),
        );
        params1.insert(
            "trainer-epochs".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(100)),
        );

        let mut params2 = HashMap::new();
        params2.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        params2.insert(
            "trainer-lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.002)),
        ); // 不同的值
        params2.insert(
            "trainer-epochs".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(100)),
        );

        let different_params = count_different_parameters(&params1, &params2, &strict_config);

        assert_eq!(
            different_params, 1,
            "Only 'trainer-lr' parameter should be different"
        );
    }

    // 测试参数哈希计算
    #[test]
    fn test_compute_params_hash() {
        let config = create_test_config();
        let mut params1 = HashMap::new();
        params1.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        params1.insert(
            "lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.001)),
        );
        params1.insert(
            "batch_size".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(32)),
        );

        // 创建一个顺序不同但内容相同的参数集
        let mut params2 = HashMap::new();
        params2.insert(
            "batch_size".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(32)),
        );
        params2.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        params2.insert(
            "lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.001)),
        );

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

    // 测试没有配置main_key时，从所有版本中删除共有参数的功能
    #[test]
    fn test_remove_common_hparams_without_main_key() {
        // 准备测试环境
        let config = create_test_config();

        // 创建临时目录和文件用于测试
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");

        // 创建三个测试文件，它们有一些共同的参数和一些不同的参数
        let file1_path = temp_dir.path().join("version_001/hparams.yaml");
        let file2_path = temp_dir.path().join("version_002/hparams.yaml");
        let file3_path = temp_dir.path().join("version_003/hparams.yaml");

        // 创建目录结构
        std::fs::create_dir_all(file1_path.parent().unwrap()).expect("Failed to create directory");
        std::fs::create_dir_all(file2_path.parent().unwrap()).expect("Failed to create directory");
        std::fs::create_dir_all(file3_path.parent().unwrap()).expect("Failed to create directory");

        // 写入测试内容（共同参数：model=cnn, batch_size=32；不同参数：lr）
        std::fs::write(&file1_path, "model: cnn\nlr: 0.001\nbatch_size: 32\n")
            .expect("Failed to write file1");

        std::fs::write(&file2_path, "model: cnn\nlr: 0.01\nbatch_size: 32\n")
            .expect("Failed to write file2");

        std::fs::write(&file3_path, "model: cnn\nlr: 0.1\nbatch_size: 32\n")
            .expect("Failed to write file3");

        // 调用被测试的函数
        let hparams_files = vec![file1_path, file2_path, file3_path];
        let (versions, _group_common_hparams) = create_version_data_list(&config, &hparams_files)
            .expect("Failed to create version data list");

        // 验证结果：
        // 1. 应该有3个版本
        assert_eq!(versions.len(), 3, "Should have 3 versions");

        // 2. 所有版本都不应该包含共同参数（model和batch_size）
        for version in &versions {
            assert!(
                !version.hparams.contains_key("model"),
                "Common parameter 'model' should be removed"
            );
            assert!(
                !version.hparams.contains_key("batch_size"),
                "Common parameter 'batch_size' should be removed"
            );
        }

        // 3. 每个版本应该只包含不同的参数（lr）
        let lr_values: Vec<_> = versions
            .iter()
            .filter_map(|v| v.hparams.get("lr"))
            .collect();
        assert_eq!(
            lr_values.len(),
            3,
            "All versions should have 'lr' parameter"
        );

        // 清理临时文件
        temp_dir.close().expect("Failed to clean up temp directory");
    }

    // 测试完整的流程：create_version_data_list过滤参数，group_versions使用过滤后的参数
    #[test]
    fn test_full_flow_with_parameter_filtering() {
        // 创建测试配置，包含分组参数
        let mut config = create_test_config();
        config.grouping.grouping_parameters = Some(vec!["model".to_string(), "lr".to_string()]);
        config.ignored_parameters.parameters = vec!["fold".to_string()];

        // 创建临时目录和文件用于测试
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");

        // 创建两个测试文件，具有相同的模型但不同的学习率
        let file1 = temp_dir.path().join("version_001/hparams.yaml");
        let file2 = temp_dir.path().join("version_002/hparams.yaml");

        std::fs::create_dir_all(file1.parent().unwrap()).expect("Failed to create directory");
        std::fs::create_dir_all(file2.parent().unwrap()).expect("Failed to create directory");

        // 两个版本都有model=cnn，但学习率不同
        std::fs::write(
            &file1,
            "model: cnn\nlr: 0.001\nbatch_size: 32\nfold: 1\noptimizer: adam\n",
        )
        .expect("Failed to write file1");
        std::fs::write(
            &file2,
            "model: cnn\nlr: 0.01\nbatch_size: 64\nfold: 2\noptimizer: sgd\n",
        )
        .expect("Failed to write file2");

        // 调用create_version_data_list进行参数过滤
        let hparams_files = vec![file1, file2];
        let (versions, _) = create_version_data_list(&config, &hparams_files)
            .expect("Failed to create version data list");

        // 验证参数过滤结果
        assert_eq!(versions.len(), 2);

        for version in &versions {
            // 应该只包含分组参数（model和lr）
            assert!(version.hparams.contains_key("model"));
            assert!(version.hparams.contains_key("lr"));
            // 不应该包含非分组参数
            assert!(!version.hparams.contains_key("batch_size"));
            assert!(!version.hparams.contains_key("optimizer"));
            // 不应该包含忽略参数
            assert!(!version.hparams.contains_key("fold"));
        }

        // 调用group_versions进行分组
        let groups = group_versions(&config, versions).expect("Failed to group versions");

        // 验证分组结果
        // 由于两个版本的model相同，lr不同，应该被分到同一组
        assert_eq!(
            groups.len(),
            1,
            "Expected 1 group, but got {} groups. This means the grouping logic is not working as expected with filtered parameters.",
            groups.len()
        );
        assert_eq!(groups[0].member_versions.len(), 2);

        // 清理临时文件
        temp_dir.close().expect("Failed to clean up temp directory");
    }

    // 测试配置了main_key时，按main_key分组并在分组内删除共有参数的功能
    #[test]
    fn test_remove_common_hparams_with_main_key() {
        // 准备测试环境，配置main_key为["model"]
        let config = create_test_config_with_main_key(Some(vec!["model".to_string()]));

        // 创建临时目录和文件用于测试
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");

        // 创建多个测试文件，具有不同的main_key值（model）
        // CNN模型组：
        let cnn_file1 = temp_dir.path().join("version_001/hparams.yaml");
        let cnn_file2 = temp_dir.path().join("version_002/hparams.yaml");
        // RNN模型组：
        let rnn_file1 = temp_dir.path().join("version_003/hparams.yaml");
        let rnn_file2 = temp_dir.path().join("version_004/hparams.yaml");

        // 创建目录结构
        std::fs::create_dir_all(cnn_file1.parent().unwrap()).expect("Failed to create directory");
        std::fs::create_dir_all(cnn_file2.parent().unwrap()).expect("Failed to create directory");
        std::fs::create_dir_all(rnn_file1.parent().unwrap()).expect("Failed to create directory");
        std::fs::create_dir_all(rnn_file2.parent().unwrap()).expect("Failed to create directory");

        // CNN组 - 共同参数：batch_size=32；不同参数：lr
        std::fs::write(
            &cnn_file1,
            "model: cnn\nlr: 0.001\nbatch_size: 32\noptimizer: adam\n",
        )
        .expect("Failed to write cnn_file1");

        std::fs::write(
            &cnn_file2,
            "model: cnn\nlr: 0.01\nbatch_size: 32\noptimizer: adam\n",
        )
        .expect("Failed to write cnn_file2");

        // RNN组 - 共同参数：hidden_size=128；不同参数：lr
        std::fs::write(
            &rnn_file1,
            "model: rnn\nlr: 0.005\nhidden_size: 128\ndropout: 0.2\n",
        )
        .expect("Failed to write rnn_file1");

        std::fs::write(
            &rnn_file2,
            "model: rnn\nlr: 0.05\nhidden_size: 128\ndropout: 0.2\n",
        )
        .expect("Failed to write rnn_file2");

        // 调用被测试的函数
        let hparams_files = vec![
            cnn_file1.clone(),
            cnn_file2.clone(),
            rnn_file1.clone(),
            rnn_file2.clone(),
        ];
        let (versions, group_common_hparams) = create_version_data_list(&config, &hparams_files)
            .expect("Failed to create version data list");

        // 验证分组内相同hparams数据
        // 应该有2个分组包含共同参数（cnn和rnn）
        assert_eq!(group_common_hparams.len(), 2);

        // 检查cnn组的共同参数
        let cnn_group_key = "model=cnn";
        if let Some(common_params) = group_common_hparams.get(cnn_group_key) {
            assert_eq!(common_params.len(), 2);
            assert!(common_params.contains_key("batch_size"));
            assert!(common_params.contains_key("optimizer"));
        } else {
            panic!(
                "CNN group common parameters not found with key '{}'",
                cnn_group_key
            );
        }

        // 检查rnn组的共同参数
        let rnn_group_key = "model=rnn";
        if let Some(common_params) = group_common_hparams.get(rnn_group_key) {
            assert_eq!(common_params.len(), 2);
            assert!(common_params.contains_key("hidden_size"));
            assert!(common_params.contains_key("dropout"));
        } else {
            panic!(
                "RNN group common parameters not found with key '{}'",
                rnn_group_key
            );
        }

        // 验证结果：
        // 1. 应该有4个版本
        assert_eq!(versions.len(), 4, "Should have 4 versions");

        // 2. 分离CNN组和RNN组的版本
        let cnn_versions: Vec<_> = versions
            .iter()
            .filter(|v| {
                v.hparams.get("model")
                    == Some(&ParameterValue::Basic(BasicParameterValue::String(
                        "cnn".to_string(),
                    )))
            })
            .collect();
        let rnn_versions: Vec<_> = versions
            .iter()
            .filter(|v| {
                v.hparams.get("model")
                    == Some(&ParameterValue::Basic(BasicParameterValue::String(
                        "rnn".to_string(),
                    )))
            })
            .collect();

        assert_eq!(cnn_versions.len(), 2, "Should have 2 CNN versions");
        assert_eq!(rnn_versions.len(), 2, "Should have 2 RNN versions");

        // 3. CNN组内：
        // - 应该保留model（main_key）
        // - 应该删除共同参数batch_size和optimizer
        // - 应该保留不同参数lr
        for version in &cnn_versions {
            assert!(
                version.hparams.contains_key("model"),
                "CNN version should keep 'model' main_key"
            );
            assert!(
                !version.hparams.contains_key("batch_size"),
                "CNN version should remove common parameter 'batch_size'"
            );
            assert!(
                !version.hparams.contains_key("optimizer"),
                "CNN version should remove common parameter 'optimizer'"
            );
            assert!(
                version.hparams.contains_key("lr"),
                "CNN version should keep different parameter 'lr'"
            );
        }

        // 4. RNN组内：
        // - 应该保留model（main_key）
        // - 应该删除共同参数hidden_size和dropout
        // - 应该保留不同参数lr
        for version in &rnn_versions {
            assert!(
                version.hparams.contains_key("model"),
                "RNN version should keep 'model' main_key"
            );
            assert!(
                !version.hparams.contains_key("hidden_size"),
                "RNN version should remove common parameter 'hidden_size'"
            );
            assert!(
                !version.hparams.contains_key("dropout"),
                "RNN version should remove common parameter 'dropout'"
            );
            assert!(
                version.hparams.contains_key("lr"),
                "RNN version should keep different parameter 'lr'"
            );
        }

        // 清理临时文件
        temp_dir.close().expect("Failed to clean up temp directory");
    }

    // 测试参数过滤逻辑在create_version_data_list中的正确应用
    #[test]
    fn test_parameter_filtering_in_create_version_data_list() {
        // 创建测试配置，包含忽略参数和分组参数
        let mut config = create_test_config();
        config.grouping.grouping_parameters = Some(vec!["model".to_string(), "lr".to_string()]);
        config.ignored_parameters.parameters = vec!["fold".to_string(), "devices".to_string()];

        // 创建临时目录和文件用于测试
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");

        // 创建测试文件，包含各种参数
        let file_path = temp_dir.path().join("version_001/hparams.yaml");
        std::fs::create_dir_all(file_path.parent().unwrap()).expect("Failed to create directory");

        // 写入测试内容：包含分组参数、忽略参数和其他参数
        let yaml_content =
            "model: cnn\nlr: 0.001\nbatch_size: 32\nfold: 1\ndevices: 2\noptimizer: adam\n";
        std::fs::write(&file_path, yaml_content).expect("Failed to write test file");

        // 调用被测试的函数
        let hparams_files = vec![file_path];
        let (versions, _) = create_version_data_list(&config, &hparams_files)
            .expect("Failed to create version data list");

        // 验证结果
        assert_eq!(versions.len(), 1, "Should have 1 version");

        let version = &versions[0];

        // 应该只包含分组参数（model和lr），不包含忽略参数和其他参数
        assert!(
            version.hparams.contains_key("model"),
            "Should contain 'model' parameter. Actual keys: {:?}",
            version.hparams.keys().collect::<Vec<_>>()
        );
        assert!(
            version.hparams.contains_key("lr"),
            "Should contain 'lr' parameter. Actual keys: {:?}",
            version.hparams.keys().collect::<Vec<_>>()
        );
        assert!(
            !version.hparams.contains_key("batch_size"),
            "Should not contain 'batch_size' parameter (not in grouping_params)"
        );
        assert!(
            !version.hparams.contains_key("fold"),
            "Should not contain 'fold' parameter (ignored)"
        );
        assert!(
            !version.hparams.contains_key("devices"),
            "Should not contain 'devices' parameter (ignored)"
        );
        assert!(
            !version.hparams.contains_key("optimizer"),
            "Should not contain 'optimizer' parameter (not in grouping_params)"
        );

        // 清理临时文件
        temp_dir.close().expect("Failed to clean up temp directory");
    }

    // 测试配置了多个main_key时，按main_key组合值分组并在分组内删除共有参数的功能
    #[test]
    fn test_remove_common_hparams_with_multiple_main_keys() {
        // 准备测试环境，配置main_key为["model", "dataset"]
        let config = create_test_config_with_main_key(Some(vec![
            "model".to_string(),
            "dataset".to_string(),
        ]));

        // 创建临时目录和文件用于测试
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");

        // 创建多个测试文件，具有不同的main_key组合值
        // CNN+MNIST组：
        let cnn_mnist_file1 = temp_dir.path().join("version_001/hparams.yaml");
        let cnn_mnist_file2 = temp_dir.path().join("version_002/hparams.yaml");
        // CNN+CIFAR10组：
        let cnn_cifar_file1 = temp_dir.path().join("version_003/hparams.yaml");
        let cnn_cifar_file2 = temp_dir.path().join("version_004/hparams.yaml");
        // RNN+MNIST组：
        let rnn_mnist_file1 = temp_dir.path().join("version_005/hparams.yaml");

        // 创建目录结构
        std::fs::create_dir_all(cnn_mnist_file1.parent().unwrap())
            .expect("Failed to create directory");
        std::fs::create_dir_all(cnn_mnist_file2.parent().unwrap())
            .expect("Failed to create directory");
        std::fs::create_dir_all(cnn_cifar_file1.parent().unwrap())
            .expect("Failed to create directory");
        std::fs::create_dir_all(cnn_cifar_file2.parent().unwrap())
            .expect("Failed to create directory");
        std::fs::create_dir_all(rnn_mnist_file1.parent().unwrap())
            .expect("Failed to create directory");

        // CNN+MNIST组 - 共同参数：optimizer=adam；不同参数：lr
        std::fs::write(
            &cnn_mnist_file1,
            "model: cnn\ndataset: mnist\nlr: 0.001\noptimizer: adam\nbatch_size: 32\n",
        )
        .expect("Failed to write cnn_mnist_file1");

        std::fs::write(
            &cnn_mnist_file2,
            "model: cnn\ndataset: mnist\nlr: 0.01\noptimizer: adam\nbatch_size: 32\n",
        )
        .expect("Failed to write cnn_mnist_file2");

        // CNN+CIFAR10组 - 共同参数：optimizer=sgt；不同参数：lr
        std::fs::write(
            &cnn_cifar_file1,
            "model: cnn\ndataset: cifar10\nlr: 0.005\noptimizer: sgd\nbatch_size: 64\n",
        )
        .expect("Failed to write cnn_cifar_file1");

        std::fs::write(
            &cnn_cifar_file2,
            "model: cnn\ndataset: cifar10\nlr: 0.05\noptimizer: sgd\nbatch_size: 64\n",
        )
        .expect("Failed to write cnn_cifar_file2");

        // RNN+MNIST组
        std::fs::write(
            &rnn_mnist_file1,
            "model: rnn\ndataset: mnist\nlr: 0.003\noptimizer: adam\nhidden_size: 128\n",
        )
        .expect("Failed to write rnn_mnist_file1");

        // 调用被测试的函数
        let hparams_files = vec![
            cnn_mnist_file1.clone(),
            cnn_mnist_file2.clone(),
            cnn_cifar_file1.clone(),
            cnn_cifar_file2.clone(),
            rnn_mnist_file1.clone(),
        ];
        let (versions, group_common_hparams) = create_version_data_list(&config, &hparams_files)
            .expect("Failed to create version data list");

        // 验证分组内相同hparams数据
        // 应该有2个分组包含共同参数（CNN+MNIST和CNN+CIFAR10）
        assert_eq!(group_common_hparams.len(), 2);

        // 检查CNN+MNIST组的共同参数
        let cnn_mnist_group_key = "model=cnn, dataset=mnist";
        if let Some(common_params) = group_common_hparams.get(cnn_mnist_group_key) {
            assert_eq!(common_params.len(), 2);
            assert!(common_params.contains_key("optimizer"));
            assert!(common_params.contains_key("batch_size"));
        } else {
            panic!(
                "CNN+MNIST group common parameters not found with key '{}'",
                cnn_mnist_group_key
            );
        }

        // 检查CNN+CIFAR10组的共同参数
        let cnn_cifar_group_key = "model=cnn, dataset=cifar10";
        if let Some(common_params) = group_common_hparams.get(cnn_cifar_group_key) {
            assert_eq!(common_params.len(), 2);
            assert!(common_params.contains_key("optimizer"));
            assert!(common_params.contains_key("batch_size"));
        } else {
            panic!(
                "CNN+CIFAR10 group common parameters not found with key '{}'",
                cnn_cifar_group_key
            );
        }

        // 验证结果：
        // 1. 应该有5个版本
        assert_eq!(versions.len(), 5, "Should have 5 versions");

        // 2. 按main_key组合值分组
        let cnn_mnist_versions: Vec<_> = versions
            .iter()
            .filter(|v| {
                v.hparams.get("model")
                    == Some(&ParameterValue::Basic(BasicParameterValue::String(
                        "cnn".to_string(),
                    )))
                    && v.hparams.get("dataset")
                        == Some(&ParameterValue::Basic(BasicParameterValue::String(
                            "mnist".to_string(),
                        )))
            })
            .collect();

        let cnn_cifar_versions: Vec<_> = versions
            .iter()
            .filter(|v| {
                v.hparams.get("model")
                    == Some(&ParameterValue::Basic(BasicParameterValue::String(
                        "cnn".to_string(),
                    )))
                    && v.hparams.get("dataset")
                        == Some(&ParameterValue::Basic(BasicParameterValue::String(
                            "cifar10".to_string(),
                        )))
            })
            .collect();

        let rnn_mnist_versions: Vec<_> = versions
            .iter()
            .filter(|v| {
                v.hparams.get("model")
                    == Some(&ParameterValue::Basic(BasicParameterValue::String(
                        "rnn".to_string(),
                    )))
                    && v.hparams.get("dataset")
                        == Some(&ParameterValue::Basic(BasicParameterValue::String(
                            "mnist".to_string(),
                        )))
            })
            .collect();

        assert_eq!(
            cnn_mnist_versions.len(),
            2,
            "Should have 2 CNN+MNIST versions"
        );
        assert_eq!(
            cnn_cifar_versions.len(),
            2,
            "Should have 2 CNN+CIFAR10 versions"
        );
        assert_eq!(
            rnn_mnist_versions.len(),
            1,
            "Should have 1 RNN+MNIST version"
        );

        // 3. CNN+MNIST组内：
        // - 应该保留model和dataset（main_key）
        // - 应该删除共同参数optimizer和batch_size
        // - 应该保留不同参数lr
        for version in &cnn_mnist_versions {
            assert!(
                version.hparams.contains_key("model"),
                "CNN+MNIST version should keep 'model' main_key"
            );
            assert!(
                version.hparams.contains_key("dataset"),
                "CNN+MNIST version should keep 'dataset' main_key"
            );
            assert!(
                !version.hparams.contains_key("optimizer"),
                "CNN+MNIST version should remove common parameter 'optimizer'"
            );
            assert!(
                !version.hparams.contains_key("batch_size"),
                "CNN+MNIST version should remove common parameter 'batch_size'"
            );
            assert!(
                version.hparams.contains_key("lr"),
                "CNN+MNIST version should keep different parameter 'lr'"
            );
        }

        // 4. CNN+CIFAR10组内：
        // - 应该保留model和dataset（main_key）
        // - 应该删除共同参数optimizer和batch_size
        // - 应该保留不同参数lr
        for version in &cnn_cifar_versions {
            assert!(
                version.hparams.contains_key("model"),
                "CNN+CIFAR10 version should keep 'model' main_key"
            );
            assert!(
                version.hparams.contains_key("dataset"),
                "CNN+CIFAR10 version should keep 'dataset' main_key"
            );
            assert!(
                !version.hparams.contains_key("optimizer"),
                "CNN+CIFAR10 version should remove common parameter 'optimizer'"
            );
            assert!(
                !version.hparams.contains_key("batch_size"),
                "CNN+CIFAR10 version should remove common parameter 'batch_size'"
            );
            assert!(
                version.hparams.contains_key("lr"),
                "CNN+CIFAR10 version should keep different parameter 'lr'"
            );
        }

        // 5. RNN+MNIST组（只有一个版本）：
        // - 应该保留所有参数（因为没有其他版本来比较共有参数）
        if let Some(version) = rnn_mnist_versions.first() {
            assert!(
                version.hparams.contains_key("model"),
                "RNN+MNIST version should keep 'model' main_key"
            );
            assert!(
                version.hparams.contains_key("dataset"),
                "RNN+MNIST version should keep 'dataset' main_key"
            );
            assert!(
                version.hparams.contains_key("optimizer"),
                "RNN+MNIST version should keep all parameters (only one version)"
            );
            assert!(
                version.hparams.contains_key("hidden_size"),
                "RNN+MNIST version should keep all parameters (only one version)"
            );
            assert!(
                version.hparams.contains_key("lr"),
                "RNN+MNIST version should keep all parameters (only one version)"
            );
        }

        // 清理临时文件
        temp_dir.close().expect("Failed to clean up temp directory");
    }

    // 测试配置了main_key但某个版本缺少该键时，应该报错
    #[test]
    fn test_missing_main_key_should_error() {
        // 准备测试环境，配置main_key为["model"]
        let config = create_test_config_with_main_key(Some(vec!["model".to_string()]));

        // 创建临时目录和文件用于测试
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");

        // 创建两个测试文件，一个包含model键，一个不包含
        let valid_file = temp_dir.path().join("version_001/hparams.yaml");
        let invalid_file = temp_dir.path().join("version_002/hparams.yaml");

        // 创建目录结构
        std::fs::create_dir_all(valid_file.parent().unwrap()).expect("Failed to create directory");
        std::fs::create_dir_all(invalid_file.parent().unwrap())
            .expect("Failed to create directory");

        // 写入测试内容
        std::fs::write(&valid_file, "model: cnn\nlr: 0.001\n").expect("Failed to write valid_file");

        // 故意不包含model键
        std::fs::write(&invalid_file, "lr: 0.01\nbatch_size: 32\n")
            .expect("Failed to write invalid_file");

        // 调用被测试的函数，应该返回错误
        let hparams_files = vec![valid_file, invalid_file];
        let result = create_version_data_list(&config, &hparams_files);

        // 验证结果应该是错误
        assert!(
            result.is_err(),
            "Should return error when version is missing main_key"
        );
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("missing required main_key"),
            "Error message should indicate missing main_key"
        );

        // 清理临时文件
        temp_dir.close().expect("Failed to clean up temp directory");
    }

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
        group1.base_parameters.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        group1.base_parameters.insert(
            "lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.001)),
        );
        group1.base_parameters.insert(
            "batch_size".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(32)),
        );

        let mut group2 = ExperimentGroup {
            group_id: "group_2".to_string(),
            member_versions: vec![version2],
            base_parameters: HashMap::new(),
        };
        group2.base_parameters.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("cnn".to_string())),
        );
        group2.base_parameters.insert(
            "lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.001)),
        );
        group2.base_parameters.insert(
            "batch_size".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(64)),
        ); // 与group1不同

        let mut group3 = ExperimentGroup {
            group_id: "group_3".to_string(),
            member_versions: vec![version3],
            base_parameters: HashMap::new(),
        };
        group3.base_parameters.insert(
            "model".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("rnn".to_string())),
        );
        group3.base_parameters.insert(
            "lr".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.002)),
        );
        group3.base_parameters.insert(
            "optimizer".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("adam".to_string())),
        );

        let groups = vec![group1, group2, group3];
        let similar_groups = find_similar_groups(&groups, &config);

        // 验证group1和group2是否相似
        assert!(
            similar_groups.get("group_1").is_some(),
            "Group 1 should have similar groups"
        );
        let group1_similar = similar_groups.get("group_1").unwrap();
        assert!(
            group1_similar.contains(&"group_2".to_string()),
            "Group 1 should be similar to Group 2"
        );

        // group3应该与其他组不相似
        if let Some(group3_similar) = similar_groups.get("group_3") {
            assert!(
                group3_similar.is_empty(),
                "Group 3 should not be similar to any other group"
            );
        }
    }

    // 测试嵌套map展开功能
    #[test]
    fn test_nested_map_flattening() {
        // 由于ParameterValue已经不包含复杂的嵌套结构，这个测试不再适用
        // 保留函数体但不执行任何操作
        assert!(true);
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

            if let ParameterValue::Basic(BasicParameterValue::String(optimizer_name)) =
                &optimizers_list[0]
            {
                assert_eq!(optimizer_name, "adam");
            } else {
                panic!("First optimizer should be a Basic String");
            }
        } else {
            panic!("optimizers should be a List");
        }
    }
}
