use crate::models::config::Config;
use crate::models::models::{ExperimentGroup, VersionData};
use crate::models::parameter_value::ParameterValue;
use std::collections::{HashMap, VecDeque};

/// 应用程序状态结构，包含所有实验数据和配置
#[derive(Debug)]
pub struct AppState {
    // ————————————————————————————————————————————————————————————————————————
    // 所有实验版本数据的集合，包含完整的实验信息
    // ————————————————————————————————————————————————————————————————————————
    pub all_versions: Vec<VersionData>,
    // ————————————————————————————————————————————————————————————————————————
    // 实验分组列表，将相似的实验组织在一起
    // ————————————————————————————————————————————————————————————————————————
    pub experiment_groups: Vec<ExperimentGroup>,
    // ————————————————————————————————————————————————————————————————————————
    // 应用程序配置，包含所有可配置参数
    // ————————————————————————————————————————————————————————————————————————
    pub config: Config,
    // ————————————————————————————————————————————————————————————————————————
    // 存储每个main_key分组内的相同hparams数据
    // 键为分组键（由main_key值组合而成），值为该分组内所有版本共有的参数
    // ————————————————————————————————————————————————————————————————————————
    pub group_common_hparams: HashMap<String, HashMap<String, ParameterValue>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::parameter_value::{BasicParameterValue, ParameterValue};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_app_state_creation() {
        // 创建一个简单的配置
        let config = Config::default();

        // 创建一些版本数据
        let mut versions = Vec::new();
        let mut hparams = HashMap::new();
        hparams.insert(
            "learning_rate".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.01)),
        );

        let version = VersionData {
            version_num: 1,
            path: PathBuf::from("logs/version_1"),
            hparams,
        };
        versions.push(version);

        // 创建一个实验组
        let mut groups = Vec::new();
        let mut base_params = HashMap::new();
        base_params.insert(
            "model_type".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("CNN".to_string())),
        );

        let group = ExperimentGroup {
            group_id: "group_1".to_string(),
            base_parameters: base_params,
            member_versions: versions.clone(),
        };
        groups.push(group);

        // 创建分组共有参数
        let mut group_common_hparams = HashMap::new();
        let mut common_params = HashMap::new();
        common_params.insert(
            "optimizer".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("adam".to_string())),
        );
        group_common_hparams.insert("model_type=CNN".to_string(), common_params);

        // 创建AppState
        let app_state = AppState {
            all_versions: versions,
            experiment_groups: groups,
            config,
            group_common_hparams,
        };

        assert_eq!(app_state.all_versions.len(), 1);
        assert_eq!(app_state.experiment_groups.len(), 1);
    }
}
