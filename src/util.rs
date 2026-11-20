use std::time::Duration;

use tokio::time::{Interval, MissedTickBehavior, interval};

/// Redirects stderr to `debug.log` so `eprintln!` traces never bleed into
/// the terminal UI. Safe to call multiple times; subsequent calls are no-ops
/// if the file cannot be opened.
pub(crate) fn redirect_stderr() {
    unsafe {
        let path = b"debug.log\0";
        let fd = libc::open(
            path.as_ptr() as *const libc::c_char,
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o644,
        );
        if fd >= 0 {
            libc::dup2(fd, libc::STDERR_FILENO);
            libc::close(fd);
        }
    }
}

pub(crate) fn interval_from_millis(millis: Option<u64>) -> Option<Interval> {
    millis.map(|millis| {
        let mut refresh = interval(Duration::from_millis(millis));
        refresh.set_missed_tick_behavior(MissedTickBehavior::Delay);
        refresh
    })
}
