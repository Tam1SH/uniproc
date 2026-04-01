#[macro_export]
macro_rules! ratelimit {
    ($timeout:expr, $tracing_macro:ident ! ($($arg:tt)+)) => {
        {
            static LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let last = LAST.load(std::sync::atomic::Ordering::Relaxed);

            if now >= last + $timeout {
                if LAST.compare_exchange(last, now, std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed).is_ok() {
                    let gap = if last == 0 { 0 } else { now - last };

                    if last > 0 && gap > $timeout {
                        tracing::error!(gap_s = gap, timeout_s = $timeout, "Silence timeout exceeded");
                    }

                    tracing::$tracing_macro!(timeout_s = $timeout, gap_s = gap, $($arg)+);
                }
            }
        }
    };
}
