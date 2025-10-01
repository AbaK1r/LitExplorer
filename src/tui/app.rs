use crate::models::AppState;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use crate::tui::input::UserAction;

/// TUI应用主结构
pub struct App {
    pub state: AppState,
    pub columns: usize,
    pub selected_version_index: usize, // 当前选中的版本索引
    pub view_mode: ViewMode,           // 视图模式：版本列表或实验组
    pub last_user_action: UserAction, // 上次用户操作
    pub should_quit: bool,
    pub version_list_scroll_offset: usize, // 版本列表滚动偏移
    pub detail_content_cache: Vec<Line<'static>>, // 详情面板内容缓存
    pub detail_content_version: Option<u32>, // 缓存对应的版本号，用于判断是否需要更新
    pub detail_scroll_offset: usize,       // 详情面板滚动偏移（用于渲染器）
}

/// 视图模式 - 已简化，只支持版本列表模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    VersionList, // 版本列表模式（类似ls命令）
}

impl App {
    pub fn new(state: AppState) -> Self {
        let mut app = Self {
            state,
            columns: 1,
            selected_version_index: 0,
            view_mode: ViewMode::VersionList, // 默认使用版本列表模式
            last_user_action: UserAction::None, // 初始时无上次操作
            should_quit: false,
            version_list_scroll_offset: 0,
            detail_content_cache: Vec::new(),
            detail_content_version: None,
            detail_scroll_offset: 0, // 详情面板滚动偏移初始化为0
        };
        // 初始化详情面板内容
        app.update_detail_content_cache();
        app
    }

    /// 处理退出操作
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// 重置详情面板滚动偏移
    pub fn reset_detail_scroll(&mut self) {
        self.detail_scroll_offset = 0; // 重置渲染器使用的滚动偏移
    }

    /// 获取当前选中的版本
    pub fn get_selected_version(&self) -> Option<&crate::models::VersionData> {
        self.state.all_versions.get(self.selected_version_index)
    }

    /// 获取当前版本（兼容renderer.rs中的调用）
    pub fn get_current_version(&self) -> Option<&crate::models::VersionData> {
        self.get_selected_version()
    }

    /// 获取当前选中版本所属的实验组
    pub fn get_selected_version_group(&self) -> Option<usize> {
        if let Some(version) = self.get_selected_version() {
            for (group_idx, group) in self.state.experiment_groups.iter().enumerate() {
                if group
                    .member_versions
                    .iter()
                    .any(|v| v.version_num == version.version_num)
                {
                    return Some(group_idx);
                }
            }
        }
        None
    }

    /// 获取当前选中版本的main_key参数
    pub fn get_selected_version_main_key_params(
        &self,
    ) -> Option<&std::collections::HashMap<String, crate::models::parameter_value::ParameterValue>>
    {
        if let Some(version) = self.get_selected_version() {
            // 构建分组键，格式为 "main_key1=value1, main_key2=value2"
            if let Some(main_keys) = &self.state.config.grouping.main_key {
                let mut group_key_parts = Vec::new();
                for main_key in main_keys {
                    if let Some(main_key_value) = version.hparams.get(main_key) {
                        group_key_parts.push(format!("{}={}", main_key, main_key_value));
                    }
                }

                // 如果所有main_key都存在，则创建分组键并查找
                if group_key_parts.len() == main_keys.len() {
                    let group_key = group_key_parts.join(", ");
                    return self.state.group_common_hparams.get(&group_key);
                }
            }
        }
        None
    }

    /// 更新详情面板内容缓存
    pub fn update_detail_content_cache(&mut self) {
        let mut all_content_lines = Vec::new();

        // 先获取版本信息，避免借用冲突
        let version_info = self
            .get_selected_version()
            .map(|v| (v.version_num, v.clone()));

        if let Some((version_num, version)) = version_info {
            self.build_version_content(&mut all_content_lines, &version);
            self.build_experiment_group_content(&mut all_content_lines, &version);
            self.build_main_key_content(&mut all_content_lines, &version);
            self.detail_content_version = Some(version_num);
        } else {
            all_content_lines.push(Line::from("No version selected"));
            self.detail_content_version = None;
        }

        self.detail_content_cache = all_content_lines;
        self.reset_detail_scroll();
    }

    fn build_version_content(
        &mut self,
        lines: &mut Vec<Line<'static>>,
        _version: &crate::models::models::VersionData,
    ) {
        lines.push(Line::from(vec![
            Span::styled(
                "Version: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                _version.version_num.to_string(),
                Style::default().fg(Color::Green),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Hyperparameters:",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]));

        for (key, value) in &_version.hparams {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", key),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(value.to_simple_string(), Style::default().fg(Color::Green)),
            ]));
        }
    }

    /// 构建实验组内容
    fn build_experiment_group_content(
        &self,
        lines: &mut Vec<Line<'static>>,
        _version: &crate::models::models::VersionData,
    ) {
        if let Some(group_idx) = self.get_selected_version_group() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                format!("Experiment Group {}:", group_idx + 1),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )]));

            let group = &self.state.experiment_groups[group_idx];
            for (key, value) in &group.base_parameters {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {}: ", key),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(value.to_simple_string(), Style::default().fg(Color::Green)),
                ]));
            }
        }
    }

    /// 构建main_key内容
    fn build_main_key_content(
        &self,
        lines: &mut Vec<Line<'static>>,
        _version: &crate::models::models::VersionData,
    ) {
        if let Some(main_key_params) = self.get_selected_version_main_key_params() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Main Key Groups:",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )]));

            for (key, value) in main_key_params {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {}: ", key),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(value.to_simple_string(), Style::default().fg(Color::Green)),
                ]));
            }
        }
    }

    /// 智能更新详情内容缓存
    /// 只在需要时（版本发生变化时）才重新生成缓存
    ///
    /// # 返回
    /// * `bool` - 如果缓存被更新返回true，否则返回false
    pub fn smart_update_detail_content_cache(&mut self) -> bool {
        let current_version = self.get_selected_version().map(|v| v.version_num);
        let cached_version = self.detail_content_version;

        // 只在版本发生变化时才更新缓存
        if current_version != cached_version.into() {
            self.update_detail_content_cache();
            true
        } else {
            false
        }
    }

    /// 获取缓存的详情内容（兼容renderer.rs中的调用）
    pub fn get_detail_content_cached(&self) -> Option<&Vec<Line<'static>>> {
        if self.detail_content_cache.is_empty() {
            None
        } else {
            Some(&self.detail_content_cache)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::Config;
    use crate::models::models::{ExperimentGroup, VersionData};
    use crate::models::parameter_value::{BasicParameterValue, ParameterValue};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_test_app_state() -> AppState {
        let config = Config::default();

        // 创建测试版本数据
        let mut hparams1 = HashMap::new();
        hparams1.insert(
            "learning_rate".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.01)),
        );
        hparams1.insert(
            "batch_size".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(32)),
        );

        let mut hparams2 = HashMap::new();
        hparams2.insert(
            "learning_rate".to_string(),
            ParameterValue::Basic(BasicParameterValue::Float(0.001)),
        );
        hparams2.insert(
            "batch_size".to_string(),
            ParameterValue::Basic(BasicParameterValue::Int(64)),
        );

        let version1 = VersionData {
            version_num: 1,
            path: PathBuf::from("logs/version_1"),
            hparams: hparams1,
        };

        let version2 = VersionData {
            version_num: 2,
            path: PathBuf::from("logs/version_2"),
            hparams: hparams2,
        };

        let all_versions = vec![version1.clone(), version2.clone()];

        // 创建测试实验组
        let mut base_params = HashMap::new();
        base_params.insert(
            "model_type".to_string(),
            ParameterValue::Basic(BasicParameterValue::String("CNN".to_string())),
        );

        let group1 = ExperimentGroup {
            group_id: "group_1".to_string(),
            base_parameters: base_params,
            member_versions: vec![version1],
        };

        let group2 = ExperimentGroup {
            group_id: "group_2".to_string(),
            base_parameters: HashMap::new(),
            member_versions: vec![version2],
        };

        let experiment_groups = vec![group1, group2];
        let group_common_hparams = HashMap::new();

        AppState {
            all_versions,
            experiment_groups,
            config,
            group_common_hparams,
        }
    }

    #[test]
    fn test_app_quit() {
        let state = create_test_app_state();
        let mut app = App::new(state);

        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn test_view_mode_simplified() {
        let state = create_test_app_state();
        let app = App::new(state);

        // 默认应该是版本列表模式，且只支持这一种模式
        assert_eq!(app.view_mode, ViewMode::VersionList);
        // 视图切换功能已移除，不再测试模式切换
    }

    #[test]
    fn test_get_selected_version() {
        let state = create_test_app_state();
        let app = App::new(state);

        let selected_version = app.get_selected_version();
        assert!(selected_version.is_some());
        assert_eq!(selected_version.unwrap().version_num, 1);
    }

    #[test]
    fn test_get_selected_version_group() {
        let state = create_test_app_state();
        let mut app = App::new(state);

        // 选中第一个版本，它应该在第一个实验组中
        app.selected_version_index = 0;
        let group_idx = app.get_selected_version_group();
        assert_eq!(group_idx, Some(0));

        // 选中第二个版本，它应该在第二个实验组中
        app.selected_version_index = 1;
        let group_idx = app.get_selected_version_group();
        assert_eq!(group_idx, Some(1));
    }
}
