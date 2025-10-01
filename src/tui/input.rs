use crate::models::KeybindingsConfig;
use crossterm::event::{KeyCode, KeyEvent};

/// 输入处理器，负责将按键事件映射到应用操作
pub struct InputHandler {
    keybindings: KeybindingsConfig,
}

/// 用户操作类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UserAction {
    Quit,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    ScrollDetailUp,
    ScrollDetailDown,
    Help,
    Filter,
    Select,
    Confirm,
    None,
}

impl InputHandler {
    pub fn new(keybindings: KeybindingsConfig) -> Self {
        Self { keybindings }
    }

    /// 处理按键事件，返回对应的用户操作
    pub fn handle_key_event(&self, key_event: KeyEvent) -> UserAction {
        match key_event.code {
            KeyCode::Char(c) => self.handle_char_key(c),
            KeyCode::Up => self.get_move_action(&self.keybindings.up),
            KeyCode::Down => self.get_move_action(&self.keybindings.down),
            KeyCode::Left => self.get_move_action(&self.keybindings.left),
            KeyCode::Right => self.get_move_action(&self.keybindings.right),
            KeyCode::Enter => self.get_action(&self.keybindings.confirm, UserAction::Confirm),
            KeyCode::Esc => UserAction::Quit,
            _ => UserAction::None,
        }
    }

    /// 处理字符按键
    fn handle_char_key(&self, c: char) -> UserAction {
        let key_str = c.to_string();

        // 构建操作映射表
        let action_map = self.build_action_map();

        // 查找匹配的操作
        self.find_matching_action(&key_str, &action_map)
    }

    /// 构建操作映射表
    fn build_action_map(&self) -> [(&str, UserAction); 11] {
        [
            (&self.keybindings.quit, UserAction::Quit),
            (&self.keybindings.help, UserAction::Help),
            (&self.keybindings.filter, UserAction::Filter),
            (&self.keybindings.select, UserAction::Select),
            (
                &self.keybindings.scroll_detail_up,
                UserAction::ScrollDetailUp,
            ),
            (
                &self.keybindings.scroll_detail_down,
                UserAction::ScrollDetailDown,
            ),
            (&self.keybindings.up, UserAction::MoveUp),
            (&self.keybindings.down, UserAction::MoveDown),
            (&self.keybindings.left, UserAction::MoveLeft),
            (&self.keybindings.right, UserAction::MoveRight),
            (&self.keybindings.confirm, UserAction::Confirm),
        ]
    }

    /// 查找匹配的操作
    fn find_matching_action(&self, key_str: &str, action_map: &[(&str, UserAction)]) -> UserAction {
        for (key, action) in action_map {
            if key_str == *key {
                return *action;
            }
        }
        UserAction::None
    }

    /// 获取移动操作
    fn get_move_action(&self, configured_key: &str) -> UserAction {
        match configured_key {
            "up" => UserAction::MoveUp,
            "down" => UserAction::MoveDown,
            "left" => UserAction::MoveLeft,
            "right" => UserAction::MoveRight,
            "k" => UserAction::MoveUp,
            "j" => UserAction::MoveDown,
            "h" => UserAction::MoveLeft,
            "l" => UserAction::MoveRight,
            _ => UserAction::None,
        }
    }

    /// 获取指定操作
    fn get_action(&self, configured_key: &str, default_action: UserAction) -> UserAction {
        if configured_key == "enter" {
            default_action
        } else {
            UserAction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::KeybindingsConfig;

    #[test]
    fn test_input_handler() {
        let keybindings = KeybindingsConfig {
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
        };

        let input_handler = InputHandler::new(keybindings);

        // 测试退出键
        let quit_event = KeyEvent::from(KeyCode::Char('q'));
        assert_eq!(input_handler.handle_key_event(quit_event), UserAction::Quit);

        // 测试详情滚动键
        let scroll_up_event = KeyEvent::from(KeyCode::Char('u'));
        assert_eq!(
            input_handler.handle_key_event(scroll_up_event),
            UserAction::ScrollDetailUp
        );

        let scroll_down_event = KeyEvent::from(KeyCode::Char('d'));
        assert_eq!(
            input_handler.handle_key_event(scroll_down_event),
            UserAction::ScrollDetailDown
        );

        // 测试方向键
        let up_event = KeyEvent::from(KeyCode::Up);
        assert_eq!(input_handler.handle_key_event(up_event), UserAction::MoveUp);

        // 测试ESC键
        let esc_event = KeyEvent::from(KeyCode::Esc);
        assert_eq!(input_handler.handle_key_event(esc_event), UserAction::Quit);
    }

    #[test]
    fn test_custom_keybindings() {
        let keybindings = KeybindingsConfig {
            up: "k".to_string(),
            down: "j".to_string(),
            left: "h".to_string(),
            right: "l".to_string(),
            select: "space".to_string(),
            confirm: "enter".to_string(),
            quit: "x".to_string(),
            help: "?".to_string(),
            filter: "f".to_string(),
            switch_view: "t".to_string(),
            scroll_detail_up: "p".to_string(),
            scroll_detail_down: "n".to_string(),
        };

        let input_handler = InputHandler::new(keybindings);

        // 测试自定义退出键
        let quit_event = KeyEvent::from(KeyCode::Char('x'));
        assert_eq!(input_handler.handle_key_event(quit_event), UserAction::Quit);

        // 测试vim风格移动键
        let up_event = KeyEvent::from(KeyCode::Char('k'));
        assert_eq!(input_handler.handle_key_event(up_event), UserAction::MoveUp);

        let down_event = KeyEvent::from(KeyCode::Char('j'));
        assert_eq!(
            input_handler.handle_key_event(down_event),
            UserAction::MoveDown
        );
    }
}
