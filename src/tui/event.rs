use crossterm::event::{self, Event as CEvent, KeyEvent};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub enum Event {
    Input(KeyEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    _tx: mpsc::Sender<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let _tx = tx.clone();

        thread::spawn(move || {
            Self::event_loop(tx, tick_rate);
        });

        EventHandler { rx, _tx }
    }

    /// 事件循环处理函数
    fn event_loop(tx: mpsc::Sender<Event>, tick_rate: Duration) {
        let mut last_tick = Instant::now();
        let mut last_key_time = Instant::now();

        loop {
            let timeout = Self::calculate_timeout(tick_rate, last_tick);

            // 处理按键事件
            Self::process_key_events(&tx, timeout, &mut last_key_time);

            // 处理定时器事件
            if Self::should_process_tick(last_tick, tick_rate) {
                Self::send_tick_event(&tx, &mut last_tick);
            }
        }
    }

    /// 处理按键事件
    fn process_key_events(
        tx: &mpsc::Sender<Event>,
        timeout: Duration,
        last_key_time: &mut Instant,
    ) {
        if Self::poll_event(timeout) {
            Self::handle_key_event(tx, last_key_time);
        }
    }

    /// 处理定时器事件
    fn should_process_tick(last_tick: Instant, tick_rate: Duration) -> bool {
        Self::should_send_tick(last_tick, tick_rate)
    }

    /// 发送定时器事件
    fn send_tick_event(tx: &mpsc::Sender<Event>, last_tick: &mut Instant) {
        if tx.send(Event::Tick).is_err() {
            return; // 如果发送失败，直接返回，循环会在下次迭代中退出
        }
        *last_tick = Instant::now();
    }

    /// 计算超时时间
    fn calculate_timeout(tick_rate: Duration, last_tick: Instant) -> Duration {
        tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0))
    }

    /// 轮询事件
    fn poll_event(timeout: Duration) -> bool {
        event::poll(timeout).expect("poll works")
    }

    /// 处理按键事件
    fn handle_key_event(tx: &mpsc::Sender<Event>, last_key_time: &mut Instant) {
        if let CEvent::Key(key) = event::read().expect("can read events") {
            // 添加按键防抖，防止一次按键多次触发
            if last_key_time.elapsed() > Duration::from_millis(150) {
                tx.send(Event::Input(key)).expect("can send events");
                *last_key_time = Instant::now();
            }
        }
    }

    /// 判断是否应该发送Tick事件
    fn should_send_tick(last_tick: Instant, tick_rate: Duration) -> bool {
        last_tick.elapsed() >= tick_rate
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
