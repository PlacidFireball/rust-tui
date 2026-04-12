use std::{
    ffi::CString,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::{
    config::{TerminalUIConfig, TerminalUIConfigValidator},
    renderer::TerminalRenderer,
    sequencer::EscapeSequencer,
};
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyDict;

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
            if RESIZED.swap(false, Ordering::Relaxed) {
                self.resize()?;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
