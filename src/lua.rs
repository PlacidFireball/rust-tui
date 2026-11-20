use std::path::Path;
use std::sync::{Arc, Mutex};

use mlua::{FromLuaMulti, IntoLuaMulti, Lua, Table};

use crate::config::LuaSettings;
use crate::pane_ui::PaneUi;
use crate::renderer::TerminalRenderer;
use crate::util;

impl PaneUi {
    fn create_ui_table(&self, lua: &Lua) -> mlua::Result<Table> {
        let lua_ui = lua.create_table()?;

        self.bind_function(lua, &lua_ui, "set_text", |ui, text: String| {
            ui.set_text(text);
            Ok(())
        })?;

        Ok(lua_ui)
    }

    fn bind_function<A, R, F>(
        &self,
        lua: &Lua,
        table: &Table,
        name: &str,
        method: F,
    ) -> mlua::Result<()>
    where
        A: FromLuaMulti,
        R: IntoLuaMulti,
        F: Fn(&PaneUi, A) -> mlua::Result<R> + Send + 'static,
    {
        let ui = self.clone();
        let function = lua.create_function(move |_, args: A| method(&ui, args))?;
        table.set(name, function)
    }
}

pub struct LuaPaneRuntime {
    lua: Lua,
    pane_id: String,
}

impl LuaPaneRuntime {
    pub fn new(pane_id: String, renderer: Arc<Mutex<TerminalRenderer>>) -> mlua::Result<Self> {
        let lua = Lua::new();
        let ui = PaneUi::new(pane_id.clone(), renderer);
        lua.globals().set("ui", ui.create_ui_table(&lua)?)?;

        Ok(LuaPaneRuntime { lua, pane_id })
    }

    pub fn run(&self, settings: &LuaSettings) -> mlua::Result<()> {
        match (&settings.code, &settings.script) {
            (Some(code), None) => self
                .lua
                .load(code.as_str())
                .set_name(format!("={}", self.pane_id))
                .exec(),
            (None, Some(script)) => self.lua.load(Path::new(script)).exec(),
            _ => unreachable!("should never be hit due to validation"),
        }
    }

    pub async fn run_on_schedule(&self, settings: &LuaSettings, refresh_millis: Option<u64>) {
        let mut refresh = util::interval_from_millis(refresh_millis);
        loop {
            if let Some(refresh) = &mut refresh {
                refresh.tick().await;
            }

            match self.run(settings) {
                Ok(_) => {}
                Err(e) => eprintln!("lua error in pane {pane_id}: {e}", pane_id = self.pane_id),
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

    fn code(code: &str) -> LuaSettings {
        LuaSettings {
            code: Some(code.into()),
            script: None,
        }
    }

    #[test]
    fn lua_set_text_updates_pane() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        runtime
            .run(&code(r#"ui.set_text("hello from lua")"#))
            .expect("lua should run");

        assert_eq!(pane_lines(&renderer, "pane"), vec!["hello from lua"]);
    }

    #[test]
    fn lua_set_text_last_call_wins() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        runtime
            .run(&code(
                r#"
                local parts = {}
                for i = 1, 3 do
                    parts[#parts + 1] = "line " .. i
                    ui.set_text(table.concat(parts, "\n"))
                end
                "#,
            ))
            .expect("lua should run");

        assert_eq!(
            pane_lines(&renderer, "pane"),
            vec!["line 1", "line 2", "line 3"]
        );
    }

    #[test]
    fn lua_set_text_coerces_numbers() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        runtime
            .run(&code("ui.set_text(42)"))
            .expect("lua should run");

        assert_eq!(pane_lines(&renderer, "pane"), vec!["42"]);
    }

    #[test]
    fn lua_set_text_rejects_nil() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        assert!(runtime.run(&code("ui.set_text()")).is_err());
        assert_eq!(pane_lines(&renderer, "pane"), vec!["initial"]);
    }

    #[test]
    fn lua_error_after_set_text_keeps_text() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        let result = runtime.run(&code(r#"ui.set_text("before") error("boom")"#));

        let message = result.expect_err("lua should fail").to_string();
        assert!(message.contains("boom"), "unexpected error: {message}");
        assert_eq!(pane_lines(&renderer, "pane"), vec!["before"]);
    }

    #[test]
    fn lua_ui_is_bound_to_its_own_pane() {
        let renderer = renderer_with_panes(&["left", "right"]);
        let left = LuaPaneRuntime::new("left".into(), renderer.clone()).expect("runtime");
        let right = LuaPaneRuntime::new("right".into(), renderer.clone()).expect("runtime");

        left.run(&code(r#"ui.set_text("L")"#))
            .expect("lua should run");
        right
            .run(&code(r#"ui.set_text("R")"#))
            .expect("lua should run");

        assert_eq!(pane_lines(&renderer, "left"), vec!["L"]);
        assert_eq!(pane_lines(&renderer, "right"), vec!["R"]);
    }

    #[test]
    fn lua_state_persists_between_runs() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        let tick = code(r#"count = (count or 0) + 1 ui.set_text("tick " .. count)"#);
        runtime.run(&tick).expect("lua should run");
        runtime.run(&tick).expect("lua should run");

        assert_eq!(pane_lines(&renderer, "pane"), vec!["tick 2"]);
    }

    #[test]
    fn lua_runs_script_file() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        let script_path = std::env::temp_dir().join(format!(
            "rust-tui-lua-test-{pid}.lua",
            pid = std::process::id()
        ));
        std::fs::write(&script_path, r#"ui.set_text("from " .. "script")"#)
            .expect("failure to write test script");

        let result = runtime.run(&LuaSettings {
            code: None,
            script: Some(script_path.to_string_lossy().into_owned()),
        });
        std::fs::remove_file(&script_path).expect("failure to remove test script");

        result.expect("lua should run");
        assert_eq!(pane_lines(&renderer, "pane"), vec!["from script"]);
    }

    #[test]
    fn lua_missing_script_file_errors() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        let result = runtime.run(&LuaSettings {
            code: None,
            script: Some("/definitely/not/a/real/script.lua".into()),
        });

        assert!(result.is_err());
        assert_eq!(pane_lines(&renderer, "pane"), vec!["initial"]);
    }

    #[tokio::test(start_paused = true)]
    async fn lua_without_refresh_runs_once() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        let tick = code(r#"count = (count or 0) + 1 ui.set_text("tick " .. count)"#);
        runtime.run_on_schedule(&tick, None).await;

        assert_eq!(pane_lines(&renderer, "pane"), vec!["tick 1"]);
    }

    #[tokio::test(start_paused = true)]
    async fn lua_refresh_reuses_runtime_on_each_tick() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        let tick = code(r#"count = (count or 0) + 1 ui.set_text("tick " .. count)"#);
        let task = tokio::spawn(async move { runtime.run_on_schedule(&tick, Some(1000)).await });

        tokio::time::sleep(Duration::from_millis(2500)).await;
        task.abort();

        assert_eq!(pane_lines(&renderer, "pane"), vec!["tick 3"]);
    }

    #[tokio::test(start_paused = true)]
    async fn lua_refresh_continues_after_error() {
        let renderer = renderer_with_panes(&["pane"]);
        let runtime = LuaPaneRuntime::new("pane".into(), renderer.clone()).expect("runtime");

        let flaky = code(
            r#"
            count = (count or 0) + 1
            if count == 1 then error("first run fails") end
            ui.set_text("recovered on " .. count)
            "#,
        );
        let task = tokio::spawn(async move { runtime.run_on_schedule(&flaky, Some(1000)).await });

        tokio::time::sleep(Duration::from_millis(1500)).await;
        task.abort();

        assert_eq!(pane_lines(&renderer, "pane"), vec!["recovered on 2"]);
    }
}
