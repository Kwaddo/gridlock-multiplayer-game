macro_rules! log {
    ($($arg:tt)*) => {{
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| {
                let s = d.as_secs();
                format!("{:02}:{:02}:{:02}", (s % 86400) / 3600, (s % 3600) / 60, s % 60)
            })
            .unwrap_or_else(|_| "??:??:??".to_string());
        println!("[{}] {}", now, format_args!($($arg)*));
    }};
}
