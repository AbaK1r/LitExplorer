use std::path::PathBuf;
use std::collections::HashMap;
use crate::models::parameter_value::ParameterValue;

/// 版本数据结构，包含实验版本的相关信息
#[derive(Debug, Clone, PartialEq)]
pub struct VersionData {
    pub version_num: u32,
    pub path: PathBuf,
    pub hparams: HashMap<String, ParameterValue>,
}

/// 实验组结构，包含一组相关的实验版本
#[derive(Debug, PartialEq)]
pub struct ExperimentGroup {
    pub group_id: String,
    pub base_parameters: HashMap<String, ParameterValue>,
    pub member_versions: Vec<VersionData>,
}

#[cfg(test)]
mod tests {
    use crate::models::BasicParameterValue;

    use super::*;
    
    #[test]
    fn test_version_data_creation() {
        let mut hparams = HashMap::new();
        hparams.insert("learning_rate".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.01)));
        
        let version = VersionData {
            version_num: 1,
            path: PathBuf::from("logs/version_1"),
            hparams,
        };
        
        assert_eq!(version.version_num, 1);
        assert_eq!(version.path.to_str().unwrap(), "logs/version_1");
        assert!(version.hparams.contains_key("learning_rate"));
    }
    
    #[test]
    fn test_experiment_group_creation() {
        let mut base_params = HashMap::new();
        base_params.insert("model_type".to_string(), ParameterValue::Basic(BasicParameterValue::String("CNN".to_string())));
        
        let mut hparams = HashMap::new();
        hparams.insert("learning_rate".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.01)));
        
        let version = VersionData {
            version_num: 1,
            path: PathBuf::from("logs/version_1"),
            hparams,
        };
        
        let group = ExperimentGroup {
            group_id: "group_1".to_string(),
            base_parameters: base_params,
            member_versions: vec![version],
        };
        
        assert_eq!(group.group_id, "group_1");
        assert!(group.base_parameters.contains_key("model_type"));
        assert_eq!(group.member_versions.len(), 1);
    }
}