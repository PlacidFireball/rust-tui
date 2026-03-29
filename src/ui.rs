use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use crate::{
    config::{TerminalUIConfig, TerminalUIConfigValidator},
    renderer::TerminalRenderer,
    sequencer::EscapeSequencer,
};

static RESIZED: AtomicBool = AtomicBool::new(false);
extern "C" fn handle_sigwinch(_: libc::c_int) {
    RESIZED.store(true, Ordering::Relaxed);
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum TerminalUIError {
    ConfigMissing,
    InvalidJson,
    InvalidConfig { value_ptr: String, message: String },
}
use TerminalUIError::*;

pub(crate) type Result<T> = std::result::Result<T, TerminalUIError>;

#[allow(dead_code)]
pub struct TerminalUI {
    renderer: TerminalRenderer,
    config: TerminalUIConfig,
}
#[allow(dead_code)]
impl TerminalUI {
    pub fn get_term_size() -> (usize, usize) {
        unsafe {
            let mut winsize = libc::winsize {
                ws_row: 0,
                ws_col: 0,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let ret = libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize);

            if ret < 0 {
                eprintln!("[FATAL] Failure to get terminal size!");
                panic!("failure to get terminal size")
            }

            (winsize.ws_col as usize, winsize.ws_row as usize)
        }
    }

    /// Redirects stderr to `debug.log` so `eprintln!` traces never bleed into
    /// the terminal UI. Safe to call multiple times; subsequent calls are no-ops
    /// if the file cannot be opened.
    fn redirect_stderr() {
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

    pub fn from_config(config_path: &str) -> Result<Self> {
        Self::redirect_stderr();
        let config_raw: String = match std::fs::read_to_string(config_path) {
            Ok(s) => s,
            Err(_) => return Err(ConfigMissing),
        };

        let config: TerminalUIConfig = match serde_json::from_str(&config_raw) {
            Ok(c) => c,
            Err(_) => return Err(InvalidJson),
        };

        println!("{config:?}");

        let (term_width, term_height) = Self::get_term_size();
        let mut renderer: TerminalRenderer =
            TerminalRenderer::new(EscapeSequencer::new(term_width, term_height));

        let mut config_validator = TerminalUIConfigValidator::new(term_width, term_height);
        config_validator.validate(&config)?;

        match renderer.with_config(&config) {
            Ok(_) => {} // yey
            Err(e) => match e {},
        };

        renderer.clear_screen();

        Ok(TerminalUI { renderer, config })
    }

    pub fn resize(&mut self) -> Result<()> {
        let (new_term_width, new_term_height) = Self::get_term_size();
        self.renderer.on_resize(new_term_width, new_term_height);
        Ok(())
    }

    pub fn running_loop(&mut self) {
        // Register SIGWINCH handler
        unsafe {
            libc::signal(
                libc::SIGWINCH,
                handle_sigwinch as *const () as libc::sighandler_t,
            );
        }

        loop {
            if RESIZED.swap(false, Ordering::Relaxed) {
                self.resize();
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
