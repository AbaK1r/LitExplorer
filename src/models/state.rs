use crate::models::config::Config;
use crate::models::models::{VersionData, ExperimentGroup};

/// 应用程序状态结构，包含所有实验数据和配置
#[derive(Debug)]
pub struct AppState {
    pub all_versions: Vec<VersionData>,
    pub experiment_groups: Vec<ExperimentGroup>,
    pub config: Config,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use crate::models::parameter_value::{ParameterValue, BasicParameterValue};
    
    #[test]
    fn test_app_state_creation() {
        // 创建一个简单的配置
        let config = Config::default();
        
        // 创建一些版本数据
        let mut versions = Vec::new();
        let mut hparams = HashMap::new();
        hparams.insert("learning_rate".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.01)));
        
        let version = VersionData {
            version_num: 1,
            path: PathBuf::from("logs/version_1"),
            hparams,
        };
        versions.push(version);
        
        // 创建一个实验组
        let mut groups = Vec::new();
        let mut base_params = HashMap::new();
        base_params.insert("model_type".to_string(), ParameterValue::Basic(BasicParameterValue::String("CNN".to_string())));
        
        let group = ExperimentGroup {
            group_id: "group_1".to_string(),
            base_parameters: base_params,
            member_versions: versions.clone(),
        };
        groups.push(group);
        
        // 创建AppState
        let app_state = AppState {
            all_versions: versions,
            experiment_groups: groups,
            config,
        };
        
        assert_eq!(app_state.all_versions.len(), 1);
        assert_eq!(app_state.experiment_groups.len(), 1);
    }
}