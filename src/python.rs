use std::sync::{Arc, Mutex};

use pyo3::exceptions::PyOSError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::config::PythonSettings;
use crate::pane_ui::PaneUi;
use crate::renderer::TerminalRenderer;
use crate::util;

#[pyclass(name = "PaneUi", frozen)]
pub struct PyPaneUi {
    ui: PaneUi,
}

#[pymethods]
impl PyPaneUi {
    fn set_text(&self, py: Python<'_>, text: String) {
        py.detach(|| self.ui.set_text(text));
    }
}

pub struct PythonPaneRuntime {
    settings: PythonSettings,
    globals: Py<PyDict>,
    pane_id: String,
}

impl PythonPaneRuntime {
    pub fn new(
        settings: PythonSettings,
        pane_id: String,
        renderer: Arc<Mutex<TerminalRenderer>>,
    ) -> PyResult<Self> {
        Python::attach(|py| {
            let ui = Py::new(
                py,
                PyPaneUi {
                    ui: PaneUi::new(pane_id.clone(), renderer),
                },
            )?;
            let globals = PyDict::new(py);
            globals.set_item("ui", ui)?;
            if let Some(script) = &settings.script {
                globals.set_item("__file__", script)?;
            }

            Ok(PythonPaneRuntime {
                settings,
                globals: globals.unbind(),
                pane_id,
            })
        })
    }

    pub fn run(&self) -> PyResult<()> {
        Python::attach(|py| {
            let (source, filename) = match (&self.settings.code, &self.settings.script) {
                (Some(code), None) => (code.clone(), format!("<{}>", self.pane_id)),
                (None, Some(script)) => {
                    let source = std::fs::read_to_string(script).map_err(|e| {
                        PyOSError::new_err(format!("failed to read script {script}: {e}"))
                    })?;
                    (source, script.clone())
                }
                _ => unreachable!("should never be hit due to validation"),
            };

            let builtins = py.import("builtins")?;
            let compiled = builtins.call_method1("compile", (source, filename, "exec"))?;
            builtins.call_method1("exec", (compiled, self.globals.bind(py)))?;

            Ok(())
        })
    }

    pub async fn run_on_schedule(&self, refresh_millis: Option<u64>) {
        let mut refresh = util::interval_from_millis(refresh_millis);
        loop {
            if let Some(refresh) = &mut refresh {
                refresh.tick().await;
            }

            match self.run() {
                Ok(_) => {}
                Err(e) => eprintln!(
                    "python error in pane {pane_id}: {e}",
                    pane_id = self.pane_id
                ),
            }

            if refresh.is_none() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    use crate::pane_ui::test_support::{pane_lines, renderer_with_panes};

    fn runtime(
        code: &str,
        pane_id: &str,
        renderer: &Arc<Mutex<TerminalRenderer>>,
    ) -> PythonPaneRuntime {
        let settings = PythonSettings {
            code: Some(code.into()),
            script: None,
        };
        PythonPaneRuntime::new(settings, pane_id.into(), renderer.clone()).expect("runtime")
    }

    #[test]
    fn python_set_text_updates_pane() {
        let renderer = renderer_with_panes(&["pane"]);

        runtime(r#"ui.set_text("hello from python")"#, "pane", &renderer)
            .run()
            .expect("python should run");

        assert_eq!(pane_lines(&renderer, "pane"), vec!["hello from python"]);
    }

    #[test]
    fn python_functions_can_use_ui() {
        let renderer = renderer_with_panes(&["pane"]);

        runtime(
            r#"
def render(name):
    ui.set_text(f"hello {name}")

render("function")
"#,
            "pane",
            &renderer,
        )
        .run()
        .expect("python should run");

        assert_eq!(pane_lines(&renderer, "pane"), vec!["hello function"]);
    }

    #[test]
    fn python_set_text_rejects_non_str() {
        let renderer = renderer_with_panes(&["pane"]);

        let result = runtime("ui.set_text(42)", "pane", &renderer).run();

        let message = result.expect_err("python should fail").to_string();
        assert!(message.contains("TypeError"), "unexpected error: {message}");
        assert_eq!(pane_lines(&renderer, "pane"), vec!["initial"]);
    }

    #[test]
    fn python_error_after_set_text_keeps_text() {
        let renderer = renderer_with_panes(&["pane"]);

        let result = runtime(
            r#"ui.set_text("before")
raise ValueError("boom")"#,
            "pane",
            &renderer,
        )
        .run();

        let message = result.expect_err("python should fail").to_string();
        assert!(message.contains("boom"), "unexpected error: {message}");
        assert_eq!(pane_lines(&renderer, "pane"), vec!["before"]);
    }

    #[test]
    fn python_ui_is_bound_to_its_own_pane() {
        let renderer = renderer_with_panes(&["left", "right"]);

        runtime(r#"ui.set_text("L")"#, "left", &renderer)
            .run()
            .expect("python should run");
        runtime(r#"ui.set_text("R")"#, "right", &renderer)
            .run()
            .expect("python should run");

        assert_eq!(pane_lines(&renderer, "left"), vec!["L"]);
        assert_eq!(pane_lines(&renderer, "right"), vec!["R"]);
    }

    #[test]
    fn python_panes_do_not_share_globals() {
        let renderer = renderer_with_panes(&["left", "right"]);

        runtime("shared = 'left'", "left", &renderer)
            .run()
            .expect("python should run");
        runtime(
            r#"ui.set_text(str(globals().get("shared")))"#,
            "right",
            &renderer,
        )
        .run()
        .expect("python should run");

        assert_eq!(pane_lines(&renderer, "right"), vec!["None"]);
    }

    fn script_runtime(
        script: &str,
        pane_id: &str,
        renderer: &Arc<Mutex<TerminalRenderer>>,
    ) -> PythonPaneRuntime {
        let settings = PythonSettings {
            code: None,
            script: Some(script.into()),
        };
        PythonPaneRuntime::new(settings, pane_id.into(), renderer.clone()).expect("runtime")
    }

    fn temp_script(name: &str, source: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "rust-tui-python-test-{pid}-{name}.py",
            pid = std::process::id()
        ));
        std::fs::write(&path, source).expect("failure to write test script");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn python_runs_script_file() {
        let renderer = renderer_with_panes(&["pane"]);
        let script = temp_script(
            "runs",
            "import os\nui.set_text(f\"from {os.path.basename(__file__)}\")\n",
        );

        let result = script_runtime(&script, "pane", &renderer).run();
        std::fs::remove_file(&script).expect("failure to remove test script");

        result.expect("python should run");
        let expected = format!(
            "from rust-tui-python-test-{pid}-runs.py",
            pid = std::process::id()
        );
        assert_eq!(pane_lines(&renderer, "pane"), vec![expected]);
    }

    #[test]
    fn python_script_is_reread_each_run() {
        let renderer = renderer_with_panes(&["pane"]);
        let script = temp_script("reread", "ui.set_text('first')");
        let runtime = script_runtime(&script, "pane", &renderer);

        runtime.run().expect("python should run");
        std::fs::write(&script, "ui.set_text('second')").expect("failure to rewrite script");
        let result = runtime.run();
        std::fs::remove_file(&script).expect("failure to remove test script");

        result.expect("python should run");
        assert_eq!(pane_lines(&renderer, "pane"), vec!["second"]);
    }

    #[test]
    fn python_script_traceback_names_file() {
        let renderer = renderer_with_panes(&["pane"]);
        let script = temp_script("traceback", "raise ValueError('boom')");

        let result = script_runtime(&script, "pane", &renderer).run();
        std::fs::remove_file(&script).expect("failure to remove test script");

        let traceback = Python::attach(|py| {
            let err = result.expect_err("python should fail");
            err.traceback(py)
                .expect("error should have a traceback")
                .format()
                .expect("traceback should format")
        });
        assert!(
            traceback.contains(&script),
            "unexpected traceback: {traceback}"
        );
    }

    #[test]
    fn python_missing_script_file_errors() {
        let renderer = renderer_with_panes(&["pane"]);

        let result = script_runtime("/definitely/not/a/real/script.py", "pane", &renderer).run();

        let message = result.expect_err("python should fail").to_string();
        assert!(
            message.contains("failed to read script"),
            "unexpected error: {message}"
        );
        assert_eq!(pane_lines(&renderer, "pane"), vec!["initial"]);
    }

    #[tokio::test(start_paused = true)]
    async fn python_refresh_reuses_globals_on_each_tick() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = runtime(
            r#"count = globals().get("count", 0) + 1
ui.set_text(f"tick {count}")"#,
            "pane",
            &renderer,
        );

        let task = tokio::spawn(async move { runtime.run_on_schedule(Some(1000)).await });

        tokio::time::sleep(Duration::from_millis(2500)).await;
        task.abort();

        assert_eq!(pane_lines(&renderer, "pane"), vec!["tick 3"]);
    }
}
