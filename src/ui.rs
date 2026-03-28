use crate::{
    config::{TerminalUIConfig, TerminalUIConfigValidator},
    get_term_size,
    renderer::TerminalRenderer,
    sequencer::EscapeSequencer,
};

#[allow(dead_code)]
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
    fn get_term_size() -> (usize, usize) {
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

    pub fn from_config(config_path: &str) -> Result<Self> {
        let config_raw: String = match std::fs::read_to_string(config_path) {
            Ok(s) => s,
            Err(_) => return Err(ConfigMissing),
        };

        let config: TerminalUIConfig = match serde_json::from_str(&config_raw) {
            Ok(c) => c,
            Err(_) => return Err(InvalidJson),
        };

        println!("{config:?}");

        let (term_width, term_height) = get_term_size();
        let mut renderer: TerminalRenderer =
            TerminalRenderer::new(EscapeSequencer::new(term_width, term_height));

        let mut config_validator = TerminalUIConfigValidator::new(term_width, term_height);
        config_validator.validate(&config)?;

        match renderer.with_config(&config) {
            Ok(_) => {} // yey
            Err(e) => match e {},
        };

        Ok(TerminalUI { renderer, config })
    }
}
