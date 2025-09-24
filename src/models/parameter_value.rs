// use std::fmt;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use crate::models::config::ToleranceConfig;

/// 参数值类型枚举，支持递归结构
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    Basic(BasicParameterValue),
    List(Vec<ParameterValue>),
}

/// 基本参数值类型，用于List中，只包含基本类型
#[derive(Debug, Clone, PartialEq)]
pub enum BasicParameterValue {
    String(String),
    Float(f64),
    Int(i64),
    Bool(bool),
}

/// 将BasicParameterValue转换为字符串表示
impl BasicParameterValue {
    pub fn to_string_repr(&self) -> String {
        match self {
            BasicParameterValue::String(s) => s.clone(),
            BasicParameterValue::Float(n) => format!("{:.6}", n),
            BasicParameterValue::Int(n) => n.to_string(),
            BasicParameterValue::Bool(b) => b.to_string()
        }
    }
}

impl ParameterValue {
    pub fn to_simple_string(&self) -> String {
        match self {
            ParameterValue::Basic(basic_value) => basic_value.to_string_repr(),
            ParameterValue::List(list) => {
                let items: Vec<String> = list.iter().map(|item| item.to_simple_string()).collect();
                format!("[{}]", items.join(", "))
            },
        }
    }
}

impl From<&BasicParameterValue> for JsonValue {
    fn from(val: &BasicParameterValue) -> Self {
        match val {
            BasicParameterValue::String(s) => JsonValue::String(s.clone()),
            BasicParameterValue::Float(f) => {
                serde_json::Number::from_f64(*f)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null)
            },
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

pub fn print_hparams_pretty(hparams: &HashMap<String, ParameterValue>) -> Result<(), serde_json::Error> {
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
            },
            (BasicParameterValue::Float(a), BasicParameterValue::Float(b)) => {
                // 处理NaN: 如果任一值是NaN，则只有两个都是NaN时才相等
                if a.is_nan() || b.is_nan() {
                    return a.is_nan() && b.is_nan();
                }
                (a - b).abs() <= tolerance.float_tolerance
            },
            (BasicParameterValue::Int(a), BasicParameterValue::Int(b)) => {
                (a - b).abs() <= tolerance.int_tolerance
            },
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
            },
            (ParameterValue::List(a), ParameterValue::List(b)) => {
                if a.len() != b.len() { return false; }
                for (item_a, item_b) in a.iter().zip(b.iter()) {
                    if !item_a.equals_with_tolerance(item_b, tolerance) {
                        return false;
                    }
                }
                true
            },
            _ => false,
        }
    }
}
