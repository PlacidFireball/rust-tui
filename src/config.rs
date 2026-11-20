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
    none,
}

#[allow(dead_code)]
impl BorderPreset {
    pub fn to_characters(&self) -> BorderCharacters {
        match self {
            Self::rounded => BorderCharacters::rounded(),
            Self::ascii => BorderCharacters::ascii(),
            Self::none => BorderCharacters::none(),
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

    pub fn none() -> Self {
        BorderCharacters {
            top_left: ' ',
            top_right: ' ',
            bottom_left: ' ',
            bottom_right: ' ',
            left: ' ',
            right: ' ',
            horizontal: ' ',
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
    code: Option<String>,
    script: Option<String>,
}

#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LuaSettings {
    code: Option<String>,
    script: Option<String>,
}

#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DynamicText {
    refresh_millis: Option<u64>,
    python: Option<PythonSettings>,
    lua: Option<LuaSettings>,
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

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum KeySeq {
    Character { ctrl: bool, alt: bool, char: char },
    Function { n: u8 },
    OtherKey { key: String },
    #[default]
    NoKey,
}

#[pub_fields]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KeyChord {
    keys: Vec<KeySeq>
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum TerminalUIAction {

    #[default]
    Nothing,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TerminalUIBinding {
    KeyChordBinding { chord: KeyChord, action: TerminalUIAction },
    KeyBinding { key: KeySeq, action: TerminalUIAction },
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
        let dynamic_text = match &pane.dynamic_text {
            Some(dynamic_text) => dynamic_text,
            None => return Ok(()),
        };

        if let Some(0) = dynamic_text.refresh_millis {
            return Err(InvalidConfig {
                value_ptr: format!("/config/panes/{idx}/dynamic_text/refresh_millis"),
                message: "refresh_millis must be greater than 0 - omit it to run once".into(),
            });
        }

        if dynamic_text.python.is_some() && dynamic_text.lua.is_some() {
            return Err(InvalidConfig {
                value_ptr: format!("/config/panes/{idx}/dynamic_text"),
                message: "define dynamic_text.python OR dynamic_text.lua - not both".into(),
            });
        }

        match &dynamic_text.python {
            Some(python) => {
                let python_ptr = format!("/config/panes/{idx}/dynamic_text/python");
                match (&python.code, &python.script) {
                    (Some(_), Some(_)) | (None, None) => {
                        return Err(InvalidConfig {
                            value_ptr: python_ptr,
                            message:
                                "define python.code OR python.script - config is ambigious as is"
                                    .into(),
                        });
                    }
                    (Some(code), None) if code.is_empty() => {
                        return Err(InvalidConfig {
                            value_ptr: format!("{python_ptr}/code"),
                            message: "may not have empty python code".into(),
                        });
                    }
                    (None, Some(script)) if script.is_empty() => {
                        return Err(InvalidConfig {
                            value_ptr: format!("{python_ptr}/script"),
                            message: "may not have empty python script path".into(),
                        });
                    }
                    _ => {}
                }
            }
            None => {}
        }

        match &dynamic_text.lua {
            Some(lua) => {
                let lua_ptr = format!("/config/panes/{idx}/dynamic_text/lua");
                match (&lua.code, &lua.script) {
                    (Some(_), Some(_)) | (None, None) => {
                        return Err(InvalidConfig {
                            value_ptr: lua_ptr,
                            message: "define lua.code OR lua.script - config is ambigious as is"
                                .into(),
                        });
                    }
                    (Some(code), None) if code.is_empty() => {
                        return Err(InvalidConfig {
                            value_ptr: format!("{lua_ptr}/code"),
                            message: "may not have empty lua code".into(),
                        });
                    }
                    (None, Some(script)) if script.is_empty() => {
                        return Err(InvalidConfig {
                            value_ptr: format!("{lua_ptr}/script"),
                            message: "may not have empty lua script path".into(),
                        });
                    }
                    _ => {}
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn validate_dynamic_text(dynamic_text: Value) -> Result<()> {
        let config: TerminalUIConfig = serde_json::from_value(serde_json::json!({
            "name": "test",
            "panes": [{
                "name": "pane",
                "frame": { "width": "100%", "height": "100%", "x": 0, "y": 0 },
                "dynamic_text": dynamic_text,
            }],
        }))
        .expect("test config should deserialize");

        TerminalUIConfigValidator::new(80, 24).validate(&config)
    }

    fn assert_invalid_at(result: Result<()>, expected_ptr: &str) {
        match result {
            Err(InvalidConfig { value_ptr, .. }) => assert_eq!(value_ptr, expected_ptr),
            other => panic!("expected InvalidConfig at {expected_ptr}, got {other:?}"),
        }
    }

    #[test]
    fn lua_code_is_valid() {
        let result = validate_dynamic_text(serde_json::json!({
            "lua": { "code": "ui.set_text('hi')" },
        }));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn lua_script_is_valid() {
        let result = validate_dynamic_text(serde_json::json!({
            "lua": { "script": "pane.lua" },
        }));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn lua_code_and_script_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({
            "lua": { "code": "ui.set_text('hi')", "script": "pane.lua" },
        }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/lua");
    }

    #[test]
    fn lua_neither_code_nor_script_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({ "lua": {} }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/lua");
    }

    #[test]
    fn lua_empty_code_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({ "lua": { "code": "" } }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/lua/code");
    }

    #[test]
    fn lua_empty_script_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({ "lua": { "script": "" } }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/lua/script");
    }

    #[test]
    fn python_code_is_valid() {
        let result = validate_dynamic_text(serde_json::json!({
            "python": { "code": "ui.set_text('hi')" },
        }));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn python_script_is_valid() {
        let result = validate_dynamic_text(serde_json::json!({
            "python": { "script": "pane.py" },
        }));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn python_code_and_script_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({
            "python": { "code": "ui.set_text('hi')", "script": "pane.py" },
        }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/python");
    }

    #[test]
    fn python_neither_code_nor_script_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({ "python": {} }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/python");
    }

    #[test]
    fn python_empty_code_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({ "python": { "code": "" } }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/python/code");
    }

    #[test]
    fn python_empty_script_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({ "python": { "script": "" } }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/python/script");
    }

    #[test]
    fn zero_refresh_millis_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({
            "refresh_millis": 0,
            "lua": { "code": "ui.set_text('hi')" },
        }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text/refresh_millis");
    }

    #[test]
    fn python_and_lua_together_is_invalid() {
        let result = validate_dynamic_text(serde_json::json!({
            "python": { "code": "ui.set_text('hi')" },
            "lua": { "code": "ui.set_text('hi')" },
        }));
        assert_invalid_at(result, "/config/panes/0/dynamic_text");
    }
}
