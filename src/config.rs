use std::collections::HashSet;

use crate::ui::Result;
use crate::ui::TerminalUIError::*;

use pub_fields_macro::pub_fields;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[pub_fields]
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BorderSettings {
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
#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub enum BorderPreset {
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
#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct BorderCharacters {
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
    left: char,
    right: char,
    horizontal: char,
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

#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Frame {
    width: Value,
    height: Value,
    x: Value,
    y: Value,
}

#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PythonSettings {
    code: String,
}

#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DynamicText {
    refresh_millis: Option<u64>,
    python: Option<PythonSettings>,
    // more ...
}

#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TerminalUIPaneConfig {
    name: String,
    frame: Frame,
    border: Option<BorderSettings>,
    initial_text: Option<String>,
    dynamic_text: Option<DynamicText>,
}

#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TerminalUIConfig {
    name: String,
    panes: Vec<TerminalUIPaneConfig>,
}

pub struct TerminalUIConfigValidator {
    seen_pane_names: HashSet<String>,
    term_max_width: usize,
    term_max_height: usize,
}
impl TerminalUIConfigValidator {
    pub fn new(term_width: usize, term_height: usize) -> Self {
        TerminalUIConfigValidator {
            seen_pane_names: HashSet::new(),
            term_max_width: term_width,
            term_max_height: term_height,
        }
    }

    pub fn validate_pane_frame(&mut self, idx: usize, pane: &TerminalUIPaneConfig) -> Result<()> {
        fn validate_frame_value(
            validator: &mut TerminalUIConfigValidator,
            idx: usize,
            name: &str,
            value: &Value,
        ) -> Result<()> {
            let value_ptr = format!("/config/panes/{idx}/frame/{name}");

            match value {
                Value::Number(number) => {
                    match number.as_u64() {
                        Some(n) => {
                            let value = n as usize;

                            let min: usize = 0;
                            let max: usize = if name == "x" || name == "width" {
                                validator.term_max_width
                            } else {
                                validator.term_max_height
                            };

                            if value < min || value > max {
                                return Err(InvalidConfig {
                                    value_ptr,
                                    message: format!(
                                        "absolute value must be {min} <= {name} <= {max}"
                                    ),
                                });
                            }
                        }
                        None => {
                            return Err(InvalidConfig {
                                value_ptr,
                                message: format!("must be an integer"),
                            });
                        }
                    };
                }
                Value::String(maybe_pct) => {
                    let value = maybe_pct.replace("%", "").parse::<usize>();
                    if !maybe_pct.ends_with("%") {
                        return Err(InvalidConfig {
                            value_ptr,
                            message: format!("relative values must end with % e.g. 50%"),
                        });
                    } else if value.is_err() {
                        return Err(InvalidConfig {
                            value_ptr,
                            message: format!(
                                "relative values must be numeric (unsigned ints < 100) percentages e.g. 50%"
                            ),
                        });
                    } else {
                        let value = value.unwrap();
                        if value > 100 {
                            return Err(InvalidConfig {
                                value_ptr,
                                message: format!(
                                    "relative values must be relative percentages between 0 and 100 e.g. 50%"
                                ),
                            });
                        }
                    }
                }
                _ => {
                    return Err(InvalidConfig {
                        value_ptr,
                        message: format!("must be either string or number"),
                    });
                }
            }

            Ok(())
        }

        validate_frame_value(self, idx, "width", &pane.frame.width)?;
        validate_frame_value(self, idx, "height", &pane.frame.height)?;
        validate_frame_value(self, idx, "x", &pane.frame.x)?;
        validate_frame_value(self, idx, "y", &pane.frame.y)?;

        Ok(())
    }

    fn validate_pane_border_settings(
        &mut self,
        idx: usize,
        pane: &TerminalUIPaneConfig,
    ) -> Result<()> {
        match &pane.border {
            Some(border_settings) => {
                match (border_settings.preset, border_settings.custom) {
                    (Some(_), Some(_)) | (None, None) => {
                        return Err(InvalidConfig {
                            value_ptr: format!("/config/panes/{idx}/border"),
                            message: format!(
                                "define border.preset OR border.custom - config is ambigious as is"
                            ),
                        });
                    }
                    _ => {}
                }
            }
            None => {}
        }
        Ok(())
    }

    fn validate_pane_dynamic_text_settings(
        &mut self,
        idx: usize,
        pane: &TerminalUIPaneConfig,
    ) -> Result<()> {
        match &pane.dynamic_text {
            Some(dynamic_text) => match &dynamic_text.python {
                Some(python) => {
                    if python.code.is_empty() {
                        return Err(InvalidConfig {
                            value_ptr: format!("/config/panes/{idx}/dynamic_text/python/code"),
                            message: "may not have empty python code".into(),
                        });
                    }
                }
                None => {}
            },
            None => {}
        }

        Ok(())
    }

    fn validate_toplevel(&mut self, config: &TerminalUIConfig) -> Result<()> {
        if config.name.is_empty() {
            return Err(InvalidConfig {
                value_ptr: "/config/name".into(),
                message: "config name is empty".into(),
            });
        }

        Ok(())
    }

    fn validate_pane(&mut self, idx: usize, pane: &TerminalUIPaneConfig) -> Result<()> {
        let value_ptr = format!("/config/panes/{idx}/name");
        if pane.name.is_empty() {
            return Err(InvalidConfig {
                value_ptr,
                message: format!("pane {idx}'s name is empty"),
            });
        } else if self.seen_pane_names.contains(&pane.name) {
            return Err(InvalidConfig {
                value_ptr,
                message: format!("pane {idx}'s name: {} is not unique", pane.name),
            });
        }
        self.seen_pane_names.insert(pane.name.clone());

        self.validate_pane_frame(idx, pane)?;
        self.validate_pane_border_settings(idx, pane)?;
        self.validate_pane_dynamic_text_settings(idx, pane)?;

        Ok(())
    }

    pub fn validate(&mut self, config: &TerminalUIConfig) -> Result<()> {
        self.validate_toplevel(config)?;
        for (pane, idx) in config.panes.iter().zip(0..config.panes.len()).into_iter() {
            self.validate_pane(idx, pane)?;
        }

        Ok(())
    }
}
