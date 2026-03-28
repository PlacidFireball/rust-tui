use std::io::Write;

pub enum RendererError {}

type Result<T> = std::result::Result<T, RendererError>;

use crate::{
    config::{BorderCharacters, TerminalUIConfig},
    sequencer::EscapeSequencer,
};

#[allow(dead_code, unused)]
#[derive(Debug, Clone)]
pub struct TerminalPane {
    pub pos_x: usize,
    pub pos_y: usize,
    pub width: usize,
    pub height: usize,

    pub id: String,

    /// Raw wrapped content lines, without any border decoration.
    lines: Vec<String>,
    /// Active border style, if any.
    border: Option<BorderCharacters>,
}
#[allow(dead_code, unused)]
impl TerminalPane {
    pub fn new(pos_x: usize, pos_y: usize, width: usize, height: usize, id: String) -> Self {
        TerminalPane {
            pos_x,
            pos_y,
            width,
            height,
            id,
            lines: vec![],
            border: None,
        }
    }

    /// Returns the number of visible (non-ANSI-escape) characters in a string.
    fn visible_len(s: &str) -> usize {
        let mut count = 0;
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // skip until end of escape sequence (letter)
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                count += 1;
            }
        }
        count
    }

    /// Returns the byte offset after `n` visible characters, skipping ANSI escapes.
    fn byte_offset_after_visible(s: &str, n: usize) -> usize {
        let mut visible = 0;
        let mut chars = s.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if c == '\x1b' {
                for (_, c2) in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                if visible == n {
                    return i;
                }
                visible += 1;
            }
        }
        s.len()
    }

    /// Set the surface text, optionally wrapping it in a border.
    ///
    /// When `border` is `Some`, the inner content width is reduced by 2 (one
    /// column for each side glyph) and the raw wrapped lines are stored without
    /// decoration. Pass `None` to retain the previous border style unchanged,
    /// or use `set_border` to update the style independently.
    pub fn set_text(&mut self, text: String, border: Option<BorderCharacters>) {
        if let Some(style) = border {
            self.border = Some(style);
        }

        // Content width: shrink by 2 when a border is active (one column each side).
        let content_width = match &self.border {
            Some(_) => self.width.saturating_sub(2),
            None => self.width,
        };

        self.lines.clear();
        for line in text.split('\n') {
            let mut remaining = line;
            loop {
                let visible = Self::visible_len(remaining);
                if visible <= content_width {
                    self.lines.push(remaining.to_string());
                    break;
                }

                let width_byte_offset = Self::byte_offset_after_visible(remaining, content_width);

                // Find the last space within the visible content width.
                let split_byte = match remaining[..width_byte_offset].rfind(' ') {
                    Some(pos) => pos,
                    None => width_byte_offset,
                };

                self.lines.push(remaining[..split_byte].to_string());
                remaining = remaining[split_byte..].trim_start();
            }
        }
    }

    /// Compose the final lines to be rendered, applying the active border and
    /// clamping to `self.height`.
    ///
    /// When no border is active the raw lines are returned (up to `self.height`).
    /// When a border is active:
    ///   - a top border row is prepended
    ///   - up to `self.height - 2` content rows are included, each padded and
    ///     flanked by the side glyphs
    ///   - a bottom border row is appended
    ///
    /// Raw lines beyond the visible window are preserved in `self.lines` to
    /// support future scrolling.
    fn render_lines(&self) -> Vec<String> {
        match &self.border {
            None => self.lines.iter().take(self.height).cloned().collect(),
            Some(style) => {
                let ch = self
                    .border
                    .clone()
                    .or_else(|| Some(BorderCharacters::ascii()))
                    .unwrap();
                let inner_width = self.width.saturating_sub(2);
                let max_content_rows = self.height.saturating_sub(2);

                // ╭──────╮
                let top = format!(
                    "{tl}{bar}{tr}",
                    tl = ch.top_left,
                    bar = ch.horizontal.to_string().repeat(inner_width),
                    tr = ch.top_right,
                );

                // │ text │  (padded to inner_width, blank rows fill remaining height)
                let empty_row = format!(
                    "{l}{pad}{r}",
                    l = ch.left,
                    pad = " ".repeat(inner_width),
                    r = ch.right,
                );
                let content: Vec<String> = self
                    .lines
                    .iter()
                    .take(max_content_rows)
                    .map(|line| {
                        let vis = Self::visible_len(line);
                        let padding = inner_width.saturating_sub(vis);
                        format!(
                            "{l}{line}{pad}{r}",
                            l = ch.left,
                            pad = " ".repeat(padding),
                            r = ch.right,
                        )
                    })
                    .chain(std::iter::repeat(empty_row).take(
                        max_content_rows.saturating_sub(self.lines.len().min(max_content_rows)),
                    ))
                    .collect();

                // ╰──────╯
                let bottom = format!(
                    "{bl}{bar}{br}",
                    bl = ch.bottom_left,
                    bar = ch.horizontal.to_string().repeat(inner_width),
                    br = ch.bottom_right,
                );

                let mut result = Vec::with_capacity(2 + content.len());
                result.push(top);
                result.extend(content);
                result.push(bottom);
                result
            }
        }
    }
}

#[allow(dead_code, unused)]
pub struct TerminalRenderer {
    pub sequencer: EscapeSequencer,
    panes: Vec<TerminalPane>,
}

#[allow(dead_code, unused)]
impl TerminalRenderer {
    pub fn new(sequencer: EscapeSequencer) -> Self {
        TerminalRenderer {
            sequencer,
            panes: vec![],
        }
    }

    pub fn add_pane(&mut self, pane: TerminalPane) {
        self.panes.push(pane);
    }

    fn render(&mut self) {
        eprintln!("render");
        for surface in self.panes.clone() {
            self.render_surface(surface);
        }
        std::io::stdout().flush().expect("failure to flush stdout");
    }

    fn render_surface(&mut self, pane: TerminalPane) {
        eprintln!("render_surface: {:?}", pane);
        let lines = pane.render_lines();
        for (i, line) in lines.iter().enumerate() {
            self.sequencer
                .set_cursor_position(pane.pos_x, pane.pos_y + i);
            print!("{line}")
        }
    }

    pub fn on_change(&mut self) {
        eprintln!("on_change");
        self.render();
    }

    pub fn update_pane<T: FnMut(TerminalPane) -> TerminalPane>(
        &mut self,
        id: String,
        mut callback: T,
    ) {
        let mut found = false;
        for i in 0..self.panes.len() {
            if self.panes[i].id == id {
                found = true;
                self.panes[i] = callback(self.panes[i].clone());
                eprintln!("updated surface: {:?}", self.panes[i]);
                break;
            }
        }
        if found {
            self.on_change();
        }
    }

    pub fn with_config(&mut self, config: &TerminalUIConfig) -> Result<()> {
        for (pane, idx) in config.panes.iter().zip(0..config.panes.len()).into_iter() {
            unimplemented!()
        }

        Ok(())
    }

    pub fn on_resize(&mut self, term_width: usize, term_height: usize) {
        self.sequencer.on_resize(term_width, term_height);
        eprintln!("on_resize term_width: {term_width} term_height: {term_height}");
    }

    pub fn clear_screen(&mut self) {
        self.sequencer.clear_screen(false, false, false);
        eprintln!("clear_screen");
    }
}
