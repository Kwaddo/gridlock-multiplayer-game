#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        let s = $crate::platform::unix_time_ms() / 1000;
        let now = format!("{:02}:{:02}:{:02}", (s % 86400) / 3600, (s % 3600) / 60, s % 60);
        println!("[{}] {}", now, format_args!($($arg)*));
    }};
}
