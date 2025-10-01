// use std::fmt;
use crate::models::config::ToleranceConfig;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;

/// 参数值类型枚举，支持递归结构
#[derive(Clone, PartialEq)]
pub enum ParameterValue {
    // ————————————————————————————————————————————————————————————————————————
    // 基本参数值类型，包含字符串、数字、布尔值等基本类型
    // ————————————————————————————————————————————————————————————————————————
    Basic(BasicParameterValue),
    // ————————————————————————————————————————————————————————————————————————
    // 参数值列表类型，支持嵌套的参数值数组
    // ————————————————————————————————————————————————————————————————————————
    List(Vec<ParameterValue>),
}

/// 基本参数值类型，用于List中，只包含基本类型
#[derive(Clone, PartialEq)]
pub enum BasicParameterValue {
    String(String), // 字符串类型参数值
    Float(f64),     // 浮点数类型参数值
    Int(i64),       // 整数类型参数值
    Bool(bool),     // 布尔类型参数值
}

/// 为BasicParameterValue实现Debug trait，使用Display的格式
impl fmt::Debug for BasicParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
impl BasicParameterValue {
    pub fn to_string_repr(&self) -> String {
        match self {
            BasicParameterValue::String(s) => s.clone(),
            BasicParameterValue::Float(n) => format!("{:.6}", n),
            BasicParameterValue::Int(n) => n.to_string(),
            BasicParameterValue::Bool(b) => b.to_string(),
        }
    }
}

/// 为BasicParameterValue实现Display trait，支持format!("{}", value)语法
impl fmt::Display for BasicParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_repr())
    }
}

impl ParameterValue {
    pub fn to_simple_string(&self) -> String {
        match self {
            ParameterValue::Basic(basic_value) => basic_value.to_string_repr(),
            ParameterValue::List(list) => {
                let items: Vec<String> = list.iter().map(|item| item.to_simple_string()).collect();
                format!("[{}]", items.join(", "))
            }
        }
    }
}

/// 为ParameterValue实现Debug trait，使用Display的格式
impl fmt::Debug for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterValue::Basic(basic_value) => write!(f, "{}", basic_value),
            ParameterValue::List(list) => {
                let items: Vec<String> = list.iter().map(|item| item.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
        }
    }
}

impl From<&BasicParameterValue> for JsonValue {
    fn from(val: &BasicParameterValue) -> Self {
        match val {
            BasicParameterValue::String(s) => JsonValue::String(s.clone()),
            BasicParameterValue::Float(f) => serde_json::Number::from_f64(*f)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null),
            BasicParameterValue::Int(i) => JsonValue::Number((*i).into()),
            BasicParameterValue::Bool(b) => JsonValue::Bool(*b),
        }
    }
}

impl From<&ParameterValue> for JsonValue {
    fn from(val: &ParameterValue) -> Self {
        match val {
            ParameterValue::Basic(basic) => basic.into(),
            ParameterValue::List(list) => {
                JsonValue::Array(list.iter().map(|item| item.into()).collect())
            }
        }
    }
}

/// 将参数映射格式化为美观的JSON字符串并打印到控制台
///
/// 此函数将HashMap中的参数值转换为JSON格式，并使用serde_json的漂亮打印功能
/// 输出格式化的JSON字符串，便于调试和查看参数结构
///
/// # 参数
/// * `hparams` - 包含参数名和参数值的HashMap
///
/// # 返回值
/// * `Result<(), serde_json::Error>` - 成功时返回Ok(())，序列化失败时返回错误
///
/// # 示例
/// ```ignore
/// let mut params = HashMap::new();/// params.insert("learning_rate".to_string(), ParameterValue::Basic(BasicParameterValue::Float(0.001)));
/// print_hparams_pretty(&params)?;
/// // 输出:
/// // {
/// //   "learning_rate": 0.001
/// // }
/// ```
pub fn print_hparams_pretty(
    hparams: &HashMap<String, ParameterValue>,
) -> Result<(), serde_json::Error> {
    let json_value: JsonValue = hparams
        .iter()
        .map(|(k, v)| (k.clone(), v.into()))
        .collect::<serde_json::Map<_, _>>()
        .into();

    println!("{}", serde_json::to_string_pretty(&json_value)?);
    Ok(())
}

impl BasicParameterValue {
    /// 考虑容差的相等性比较
    pub fn equals_with_tolerance(&self, other: &Self, tolerance: &ToleranceConfig) -> bool {
        match (self, other) {
            (BasicParameterValue::String(a), BasicParameterValue::String(b)) => {
                if tolerance.string_case_sensitive {
                    a == b
                } else {
                    a.to_lowercase() == b.to_lowercase()
                }
            }
            (BasicParameterValue::Float(a), BasicParameterValue::Float(b)) => {
                // 处理NaN: 如果任一值是NaN，则只有两个都是NaN时才相等
                if a.is_nan() || b.is_nan() {
                    return a.is_nan() && b.is_nan();
                }
                (a - b).abs() <= tolerance.float_tolerance
            }
            (BasicParameterValue::Int(a), BasicParameterValue::Int(b)) => {
                (a - b).abs() <= tolerance.int_tolerance
            }
            (BasicParameterValue::Bool(a), BasicParameterValue::Bool(b)) => a == b,
            _ => false,
        }
    }
}

impl ParameterValue {
    /// 考虑容差的相等性比较
    pub fn equals_with_tolerance(&self, other: &Self, tolerance: &ToleranceConfig) -> bool {
        match (self, other) {
            (ParameterValue::Basic(a), ParameterValue::Basic(b)) => {
                a.equals_with_tolerance(b, tolerance)
            }
            (ParameterValue::List(a), ParameterValue::List(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for (item_a, item_b) in a.iter().zip(b.iter()) {
                    if !item_a.equals_with_tolerance(item_b, tolerance) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parameter_value_display() {
        let string_value = BasicParameterValue::String("test_string".to_string());
        assert_eq!(format!("{}", string_value), "test_string");

        let float_value = BasicParameterValue::Float(3.14159265359);
        assert_eq!(format!("{}", float_value), "3.141593");

        let int_value = BasicParameterValue::Int(42);
        assert_eq!(format!("{}", int_value), "42");

        let bool_value = BasicParameterValue::Bool(true);
        assert_eq!(format!("{}", bool_value), "true");
    }

    #[test]
    fn test_basic_parameter_value_debug() {
        let string_value = BasicParameterValue::String("test_string".to_string());
        assert_eq!(format!("{:?}", string_value), "test_string");

        let float_value = BasicParameterValue::Float(3.14159265359);
        assert_eq!(format!("{:?}", float_value), "3.141593");

        let int_value = BasicParameterValue::Int(42);
        assert_eq!(format!("{:?}", int_value), "42");

        let bool_value = BasicParameterValue::Bool(true);
        assert_eq!(format!("{:?}", bool_value), "true");
    }

    #[test]
    fn test_parameter_value_debug_basic() {
        let basic_string = ParameterValue::Basic(BasicParameterValue::String("hello".to_string()));
        assert_eq!(format!("{:?}", basic_string), "hello");

        let basic_float = ParameterValue::Basic(BasicParameterValue::Float(2.5));
        assert_eq!(format!("{:?}", basic_float), "2.500000");

        let basic_int = ParameterValue::Basic(BasicParameterValue::Int(100));
        assert_eq!(format!("{:?}", basic_int), "100");

        let basic_bool = ParameterValue::Basic(BasicParameterValue::Bool(false));
        assert_eq!(format!("{:?}", basic_bool), "false");
    }

    #[test]
    fn test_parameter_value_debug_list() {
        let list = ParameterValue::List(vec![
            ParameterValue::Basic(BasicParameterValue::Int(1)),
            ParameterValue::Basic(BasicParameterValue::String("two".to_string())),
            ParameterValue::Basic(BasicParameterValue::Float(3.0)),
        ]);
        assert_eq!(format!("{:?}", list), "[1, two, 3.000000]");
    }

    #[test]
    fn test_parameter_value_debug_nested_list() {
        let nested_list = ParameterValue::List(vec![
            ParameterValue::List(vec![
                ParameterValue::Basic(BasicParameterValue::Int(1)),
                ParameterValue::Basic(BasicParameterValue::Int(2)),
            ]),
            ParameterValue::Basic(BasicParameterValue::String("nested".to_string())),
        ]);
        assert_eq!(format!("{:?}", nested_list), "[[1, 2], nested]");
    }

    #[test]
    fn test_debug_equals_display() {
        // 测试 Debug 和 Display 的输出是否相同
        let string_value = BasicParameterValue::String("test".to_string());
        assert_eq!(format!("{:?}", string_value), format!("{}", string_value));

        let float_value = BasicParameterValue::Float(1.234567);
        assert_eq!(format!("{:?}", float_value), format!("{}", float_value));

        let int_value = BasicParameterValue::Int(42);
        assert_eq!(format!("{:?}", int_value), format!("{}", int_value));

        let bool_value = BasicParameterValue::Bool(true);
        assert_eq!(format!("{:?}", bool_value), format!("{}", bool_value));

        // 测试 ParameterValue
        let basic_value = ParameterValue::Basic(BasicParameterValue::String("hello".to_string()));
        assert_eq!(format!("{:?}", basic_value), format!("{}", basic_value));

        let list_value = ParameterValue::List(vec![
            ParameterValue::Basic(BasicParameterValue::Int(1)),
            ParameterValue::Basic(BasicParameterValue::String("two".to_string())),
        ]);
        assert_eq!(format!("{:?}", list_value), format!("{}", list_value));
    }
}
