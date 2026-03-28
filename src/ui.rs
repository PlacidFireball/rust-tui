use serde::{Deserialize, Serialize};

use crate::{get_term_size, renderer::TerminalRenderer, sequencer::EscapeSequencer};

enum TerminalUIError {
    ConfigMissing,
    InvalidJson,
}

type Result<T> = std::result::Result<T, TerminalUIError>;

#[allow(dead_code)]
struct TerminalUI {
    renderer: TerminalRenderer,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct BorderSettings {
    preset: Option<BorderPreset>,
    custom: Option<BorderCharacters>,
}

impl Default for BorderSettings {
    fn default() -> Self {
        Self {
            preset: Some(BorderPreset::ascii),
            custom: None,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
enum BorderPreset {
    rounded,
    ascii,
}

#[allow(dead_code)]
impl BorderPreset {
    pub fn to_characters(&self) -> BorderCharacters {
        match self {
            Self::rounded => BorderCharacters::rounded(),
            Self::ascii => BorderCharacters::ascii(),
        }
    }
}

/// The six characters used to draw a border around a surface.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BorderCharacters {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub left: char,
    pub right: char,
    pub horizontal: char,
}

impl BorderCharacters {
    pub fn rounded() -> Self {
        BorderCharacters {
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
            left: '│',
            right: '│',
            horizontal: '─',
        }
    }

    pub fn ascii() -> Self {
        BorderCharacters {
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
            left: '|',
            right: '|',
            horizontal: '-',
        }
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Frame {
    pub width: usize,
    pub height: usize,
    pub x: usize,
    pub y: usize,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TerminalUIPaneConfig {
    name: String,
    frame: Frame,
    border: Option<BorderSettings>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TerminalUIConfig {
    name: String,
    panes: Vec<TerminalUIPaneConfig>,
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
            Err(_) => return Err(TerminalUIError::ConfigMissing),
        };

        let config: TerminalUIConfig = match serde_json::from_str(&config_raw) {
            Ok(c) => c,
            Err(_) => return Err(TerminalUIError::InvalidJson),
        };

        println!("{config:?}");

        let (term_width, term_height) = get_term_size();
        let mut renderer: TerminalRenderer =
            TerminalRenderer::new(EscapeSequencer::new(term_width, term_height));

        for pane in config.panes {
            unimplemented!()
        }

        Ok(TerminalUI { renderer })
    }
}
