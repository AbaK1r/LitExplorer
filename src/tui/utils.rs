use ratatui::style::Color;

/// 从版本数据中提取版本名称
pub fn extract_version_names(versions: &[crate::models::VersionData]) -> Vec<String> {
    versions
        .iter()
        .map(|version| {
            version
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&format!("version_{}", version.version_num))
                .to_string()
        })
        .collect()
}

/// 将颜色字符串转换为ratatui的Color
pub fn parse_color(color_str: &str) -> Color {
    let color_map = get_color_map();
    color_map
        .get(&color_str.to_lowercase())
        .copied()
        .unwrap_or(Color::White)
}

/// 获取颜色映射表
fn get_color_map() -> std::collections::HashMap<String, Color> {
    use std::collections::HashMap;

    let mut map = HashMap::new();

    // 基础颜色
    insert_basic_colors(&mut map);

    // 灰色系
    insert_gray_colors(&mut map);

    // 亮色
    insert_light_colors(&mut map);

    map
}

/// 插入基础颜色
fn insert_basic_colors(map: &mut std::collections::HashMap<String, Color>) {
    map.insert("black".to_string(), Color::Black);
    map.insert("red".to_string(), Color::Red);
    map.insert("green".to_string(), Color::Green);
    map.insert("yellow".to_string(), Color::Yellow);
    map.insert("blue".to_string(), Color::Blue);
    map.insert("magenta".to_string(), Color::Magenta);
    map.insert("cyan".to_string(), Color::Cyan);
    map.insert("white".to_string(), Color::White);
}

/// 插入灰色系颜色
fn insert_gray_colors(map: &mut std::collections::HashMap<String, Color>) {
    map.insert("gray".to_string(), Color::Gray);
    map.insert("grey".to_string(), Color::Gray);
    map.insert("dark_gray".to_string(), Color::DarkGray);
    map.insert("dark_grey".to_string(), Color::DarkGray);
}

/// 插入亮色
fn insert_light_colors(map: &mut std::collections::HashMap<String, Color>) {
    map.insert("light_red".to_string(), Color::LightRed);
    map.insert("light_green".to_string(), Color::LightGreen);
    map.insert("light_yellow".to_string(), Color::LightYellow);
    map.insert("light_blue".to_string(), Color::LightBlue);
    map.insert("light_magenta".to_string(), Color::LightMagenta);
    map.insert("light_cyan".to_string(), Color::LightCyan);
}

/// 计算列表布局参数
pub fn calculate_list_layout(
    max_name_length: usize,
    num_names: usize,
    area_width: u16,
) -> (usize, usize) {
    let cols = calculate_optimal_columns(area_width, max_name_length, num_names);

    let spacing = ((area_width as f64 - (cols * max_name_length) as f64) / (cols - 1) as f64)
        .floor()
        .max(1f64) as usize;

    (cols, spacing)
}

/// 计算最优列数
fn calculate_optimal_columns(
    area_width: u16,
    max_name_length: usize,
    num_names: usize,
) -> usize {
    assert!(num_names > 0, "num_names must be at least 1");

    let mut best_cols = 1;

    for cols in 2..area_width.into() {
        let col_width = area_width as f64 / cols as f64;
        if col_width <= max_name_length as f64 {
            break;
        }
        best_cols = cols;
    }

    best_cols
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppState;
    use crate::models::config::Config;
    use crate::tui::app::App;

    fn create_test_app_with_versions() -> App {
        use crate::models::models::VersionData;
        use std::path::PathBuf;

        let versions = vec![
            VersionData {
                version_num: 0,
                path: PathBuf::from("version_0"),
                hparams: std::collections::HashMap::new(),
            },
            VersionData {
                version_num: 1,
                path: PathBuf::from("version_1"),
                hparams: std::collections::HashMap::new(),
            },
            VersionData {
                version_num: 2,
                path: PathBuf::from("version_2"),
                hparams: std::collections::HashMap::new(),
            },
        ];

        let app_state = AppState {
            all_versions: versions,
            experiment_groups: vec![],
            config: Config::default(),
            group_common_hparams: std::collections::HashMap::new(),
        };
        App::new(app_state)
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("red"), Color::Red);
        assert_eq!(parse_color("green"), Color::Green);
        assert_eq!(parse_color("blue"), Color::Blue);
        assert_eq!(parse_color("white"), Color::White);
        assert_eq!(parse_color("black"), Color::Black);
        assert_eq!(parse_color("cyan"), Color::Cyan);
        assert_eq!(parse_color("yellow"), Color::Yellow);
        assert_eq!(parse_color("invalid"), Color::White); // 默认颜色
        assert_eq!(parse_color("RED"), Color::Red); // 测试大小写不敏感
    }

    #[test]
    fn test_calculate_optimal_columns() {
        // 测试基本情况
        assert_eq!(calculate_optimal_columns(80, 10, 5), 7); // 80宽度，每个名称10字符
        assert_eq!(calculate_optimal_columns(40, 15, 3), 2); // 40宽度，每个名称15字符
        assert_eq!(calculate_optimal_columns(100, 20, 10), 4); // 100宽度，每个名称20字符

        // 测试边界情况
        assert_eq!(calculate_optimal_columns(10, 5, 1), 1);
        assert_eq!(calculate_optimal_columns(50, 25, 2), 1);

        // 测试无法放下多列的情况
        assert_eq!(calculate_optimal_columns(15, 10, 5), 1); // 宽度不够，只能1列
    }

    #[test]
    fn test_calculate_list_layout() {
        // 测试基本情况
        assert_eq!(calculate_list_layout(10, 5, 10), (1, 1)); // 10宽度，每个名称10字符 -> 1列，1个空格
        assert_eq!(calculate_list_layout(10, 5, 80), (7, 1)); // 80宽度，每个名称10字符 -> 7列，1个空格
        assert_eq!(calculate_list_layout(15, 3, 40), (2, 10)); // 40宽度，每个名称15字符 -> 2列，10个空格
        assert_eq!(calculate_list_layout(20, 10, 100), (4, 6)); // 100宽度，每个名称20字符 -> 4列，6个空格
    }

    /// 创建测试用的App实例
    fn create_test_app() -> App {
        let app_state = AppState {
            all_versions: vec![],
            experiment_groups: vec![],
            config: Config::default(),
            group_common_hparams: std::collections::HashMap::new(),
        };
        App::new(app_state)
    }
}
