use crate::tui::utils::{calculate_list_layout, extract_version_names, parse_color};
use crate::tui::{App, UserAction};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// TUI渲染器，负责处理所有UI渲染逻辑
pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    /// 从app结构体中读取数据并渲染
    pub fn draw(&self, f: &mut Frame, app: &mut App) {
        let version_panel_proportion = app.state.config.tui.version_panel_proportion.min(90).max(10);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Percentage(version_panel_proportion),
                Constraint::Percentage(100 - version_panel_proportion)])
            .split(f.area());
        
        self.draw_version_list(f, app, chunks[0]);
        self.draw_version_details(f, app, chunks[1]);
    }

    /// 绘制版本列表
    fn draw_version_list(&self, f: &mut Frame, app: &mut App, area: Rect) {
        let versions = &app.state.all_versions;
        
        // 处理空版本列表情况
        if versions.is_empty() {
            let empty_list = Paragraph::new("No versions found")
                .block(
                    Block::default()
                        .title("Version List")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .alignment(Alignment::Center);
            f.render_widget(empty_list, area);
            return;
        }

        let version_names: Vec<String> = extract_version_names(versions);
        let max_name_length = version_names
            .iter()
            .map(|name| name.len())
            .max()
            .unwrap_or(1);
        let num_names = version_names.len().max(1);
        let (cols, spacing) =
            calculate_list_layout(max_name_length, num_names, area.width.saturating_sub(2));
            
        if app.columns != cols {
            app.columns = cols;
        }

        // 更新好列数后处理用户动作
        let action = app.last_user_action;
        let mut selected_version_index = app.selected_version_index;
        match action {
            UserAction::MoveUp => {
                if selected_version_index != 0 {
                    selected_version_index = selected_version_index.saturating_sub(cols);
                    app.reset_detail_scroll();
                }
                app.last_user_action = UserAction::None;
            },
            UserAction::MoveDown => {
                if selected_version_index != versions.len() - 1 {
                    selected_version_index = selected_version_index.saturating_add(cols).min(versions.len() - 1);
                    app.reset_detail_scroll();
                }
                app.last_user_action = UserAction::None;
            },
            UserAction::MoveLeft => {
                if selected_version_index != 0 {
                    selected_version_index = selected_version_index.saturating_sub(1);
                    app.reset_detail_scroll();
                }
                app.last_user_action = UserAction::None;
            },
            UserAction::MoveRight => {
                if selected_version_index != versions.len() - 1 {
                    selected_version_index = selected_version_index.saturating_add(1).min(versions.len() - 1);
                    app.reset_detail_scroll();
                }
                app.last_user_action = UserAction::None;
            }
            _ => {}
        }
        app.selected_version_index = selected_version_index;

        let (visible_rows, total_rows, scroll_offset) = self.calculate_scroll_info(
            num_names,
            cols,
            area.height,
            selected_version_index,
            app.version_list_scroll_offset,
        );
        app.version_list_scroll_offset = scroll_offset;
        let lines = self.build_version_list_lines(
            &version_names,
            cols,
            visible_rows,
            scroll_offset,
            selected_version_index,
            spacing,
            max_name_length,
        );

        let title = self.generate_list_title(total_rows, visible_rows, scroll_offset);
        let version_list = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left);

        f.render_widget(version_list, area);
    }

    /// 计算滚动信息
    fn calculate_scroll_info(
        &self,
        total_versions: usize,
        cols: usize,
        area_height: u16,
        selected_index: usize,
        current_offset: usize,
    ) -> (usize, usize, usize) {
        let visible_rows = area_height.saturating_sub(2) as usize;
        let total_rows = (total_versions + cols - 1) / cols;
        let selected_row = selected_index / cols;

        let mut scroll_offset = current_offset;
        if selected_row < scroll_offset {
            scroll_offset = selected_row;
        } else if selected_row >= scroll_offset + visible_rows {
            scroll_offset = selected_row.saturating_sub(visible_rows - 1);
        }

        (visible_rows, total_rows, scroll_offset)
    }

    /// 构建版本列表行
    fn build_version_list_lines(
        &self,
        version_names: &[String],
        cols: usize,
        visible_rows: usize,
        scroll_offset: usize,
        selected_index: usize,
        spacing: usize,
        max_name_length: usize,
    ) -> Vec<Line> {
        let mut lines = Vec::new();
        let total_versions = version_names.len();
        let total_rows = (total_versions + cols - 1) / cols;

        for display_row in 0..visible_rows {
            let actual_row = scroll_offset + display_row;
            if actual_row >= total_rows {
                break;
            }

            let mut row_spans = Vec::new();
            for col in 0..cols {
                let index = actual_row * cols + col;
                if index >= total_versions {
                    break;
                }

                let version_name = &version_names[index];
                let style = self.get_version_style(index == selected_index);
                let formatted_name = format!("{:width$}", version_name, width = max_name_length);
                row_spans.push(Span::styled(formatted_name, style));

                if col < cols - 1 && index < total_versions - 1 {
                    row_spans.push(Span::raw(" ".repeat(spacing)));
                }
            }
            lines.push(Line::from(row_spans));
        }

        lines
    }

    /// 获取版本样式
    fn get_version_style(&self, is_selected: bool) -> Style {
        if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        }
    }

    /// 生成列表标题
    fn generate_list_title(
        &self,
        total_rows: usize,
        visible_rows: usize,
        scroll_offset: usize,
    ) -> String {
        if total_rows > visible_rows {
            let percentage =
                self.calculate_scroll_percentage(total_rows, visible_rows, scroll_offset);
            format!("Version List [{}%]", percentage)
        } else {
            "Version List".to_string()
        }
    }

    /// 计算滚动百分比
    fn calculate_scroll_percentage(
        &self,
        total_rows: usize,
        visible_rows: usize,
        scroll_offset: usize,
    ) -> u32 {
        if total_rows > 0 && total_rows > visible_rows {
            ((scroll_offset as f64 / (total_rows - visible_rows.min(total_rows)) as f64) * 100.0)
                as u32
        } else {
            0
        }
    }

    /// 绘制版本详情面板
    fn draw_version_details(&self, f: &mut Frame, app: &mut App, area: Rect) {
        app.smart_update_detail_content_cache();
        let content = self.get_detail_content(app);

        let action = app.last_user_action;
        let mut detail_scroll_offset = app.detail_scroll_offset;
        match action {
            UserAction::ScrollDetailUp => {
                detail_scroll_offset = detail_scroll_offset.saturating_sub(1);
                app.last_user_action = UserAction::None;
            },
            UserAction::ScrollDetailDown => {
                detail_scroll_offset = detail_scroll_offset.saturating_add(1).min(content.len() - area.height as usize);
                app.last_user_action = UserAction::None;
            },
            _ => {}
        }
        app.detail_scroll_offset = detail_scroll_offset;
        let scroll_percentage = (detail_scroll_offset as f32 / (content.len() - area.height as usize) as f32 * 100.0) as usize;

        let title = self.generate_detail_title(app, scroll_percentage);

        let details = Paragraph::new(content.join("\n"))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .scroll((app.detail_scroll_offset as u16, 0))
            .wrap(Wrap { trim: true });

        f.render_widget(details, area);
    }

    /// 获取详情内容
    fn get_detail_content(&self, app: &App) -> Vec<String> {
        if let Some(_version) = app.get_current_version() {
            app.get_detail_content_cached()
                .map(|cached| cached.iter().map(|line| line.to_string()).collect())
                .unwrap_or_else(|| vec!["Loading...".to_string()])
        } else {
            vec!["No version selected".to_string()]
        }
    }

    /// 生成详情面板标题
    fn generate_detail_title(&self, app: &App, scroll_percentage: usize) -> String {
        if let Some(version) = app.get_current_version() {
            self.extract_version_name(version)
                .map(|name| format!("Details - {} [{}%]", name, scroll_percentage))
                .unwrap_or_else(|| "Details - Unknown".to_string())
        } else {
            "Details".to_string()
        }
    }

    /// 提取版本名称
    fn extract_version_name(&self, version: &crate::models::models::VersionData) -> Option<String> {
        version
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
    }
}
