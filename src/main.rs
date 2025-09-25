// src/main.rs
mod config;
mod experiment_grouping;
mod file_utils;
mod models;
mod yaml_parser;

use anyhow::Result;
use config::load_config;
use experiment_grouping::{create_version_data_list, find_similar_groups, group_versions};
use file_utils::find_hparams_files;
use models::AppState;

fn main() -> Result<()> {
    // 加载配置文件
    let config = load_config("lightning_explorer.toml")?;
    println!("Configuration loaded successfully!");
    println!("Log directory: {}", config.general.log_dir);

    // 查找所有hparams.yaml文件
    let hparams_files = find_hparams_files(&config.general.log_dir, &config.general.hparams_file)?;
    println!("Found {} hparams files:", hparams_files.len());

    // 创建VersionData列表并获取分组内相同hparams数据
    let (version_data_list, group_common_hparams) =
        create_version_data_list(&config, &hparams_files)?;
    println!(
        "Successfully created {} version data entries",
        version_data_list.len()
    );
    if !group_common_hparams.is_empty() {
        println!(
            "Found {} main_key groups with common parameters",
            group_common_hparams.len()
        );
    }

    // 对版本进行分组（克隆version_data_list以保留所有权）
    let experiment_groups = group_versions(&config, version_data_list.clone())?;
    println!("Found {} experiment groups", experiment_groups.len());

    // 打印分组结果
    for (i, group) in experiment_groups.iter().enumerate() {
        let version_nums: Vec<_> = group
            .member_versions
            .iter()
            .map(|v| v.version_num)
            .collect();
        println!(
            "Group {} ({} versions): {:?}",
            i + 1,
            group.member_versions.len(),
            version_nums
        );

        // 如果组内有多个版本，可以打印共同的参数
        if group.member_versions.len() > 1 {
            println!("  Common parameters (ignoring specified parameters):");
            // 只打印前几个关键参数以避免输出过多
            let mut param_count = 0;
            for (key, value) in &group.base_parameters {
                if param_count < 10 {
                    // 限制打印参数数量
                    println!("    {}: {}", key, value.to_simple_string());
                    param_count += 1;
                } else {
                    println!(
                        "    ... and {} more parameters",
                        group.base_parameters.len() - param_count
                    );
                    break;
                }
            }
        }
    }

    // 查找相似组
    let similar_groups = find_similar_groups(&experiment_groups, &config);

    // 打印相似组信息
    let mut has_similar_groups = false;
    for (group_id, similar_ids) in similar_groups {
        if !similar_ids.is_empty() {
            if !has_similar_groups {
                println!("\nSimilar experiment groups:");
                has_similar_groups = true;
            }

            // 找到对应的组索引
            if let Some(group_idx) = experiment_groups
                .iter()
                .position(|g| g.group_id == group_id)
            {
                let similar_indices: Vec<_> = similar_ids
                    .iter()
                    .filter_map(|id| experiment_groups.iter().position(|g| g.group_id == *id))
                    .collect();

                if !similar_indices.is_empty() {
                    print!("  Group {} is similar to: ", group_idx + 1);
                    for (i, idx) in similar_indices.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("Group {}", idx + 1);
                    }
                    println!();
                }
            }
        }
    }

    if !has_similar_groups {
        println!("\nNo similar experiment groups found within the similarity threshold");
    }

    // 创建AppState实例，保存所有实验数据和配置
    let _app_state = AppState {
        all_versions: version_data_list,
        experiment_groups,
        config,
        group_common_hparams,
    };
    println!(
        "AppState created successfully:\n{:?}",
        _app_state.group_common_hparams
    );
    Ok(())
}
