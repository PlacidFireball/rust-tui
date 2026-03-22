use std::io::Write;

use crate::escape_sequencer::EscapeSequencer;

#[allow(dead_code, unused)]
#[derive(Debug, Clone)]
pub struct TerminalSurface {
    pub pos_x: usize,
    pub pos_y: usize,
    pub width: usize,
    pub height: usize,

    pub id: String,

    lines: Vec<String>,
}
#[allow(dead_code, unused)]
impl TerminalSurface {
    pub fn new(pos_x: usize, pos_y: usize, width: usize, height: usize, id: String) -> Self {
        TerminalSurface {
            pos_x,
            pos_y,
            width,
            height,
            id,
            lines: vec![],
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

    pub fn set_text(&mut self, text: String) {
        self.lines.clear();
        for line in text.split('\n') {
            let mut remaining = line;
            loop {
                let visible = Self::visible_len(remaining);
                if visible <= self.width {
                    self.lines.push(remaining.to_string());
                    break;
                }

                let width_byte_offset = Self::byte_offset_after_visible(remaining, self.width);

                // Find the last space within visible width
                let split_byte = match remaining[..width_byte_offset].rfind(' ') {
                    Some(pos) => pos,
                    None => width_byte_offset,
                };

                self.lines.push(remaining[..split_byte].to_string());
                remaining = remaining[split_byte..].trim_start();
            }
        }
    }
}

#[allow(dead_code, unused)]
pub struct TerminalRenderer {
    pub sequencer: EscapeSequencer,
    surfaces: Vec<TerminalSurface>,
}

#[allow(dead_code, unused)]
impl TerminalRenderer {
    pub fn new(sequencer: EscapeSequencer) -> Self {
        TerminalRenderer {
            sequencer,
            surfaces: vec![],
        }
    }

    pub fn add_surface(&mut self, surface: TerminalSurface) {
        self.surfaces.push(surface);
    }

    fn render(&mut self) {
        eprintln!("render");
        for surface in self.surfaces.clone() {
            self.render_surface(surface);
        }
        std::io::stdout().flush().expect("failure to flush stdout");
    }

    fn render_surface(&mut self, surface: TerminalSurface) {
        eprintln!("render_surface: {:?}", surface);
        for (line, i) in surface.lines.iter().zip(0..surface.lines.len()) {
            eprintln!("render_line: {i} {line}");
            self.sequencer
                .set_cursor_position(surface.pos_x, surface.pos_y + i);
            print!("{line}")
        }
    }

    pub fn on_change(&mut self) {
        eprintln!("on_change");
        self.render();
    }

    pub fn update_surface<T: FnMut(TerminalSurface) -> TerminalSurface>(
        &mut self,
        id: String,
        mut callback: T,
    ) {
        let mut found = false;
        for i in 0..self.surfaces.len() {
            if self.surfaces[i].id == id {
                found = true;
                self.surfaces[i] = callback(self.surfaces[i].clone());
                eprintln!("updated surface: {:?}", self.surfaces[i]);
                break;
            }
        }
        if found {
            self.on_change();
        }
    }

    pub fn on_resize(&mut self, term_width: usize, term_height: usize) {
        self.sequencer.on_resize(term_width, term_height);
        eprintln!("on_resize");
    }

    pub fn clear_screen(&mut self) {
        self.sequencer.clear_screen(false, false, false);
        eprintln!("clear_screen");
    }
}
