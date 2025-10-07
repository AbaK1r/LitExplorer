use crate::tui::{
    App, Event, EventHandler, InputHandler, Renderer, UserAction
};
use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;

/// TUI应用控制器，负责协调各个组件
pub struct TuiApp {
    app: App,
    input_handler: InputHandler,
    renderer: Renderer,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TuiApp {
    pub fn new(app: App, keybindings: crate::models::KeybindingsConfig) -> Result<Self> {
        // 设置终端
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let input_handler = InputHandler::new(keybindings);
        let renderer = Renderer::new();

        Ok(Self {
            app,
            input_handler,
            renderer,
            terminal,
        })
    }

    /// 运行TUI应用主循环
    pub fn run(&mut self) -> Result<()> {
        // 创建事件处理器，使用配置中的刷新率
        let tick_rate = Duration::from_millis(self.app.state.config.tui.refresh_rate_ms);
        let events = EventHandler::new(tick_rate);

        // 初始化时更新详情内容缓存
        self.app.smart_update_detail_content_cache();

        loop {
            // 渲染界面
            self.terminal.draw(|f| {
                self.renderer.draw(f, &mut self.app);
            })?;

            // 处理事件
            match events.next()? {
                Event::Input(event) => {
                    let action = self.input_handler.handle_key_event(event);
                    match action {
                        UserAction::Quit => self.app.quit(),
                        _ => self.app.last_user_action = action
                    }
                     
                    // self.handle_user_action(action)?;
                }
                Event::Tick => {
                    // 可以在这里添加定时任务
                }
            }

            if self.app.should_quit {
                break;
            }
        }

        // 在退出前清理终端状态
        self.cleanup()?;
        
        Ok(())
    }

    // fn handle_user_action(&mut self, action: UserAction) -> Result<bool> {
    //     use UserAction::*;

    //     match action {
    //         Quit => {
    //             self.app.quit();
    //             return Ok(true); // 退出应用
    //         }
    //         MoveUp | MoveDown | MoveLeft | MoveRight => {
    //             self.handle_movement_action(action)?;
    //         }
    //         ScrollDetailUp => self.handle_scroll_up_action(),
    //         ScrollDetailDown => self.handle_scroll_down_action(),
    //         Help | Filter | Select | Confirm => {
    //             // TODO: 实现相应功能
    //         }
    //         None => {}
    //     }

    //     Ok(false) // 不退出应用
    // }


    // 清理终端设置
    pub fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}
