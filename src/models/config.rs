use serde::{Deserialize, Deserializer};

/// 应用程序配置结构
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    // ————————————————————————————————————————————————————————————————————————
    // 通用配置，包含日志目录、超参数文件等基本设置
    // ————————————————————————————————————————————————————————————————————————
    pub general: GeneralConfig,
    // ————————————————————————————————————————————————————————————————————————
    // 忽略参数配置，定义在比较和分组时需要忽略的参数
    // ————————————————————————————————————————————————————————————————————————
    pub ignored_parameters: IgnoredConfig,
    // ————————————————————————————————————————————————————————————————————————
    // 容差配置，定义不同类型参数的容差范围
    // ————————————————————————————————————————————————————————————————————————
    pub tolerance: ToleranceConfig,
    // ————————————————————————————————————————————————————————————————————————
    // 分组配置，定义实验分组的行为和参数
    // ————————————————————————————————————————————————————————————————————————
    pub grouping: GroupingConfig,
    // ————————————————————————————————————————————————————————————————————————
    // 差异比较配置，控制差异显示的格式和行为
    // ————————————————————————————————————————————————————————————————————————
    pub diff: DiffConfig,
    // ————————————————————————————————————————————————————————————————————————
    // TUI界面配置，定义终端用户界面的外观和行为
    // ————————————————————————————————————————————————————————————————————————
    pub tui: TuiConfig,
    // ————————————————————————————————————————————————————————————————————————
    // 键盘绑定配置，定义用户界面的快捷键
    // ————————————————————————————————————————————————————————————————————————
    pub keybindings: KeybindingsConfig,
    // ————————————————————————————————————————————————————————————————————————
    // 测试脚本配置，定义测试脚本的执行参数
    // ————————————————————————————————————————————————————————————————————————
    pub test_script: TestScriptConfig,
}

/// 通用配置
#[derive(Debug, Deserialize, Default)]
pub struct GeneralConfig {
    // ————————————————————————————————————————————————————————————————————————
    // 日志文件存储目录路径
    // ————————————————————————————————————————————————————————————————————————
    pub log_dir: String,
    // ————————————————————————————————————————————————————————————————————————
    // 超参数文件名，用于存储实验参数配置
    // ————————————————————————————————————————————————————————————————————————
    pub hparams_file: String,
    // ————————————————————————————————————————————————————————————————————————
    // 是否启用缓存功能，提高程序运行效率
    // ————————————————————————————————————————————————————————————————————————
    pub cache_enabled: bool,
}

/// 忽略参数配置
#[derive(Debug, Deserialize, Default)]
pub struct IgnoredConfig {
    // ————————————————————————————————————————————————————————————————————————
    // 需要忽略的参数名称列表，这些参数在比较和分组时将被排除
    // ————————————————————————————————————————————————————————————————————————
    pub parameters: Vec<String>,
}

/// 容差配置
#[derive(Debug, Deserialize, Default)]
pub struct ToleranceConfig {
    pub float_tolerance: f64, // 浮点数比较的容差范围，两个浮点数差值小于此值时视为相等
    pub int_tolerance: i64,   // 整数比较的容差范围，两个整数差值小于此值时视为相等
    pub string_case_sensitive: bool, // 字符串比较时是否区分大小写，true为区分大小写，false为不区分
}

/// 分组配置
#[derive(Debug, Deserialize, Default)]
pub struct GroupingConfig {
    // ————————————————————————————————————————————————————————————————————————
    // 是否使用所有参数进行分组，true时使用所有参数，false时只使用指定参数
    // ————————————————————————————————————————————————————————————————————————
    pub group_by_all_parameters: bool, // 是否使用所有参数进行分组，true时使用所有参数，false时只使用指定参数
    // ————————————————————————————————————————————————————————————————————————
    // 分组参数列表，当group_by_all_parameters为false时使用这些参数进行分组
    // ————————————————————————————————————————————————————————————————————————
    pub grouping_parameters: Option<Vec<String>>,
    pub similarity_threshold: usize, // 相似度阈值，用于判断实验是否属于同一组
    #[serde(default)]
    pub main_key: Option<Vec<String>>, // 主键参数列表，用于定义实验的主要标识参数
}

/// 差异比较配置
#[derive(Debug, Deserialize, Default)]
pub struct DiffConfig {
    pub show_detailed_diff: bool, // 是否显示详细的差异信息，true时显示所有差异，false时只显示关键差异
    pub diff_format: String,      // 差异显示格式，定义差异信息的展示方式
    pub highlight_diff_keys: bool, // 是否高亮显示差异键名，true时突出显示有差异的参数名
}

/// TUI界面配置
#[derive(Debug, Deserialize)]
pub struct TuiConfig {
    pub color_theme: String,      // 颜色主题名称，定义界面的整体配色方案
    pub colors: ColorConfig,      // 颜色配置，定义各种界面元素的具体颜色
    pub layout: String,           // 界面布局方式，定义界面的整体排列结构
    pub show_help_bar: bool,      // 是否显示帮助栏，true时在界面底部显示操作提示
    pub auto_expand_groups: bool, // 是否自动展开实验组，true时默认展开所有分组
    pub detail_panel_position: DetailPanelPosition, // 详细信息面板位置配置
    pub refresh_rate_ms: u64,     // TUI刷新率（毫秒），控制界面更新频率
    pub version_panel_proportion: u16, // 版本面板占比（%），控制版本列表和详情面板的高度比例
    pub status_bar_height: u16,   // 状态栏高度（行数）
    pub scroll_indicators: bool,  // 是否显示滚动指示器
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            color_theme: "default".to_string(),
            colors: ColorConfig::default(),
            layout: "vertical".to_string(),
            show_help_bar: true,
            auto_expand_groups: false,
            detail_panel_position: DetailPanelPosition::default(),
            refresh_rate_ms: 250,    // 默认刷新率250ms
            version_panel_proportion: 70,  // 版本面板默认高度 70 %
            status_bar_height: 3,    // 状态栏默认高度3行
            scroll_indicators: true, // 默认显示滚动指示器
        }
    }
}

/// 颜色配置
#[derive(Debug, Deserialize)]
pub struct ColorConfig {
    pub same_experiment: String, // 相同实验的颜色标识，用于标记完全相同的实验
    pub similar_experiment: String, // 相似实验的颜色标识，用于标记相似的实验
    pub selected: String,        // 选中状态的颜色标识，用于标记当前选中的项目
    pub background: String,      // 背景颜色，定义界面的背景色
    pub text: String,            // 文本颜色，定义界面文字的颜色
    pub border: String,          // 边框颜色，定义界面边框的颜色
    pub highlight: String,       // 高亮颜色，用于突出显示重要信息
    pub status_bar_bg: String,   // 状态栏背景色
    pub status_bar_text: String, // 状态栏文本色
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            same_experiment: "green".to_string(),
            similar_experiment: "yellow".to_string(),
            selected: "blue".to_string(),
            background: "black".to_string(),
            text: "white".to_string(),
            border: "cyan".to_string(),
            highlight: "white".to_string(),
            status_bar_bg: "dark_gray".to_string(),
            status_bar_text: "white".to_string(),
        }
    }
}

/// 详细信息面板位置配置
#[derive(Debug, Deserialize)]
pub enum DetailPanelPosition {
    Top,
    Bottom,
    Left,
    Right,
}

impl Default for DetailPanelPosition {
    fn default() -> Self {
        DetailPanelPosition::Bottom
    }
}

/// 键盘绑定配置
#[derive(Debug, Deserialize, Clone)]
pub struct KeybindingsConfig {
    pub up: String,                 // 向上移动键，用于在列表中向上选择
    pub down: String,               // 向下移动键，用于在列表中向下选择
    pub left: String,               // 向左移动键，用于在层级结构中向左导航
    pub right: String,              // 向右移动键，用于在层级结构中向右导航
    pub select: String,             // 选择键，用于选中当前项目
    pub confirm: String,            // 确认键，用于确认当前操作
    pub quit: String,               // 退出键，用于退出程序或返回上级
    pub help: String,               // 帮助键，用于显示帮助信息
    pub filter: String,             // 过滤键，用于激活过滤功能
    pub switch_view: String,        // 切换视图键，用于在版本列表和实验组视图间切换
    pub scroll_detail_up: String,   // 详情向上滚动键
    pub scroll_detail_down: String, // 详情向下滚动键
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            up: "up".to_string(),
            down: "down".to_string(),
            left: "left".to_string(),
            right: "right".to_string(),
            select: "space".to_string(),
            confirm: "enter".to_string(),
            quit: "q".to_string(),
            help: "h".to_string(),
            filter: "/".to_string(),
            switch_view: "v".to_string(),
            scroll_detail_up: "u".to_string(),
            scroll_detail_down: "d".to_string(),
        }
    }
}

/// 测试脚本配置
#[derive(Debug, Deserialize, Default)]
pub struct TestScriptConfig {
    pub path: String,                    // 测试脚本文件路径，指定要执行的测试脚本位置
    pub default_args: DefaultArgsConfig, // 默认参数配置，定义脚本的默认执行参数
    pub prompt_for_args: bool,           // 是否提示输入参数，true时运行前会要求用户输入参数
    pub fixed_args: Vec<String>,         // 固定参数列表，这些参数会在每次运行时自动添加
}

/// 默认参数配置
#[derive(Debug, Deserialize, Default)]
pub struct DefaultArgsConfig {
    // ————————————————————————————————————————————————————————————————————————
    // 默认过滤条件，用于筛选实验数据
    // ————————————————————————————————————————————————————————————————————————
    #[serde(
        default,
        deserialize_with = "crate::models::utils::deserialize_optional_string"
    )]
    pub filter: Option<String>, // 默认过滤条件，用于筛选实验数据
    #[serde(
        default,
        deserialize_with = "crate::models::utils::deserialize_optional_string"
    )]
    pub sort_key: Option<String>, // 默认排序键，用于对实验结果进行排序
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml;

    #[test]
    fn test_config_deserialization() {
        let yaml = r#"
general:
  log_dir: logs
  hparams_file: hparams.yaml
  cache_enabled: true
ignored_parameters:
  parameters: [param1, param2]
tolerance:
  float_tolerance: 0.001
  int_tolerance: 0
  string_case_sensitive: true
grouping:
  group_by_all_parameters: false
  grouping_parameters: [param3, param4]
  similarity_threshold: 80
  main_key: null
diff:
  show_detailed_diff: true
  diff_format: unified
  highlight_diff_keys: true
tui:
  color_theme: default
  colors:
    same_experiment: green
    similar_experiment: yellow
    selected: blue
    background: black
    text: white
    border: cyan
    highlight: white
    status_bar_bg: dark_gray
    status_bar_text: white
  layout: compact
  show_help_bar: true
  auto_expand_groups: false
  detail_panel_position: Bottom
  refresh_rate_ms: 250
  version_panel_proportion: 70
  status_bar_height: 3
  scroll_indicators: true
keybindings:
  up: k
  down: j
  left: h
  right: l
  select: space
  confirm: enter
  quit: q
  help: "?"
  filter: "/"
  switch_view: v
  scroll_detail_up: u
  scroll_detail_down: d
test_script:
  path: test.sh
  default_args:
    filter: ""
    sort_key: ""
  prompt_for_args: true
  fixed_args: [--verbose]
"#;

        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize config");
        dbg!(&config);
        // 验证基本字段
        assert_eq!(config.general.log_dir, "logs");
        assert_eq!(config.general.cache_enabled, true);
        assert_eq!(config.tolerance.float_tolerance, 0.001);
    }
}
