pub(crate) mod renderer;
pub(crate) mod sequencer;
pub(crate) mod ui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread::sleep, time::Duration};

use crate::renderer::{TerminalPane, TerminalRenderer};
use crate::sequencer::{AnsiCode, EscapeSequencer};
use crate::ui::BorderCharacters;

static RESIZED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigwinch(_: libc::c_int) {
    RESIZED.store(true, Ordering::Relaxed);
}

fn get_term_size() -> (usize, usize) {
    unsafe {
        let mut winsize = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize);
        (winsize.ws_col as usize, winsize.ws_row as usize)
    }
}

fn main() {
    // Redirect stderr to a log file so eprintln! traces never bleed into the
    // terminal UI. Both the file descriptor swap and the open are done via libc
    // so we stay dependency-free.
    use AnsiCode::{FgBlue, Reset};
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

    // Register SIGWINCH handler
    unsafe {
        libc::signal(
            libc::SIGWINCH,
            handle_sigwinch as *const () as libc::sighandler_t,
        );
    }

    let (mut term_width, mut term_height) = get_term_size();

    let mut renderer = TerminalRenderer::new(EscapeSequencer::new(term_width, term_height));

    let cage = format!(
        "{g}░░░░░░░░░░░░░░{y}▄▄▄▄▄▄▄▄▄▄▄▄{g}░░░░░░░░░░░░░░{r}
{g}░░░░░░░░░░░░{y}▄████████████████▄{g}░░░░░░░░░░{r}
{g}░░░░░░░░░░{y}▄██▀░░░░░░░▀▀████████▄{g}░░░░░░░░{r}
{g}░░░░░░░░░{y}▄█▀░░░░░░░░░░░░░▀▀██████▄{g}░░░░░░{r}
{g}░░░░░░░░░{y}███▄░░░░░░░░░░░░░░░▀██████{g}░░░░░{r}
{g}░░░░░░░░{y}▄░░▀▀█░░░░░░░░░░░░░░░░██████{g}░░░░{r}
{g}░░░░░░░{y}█▄██▀▄░░░░░▄███▄▄░░░░░░███████{g}░░░{r}
{g}░░░░░░{y}▄▀▀▀██▀░░░░░▄▄▄░░▀█░░░░█████████{g}░░{r}
{g}░░░░░{y}▄▀░░░░▄▀░▄░░█▄██▀▄░░░░░██████████{g}░░{r}
{g}░░░░░{y}█░░░░▀░░░█░░░▀▀▀▀▀░░░░░██████████▄{g}░{r}
{g}░░░░░░░{y}▄█▄░░░░░▄░░░░░░░░░░░░██████████▀{g}░{r}
{g}░░░░░░{y}█▀░░░░▀▀░░░░░░░░░░░░░███▀███████{g}░░{r}
{g}░░░▄▄░{y}▀░▄░░░░░░░░░░░░░░░░░░▀░░░██████{g}░░░{r}
{y}██████░░█▄█▀░▄░░██░░░░░░░░░░░█▄█████▀{g}░░░{r}
{y}██████░░░▀████▀░▀░░░░░░░░░░░▄▀█████████▄{r}
{y}██████░░░░░░░░░░░░░░░░░░░░▀▄████████████{r}
{y}██████░░▄░░░░░░░░░░░░░▄░░░██████████████{r}
{y}██████░░░░░░░░░░░░░▄█▀░░▄███████████████{r}
{y}███████▄▄░░░░░░░░░▀░░░▄▀▄███████████████{r}",
        g = AnsiCode::FgColor(130, 130, 130),
        y = AnsiCode::FgColor(220, 180, 100),
        r = AnsiCode::Reset,
    );

    renderer.clear_screen();
    renderer.add_surface(TerminalPane::new(
        0,
        0,
        term_width / 2,
        term_height,
        "Left Box".into(),
    ));
    renderer.add_surface(TerminalPane::new(
        term_width / 2 + 1,
        0,
        term_width / 2,
        term_height,
        "Right Box".into(),
    ));
    renderer.update_pane("Left Box".into(), |mut surface| {
        surface.set_text(cage.clone(), Some(BorderCharacters::rounded()));
        surface
    });
    renderer.update_pane("Right Box".into(), |mut surface| {
        surface.set_text(
            format!("{FgBlue}Hello from the right box!{Reset}",),
            Some(BorderCharacters::rounded()),
        );
        surface
    });

    loop {
        if RESIZED.swap(false, Ordering::Relaxed) {
            (term_width, term_height) = get_term_size();
            renderer.on_resize(term_width, term_height);
            eprintln!("resized: {term_width}x{term_height}");
        }
        sleep(Duration::from_millis(100));
    }
}
