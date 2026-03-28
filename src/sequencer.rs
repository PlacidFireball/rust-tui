use std::fmt::Display;

#[allow(dead_code, unused)]
pub struct EscapeSequencer {
    pub term_width: usize,
    pub term_height: usize,

    pos_x: usize,
    pos_y: usize,
}

#[allow(dead_code, unused)]
impl EscapeSequencer {
    pub fn new(term_width: usize, term_height: usize) -> Self {
        EscapeSequencer {
            term_width,
            term_height,
            pos_x: 0,
            pos_y: 0,
        }
    }

    pub fn on_resize(&mut self, term_width: usize, term_height: usize) {
        self.term_width = term_width;
        self.term_height = term_height;
    }

    pub fn clear_screen(
        &mut self,
        clear_from_cursor_to_beginning: bool,
        clear_entire_screen: bool,
        clear_entire_screen_and_scrollback: bool,
    ) {
        self.set_cursor_position(0, 0);
        if clear_from_cursor_to_beginning {
            print!("\x1b[1J");
            return;
        } else if clear_entire_screen {
            print!("\x1b[2J");
            return;
        } else if clear_entire_screen_and_scrollback {
            print!("\x1b[3J");
            return;
        }

        print!("\x1b[2J");
    }

    pub fn move_cursor_right(&mut self, n: usize) {
        if self.pos_x + n < self.term_width {
            self.pos_x += n;
            print!("\x1b[{n}C")
        } else {
            let actually_move = self.term_width - self.pos_x;
            self.pos_x = self.term_width;
            print!("\x1b[{actually_move}C")
        }
    }

    pub fn move_cursor_left(&mut self, n: usize) {
        let actually_move = n.min(self.pos_x);
        self.pos_x -= actually_move;
        print!("\x1b[{actually_move}D")
    }

    pub fn move_cursor_up(&mut self, n: usize) {
        let actually_move = n.min(self.pos_y);
        self.pos_y -= actually_move;
        print!("\x1b[{actually_move}A")
    }

    pub fn move_cursor_down(&mut self, n: usize) {
        if self.pos_y + n < self.term_height {
            self.pos_y += n;
            print!("\x1b[{n}B")
        } else {
            let actually_move = self.term_height - self.pos_y;
            self.pos_y = self.term_height;
            print!("\x1b[{actually_move}B")
        }
    }

    pub fn set_cursor_position(&mut self, x: usize, y: usize) {
        eprintln!("set_cursor_position: x: {x} y: {y}");
        self.pos_x = x.min(self.term_width);
        self.pos_y = y.min(self.term_height);
        print!("\x1b[{};{}H", self.pos_y + 1, self.pos_x + 1)
    }
}

#[allow(unused)]
pub enum AnsiCode {
    Reset,
    Bold,
    Faint,
    Italic,
    Underline,
    SlowBlink,
    RapidBlink,
    SwapFgBg,
    Conceal,
    StrikeThrough,
    PrimaryFont,
    AlternativeFont(u8), // 1-9
    DoubleUnderline,
    NormalIntensity,
    NeitherItalicNorBlackletter,
    NotUnderlined,
    NotBlinking,
    ProportionalSpacing,
    NotReversed,
    Reveal,
    NotStrikethrough,
    FgBlack,
    FgRed,
    FgGreen,
    FgYellow,
    FgBlue,
    FgMagenta,
    FgCyan,
    FgWhite,
    FgColor(u8, u8, u8), // RGB
    FgDefault,
    BgBlack,
    BgRed,
    BgGreen,
    BgYellow,
    BgBlue,
    BgMagenta,
    BgCyan,
    BgWhite,
    BgColor(u8, u8, u8), // RGB
    BgDefault,
    FgBrightBlack,
    FgBrightRed,
    FgBrightGreen,
    FgBrightYellow,
    FgBrightBlue,
    FgBrightMagenta,
    FgBrightCyan,
    FgBrightWhite,
    BgBrightBlack,
    BgBrightRed,
    BgBrightGreen,
    BgBrightYellow,
    BgBrightBlue,
    BgBrightMagenta,
    BgBrightCyan,
    BgBrightWhite,
}

impl AnsiCode {
    pub fn as_str(&self) -> String {
        match self {
            AnsiCode::Reset => "\x1b[0m".into(),
            AnsiCode::Bold => "\x1b[1m".into(),
            AnsiCode::Faint => "\x1b[2m".into(),
            AnsiCode::Italic => "\x1b[3m".into(),
            AnsiCode::Underline => "\x1b[4m".into(),
            AnsiCode::SlowBlink => "\x1b[5m".into(),
            AnsiCode::RapidBlink => "\x1b[6m".into(),
            AnsiCode::SwapFgBg => "\x1b[7m".into(),
            AnsiCode::Conceal => "\x1b[8m".into(),
            AnsiCode::StrikeThrough => "\x1b[9m".into(),
            AnsiCode::PrimaryFont => "\x1b[10m".into(),
            AnsiCode::AlternativeFont(n) => format!("\x1b[{}m", 10 + n.clamp(&1, &9)),
            AnsiCode::DoubleUnderline => "\x1b[21m".into(),
            AnsiCode::NormalIntensity => "\x1b[22m".into(),
            AnsiCode::NeitherItalicNorBlackletter => "\x1b[23m".into(),
            AnsiCode::NotUnderlined => "\x1b[24m".into(),
            AnsiCode::NotBlinking => "\x1b[25m".into(),
            AnsiCode::ProportionalSpacing => "\x1b[26m".into(),
            AnsiCode::NotReversed => "\x1b[27m".into(),
            AnsiCode::Reveal => "\x1b[28m".into(),
            AnsiCode::NotStrikethrough => "\x1b[29m".into(),
            AnsiCode::FgBlack => "\x1b[30m".into(),
            AnsiCode::FgRed => "\x1b[31m".into(),
            AnsiCode::FgGreen => "\x1b[32m".into(),
            AnsiCode::FgYellow => "\x1b[33m".into(),
            AnsiCode::FgBlue => "\x1b[34m".into(),
            AnsiCode::FgMagenta => "\x1b[35m".into(),
            AnsiCode::FgCyan => "\x1b[36m".into(),
            AnsiCode::FgWhite => "\x1b[37m".into(),
            AnsiCode::FgColor(r, g, b) => format!("\x1b[38;2;{r};{g};{b}m"),
            AnsiCode::FgDefault => "\x1b[39m".into(),
            AnsiCode::BgBlack => "\x1b[40m".into(),
            AnsiCode::BgRed => "\x1b[41m".into(),
            AnsiCode::BgGreen => "\x1b[42m".into(),
            AnsiCode::BgYellow => "\x1b[43m".into(),
            AnsiCode::BgBlue => "\x1b[44m".into(),
            AnsiCode::BgMagenta => "\x1b[45m".into(),
            AnsiCode::BgCyan => "\x1b[46m".into(),
            AnsiCode::BgWhite => "\x1b[47m".into(),
            AnsiCode::BgColor(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
            AnsiCode::BgDefault => "\x1b[49m".into(),
            AnsiCode::FgBrightBlack => "\x1b[90m".into(),
            AnsiCode::FgBrightRed => "\x1b[91m".into(),
            AnsiCode::FgBrightGreen => "\x1b[92m".into(),
            AnsiCode::FgBrightYellow => "\x1b[93m".into(),
            AnsiCode::FgBrightBlue => "\x1b[94m".into(),
            AnsiCode::FgBrightMagenta => "\x1b[95m".into(),
            AnsiCode::FgBrightCyan => "\x1b[96m".into(),
            AnsiCode::FgBrightWhite => "\x1b[97m".into(),
            AnsiCode::BgBrightBlack => "\x1b[100m".into(),
            AnsiCode::BgBrightRed => "\x1b[101m".into(),
            AnsiCode::BgBrightGreen => "\x1b[102m".into(),
            AnsiCode::BgBrightYellow => "\x1b[103m".into(),
            AnsiCode::BgBrightBlue => "\x1b[104m".into(),
            AnsiCode::BgBrightMagenta => "\x1b[105m".into(),
            AnsiCode::BgBrightCyan => "\x1b[106m".into(),
            AnsiCode::BgBrightWhite => "\x1b[107m".into(),
        }
    }
}

impl Display for AnsiCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
