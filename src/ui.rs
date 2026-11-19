use std::{
    collections::VecDeque, ffi::CString, io::Read, sync::{
        Arc, Mutex, RwLock, atomic::{AtomicBool, Ordering}
    }, time::Duration
};

use crate::{
    config::{TerminalUIConfig, TerminalUIConfigValidator},
    renderer::TerminalRenderer,
    sequencer::EscapeSequencer, util,
};
use pub_fields_macro::pub_fields;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use termion::{event::{Event, Key}, input::TermRead};

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
use tokio::task::JoinHandle;

#[pub_fields]
#[derive(Debug, Clone, PartialEq)]
struct TerminalUIInputEvent {
    c: Option<char>,
    function: Option<u8>,
    key: Option<Key>,

    ctrl: bool,
    alt: bool,
    shift: bool,
}
impl From<Key> for TerminalUIInputEvent {
    fn from(key: Key) -> Self {
        match key {
            /* --- Characters --- */
            Key::Char(c) => TerminalUIInputEvent::new_char(c, false, false),
            Key::Up => TerminalUIInputEvent::new_key(Key::Up, false, false, false),
            Key::Down => TerminalUIInputEvent::new_key(Key::Down, false, false, false),
            Key::Left => TerminalUIInputEvent::new_key(Key::Left, false, false, false),
            Key::Right => TerminalUIInputEvent::new_key(Key::Right, false, false, false),

            Key::PageUp => TerminalUIInputEvent::new_key(Key::PageUp, false, false, false),
            Key::PageDown => TerminalUIInputEvent::new_key(Key::PageDown, false, false, false),

            // TODO: what if ctrl+alt?

            /* --- Ctrl --- */
            Key::Ctrl(c) => TerminalUIInputEvent::new_char(c, true, false),
            Key::CtrlUp => TerminalUIInputEvent::new_key(Key::CtrlUp, true, false, false),
            Key::CtrlDown => TerminalUIInputEvent::new_key(Key::CtrlDown, true, false, false),
            Key::CtrlLeft => TerminalUIInputEvent::new_key(Key::CtrlLeft, true, false, false),
            Key::CtrlRight => TerminalUIInputEvent::new_key(Key::CtrlRight, true, false, false),
            Key::CtrlHome => TerminalUIInputEvent::new_key(Key::CtrlHome, true, false, false),
            Key::CtrlEnd => TerminalUIInputEvent::new_key(Key::CtrlEnd, true, false, false),

            /* --- Alt --- */
            Key::Alt(c) => TerminalUIInputEvent::new_char(c, false, true),
            Key::AltUp => TerminalUIInputEvent::new_key(Key::AltUp, false, true, false),
            Key::AltDown => TerminalUIInputEvent::new_key(Key::AltDown, false, true, false),
            Key::AltLeft => TerminalUIInputEvent::new_key(Key::AltLeft, false, true, false),
            Key::AltRight => TerminalUIInputEvent::new_key(Key::AltRight, false, true, false),

            /* --- Shift --- */
            Key::ShiftUp => TerminalUIInputEvent::new_key(Key::ShiftUp, false, false, true),
            Key::ShiftDown => TerminalUIInputEvent::new_key(Key::ShiftDown, false, false, true),
            Key::ShiftRight => TerminalUIInputEvent::new_key(Key::ShiftRight, false, false, true),
            Key::ShiftLeft => TerminalUIInputEvent::new_key(Key::ShiftLeft, false, false, true),

            Key::Backspace => TerminalUIInputEvent::new_key(Key::Backspace, false, false, false),
            Key::Home => TerminalUIInputEvent::new_key(Key::Home, false, false, false),
            Key::End => TerminalUIInputEvent::new_key(Key::End, false, false, false),
            Key::BackTab => TerminalUIInputEvent::new_key(Key::BackTab, false, false, false),
            Key::Delete => TerminalUIInputEvent::new_key(Key::Delete, false, false, false),
            Key::Insert => TerminalUIInputEvent::new_key(Key::Insert, false, false, false),
            Key::F(n) => TerminalUIInputEvent::new_function(n),
            Key::Null => TerminalUIInputEvent::new_key(Key::Null, false, false, false),
            Key::Esc => TerminalUIInputEvent::new_key(Key::Esc, false, false, false),
            _ => unreachable!("__NotComplete? {key:?}")
        }
    }
}
impl TerminalUIInputEvent {
    pub fn new_function(n: u8) -> Self {
        Self {
            c: None,
            function: Some(n),
            key: None,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    pub fn new_char(c: char, ctrl: bool, alt: bool) -> Self {
        Self {
            c: Some(c),
            function: None,
            key: None,
            ctrl,
            alt,
            shift: c.is_uppercase(),
        }
    }

    pub fn new_key(k: Key, ctrl: bool, alt: bool, shift: bool) -> Self {
        Self {
            c: None,
            function: None,
            key: Some(k),
            ctrl,
            alt,
            shift,
        }
    }
}

/// Installs a thread-local `sys.stdout` redirector into the Python interpreter.
/// Must be called once before any threads execute Python code.
/// After this, each thread must set `sys.stdout._local.buffer = io.StringIO()`
/// before running code and call `sys.stdout._local.buffer.getvalue()` to retrieve output.
fn install_thread_local_stdout() {
    Python::attach(|py| {
        py.run(
            c_str!(
                "import sys, io, threading

class _ThreadLocalStdout:
    def __init__(self):
        self._local = threading.local()
    def write(self, s):
        buf = getattr(self._local, 'buffer', None)
        if buf is not None:
            buf.write(s)
    def flush(self): pass

sys.stdout = _ThreadLocalStdout()
"
            ),
            None,
            None,
        )
        .expect("failed to install thread-local stdout redirector");
    });
}

pub(crate) type Result<T> = std::result::Result<T, TerminalUIError>;

#[allow(dead_code)]
pub struct TerminalUI {
    renderer: Arc<Mutex<TerminalRenderer>>,
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

    pub fn from_config(config_path: &str) -> Result<Self> {
        util::redirect_stderr();

        let config: TerminalUIConfig = match std::fs::File::open(config_path) {
            Ok(file) => match serde_json::from_reader(file) {
                Ok(config) => config,
                Err(_) => return Err(InvalidJson), 
            },
            Err(_) => return Err(ConfigMissing),
        };

        println!("{config:?}");

        let (term_width, term_height) = Self::get_term_size();
        let mut renderer: TerminalRenderer =
            TerminalRenderer::new(EscapeSequencer::new(term_width, term_height));

        let mut config_validator = TerminalUIConfigValidator::new(term_width, term_height);
        config_validator.validate(&config)?;

        renderer.clear_screen();
        renderer.with_config(&config);

        Ok(TerminalUI {
            renderer: Arc::new(Mutex::new(renderer)),
            config,
        })
    }

    pub fn resize(&mut self) -> Result<()> {
        let (new_term_width, new_term_height) = Self::get_term_size();
        let mut renderer = self.renderer.lock().expect("failure to lock renderer");
        renderer.on_resize(new_term_width, new_term_height);
        Ok(())
    }

    fn spawn_dynamic_python_text_task(
        code: String,
        pane_id: String,
        renderer_ref: Arc<Mutex<TerminalRenderer>>,
    ) -> Option<JoinHandle<()>> {
        // TODO: if refresh duration, schedule a recurring task
        Some(tokio::spawn(async move {
            Python::attach(|py| {
                let code_cstring = CString::new(code.as_str()).expect("code has null byte");

                // Point this thread's buffer at a fresh StringIO.
                py.run(
                    c_str!("import sys, io\nsys.stdout._local.buffer = io.StringIO()\n"),
                    None,
                    None,
                )
                .expect("failed to set thread-local stdout buffer");

                let locals = PyDict::new(py);
                let result = py.run(code_cstring.as_c_str(), None, Some(&locals));

                let output: String = py
                    .eval(c_str!("sys.stdout._local.buffer.getvalue()"), None, None)
                    .and_then(|v| v.extract::<String>())
                    .unwrap_or_default();

                match result {
                    Ok(_) => {
                        let mut renderer =
                            renderer_ref.lock().expect("failure to get renderer lock");

                        renderer.update_pane(pane_id, |mut pane| {
                            pane.set_text(output.clone());
                            pane
                        });
                    }
                    Err(e) => {
                        eprintln!("python error {e:?}")
                    }
                }
            })
        }))
    }

    pub async fn running_loop(&mut self) -> Result<()> {
        // Register SIGWINCH handler
        unsafe {
            libc::signal(
                libc::SIGWINCH,
                handle_sigwinch as *const () as libc::sighandler_t,
            );
        }

        let mut first_python_dynamic_task = true;

        let input_queue: Arc<RwLock<VecDeque<TerminalUIInputEvent>>> = Arc::new(RwLock::new(VecDeque::new()));
        let new_input_arrived: AtomicBool = AtomicBool::new(false);

        for shared_pane in self
            .renderer
            .lock()
            .expect("failure to get lock on renderer")
            .get_panes()
        {
            {
                let pane = shared_pane.lock().expect("failure to get lock on pane");

                let _join_handle_opt = match &pane.dynamic_text {
                    Some(dynamic_text) => match &dynamic_text.python {
                        Some(python) => {
                            if first_python_dynamic_task {
                                // install a thead-local stdout so that different python
                                // invocations don't clobber each other
                                install_thread_local_stdout();
                                first_python_dynamic_task = false;
                            }
                            Self::spawn_dynamic_python_text_task(
                                python.code.clone(),
                                pane.id.clone(),
                                self.renderer.clone(),
                            )
                        }
                        None => None,
                    },
                    None => None,
                };
            }
        }

        loop {
            async fn gather_input_event(input_queue: Arc<RwLock<VecDeque<TerminalUIInputEvent>>>) {
                let write = |evt| {
                    match input_queue.write() {
                        Ok(mut write) => write.push_back(evt),
                        Err(e) => panic!("poisoned write lock - e: {e}"),
                    };
                };
                for evt_res in std::io::stdin().events() {
                    match evt_res {
                        Ok(evt) => match evt {
                            Event::Key(key) => write(key.into()),
                            Event::Mouse(mouse_event) => {
                                eprintln!("mouse event received: {mouse_event:?}")
                            },
                            Event::Unsupported(_items) => todo!(),
                        },
                        Err(e) => eprintln!("Error reading from stdin: {e}")
                    }
                }
            }

            async fn process_input_event(input_queue: Arc<RwLock<VecDeque<TerminalUIInputEvent>>>) {
                let event: Option<TerminalUIInputEvent> = match input_queue.write() {
                    Ok(mut queue) => {
                        queue.pop_front()
                    }
                    Err(e) => panic!("poisoned write lock - e: {e}"),
                };

                match event {
                    Some(event) => {
                        eprintln!("received event {event:?}");
                    },
                    _ => {},
                }
            }

            let _gather_input_event_handle = tokio::spawn(gather_input_event(input_queue.clone()));
            let _process_input_event_handle = tokio::spawn(process_input_event(input_queue.clone()));

            if RESIZED.swap(false, Ordering::Relaxed) {
                self.resize()?;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
