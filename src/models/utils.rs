use serde::{Deserialize, Deserializer};

/// 反序列化可选字符串，将空字符串转换为None
///
/// # 参数
/// - `deserializer`: 用于反序列化的serde反序列化器
///
/// # 返回值
/// 反序列化后的可选字符串，如果原字符串为空则返回None
pub fn deserialize_optional_string<'de, D>(deserializer: D) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.filter(|s| !s.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_yaml;
    
    // 测试结构体，用于测试deserialize_optional_string函数
    #[derive(Debug, Deserialize)]
    struct TestStruct {
        #[serde(default, deserialize_with = "deserialize_optional_string")]
        field: Option<String>,
    }
    
    #[test]
    fn test_deserialize_optional_string_with_content() {
        let yaml = "field: test_value";
        let test: TestStruct = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(test.field, Some("test_value".to_string()));
    }
    
    #[test]
    fn test_deserialize_optional_string_with_empty() {
        let yaml = "field: ''";
        let test: TestStruct = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(test.field, None);
    }
    
    #[test]
    fn test_deserialize_optional_string_with_missing() {
        let yaml = "";
        let test: TestStruct = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(test.field, None);
    }
}