pub(crate) mod config;
pub(crate) mod lua;
pub(crate) mod pane_ui;
pub(crate) mod python;
pub(crate) mod renderer;
pub(crate) mod sequencer;
pub(crate) mod ui;
pub(crate) mod util;

use crate::ui::TerminalUI;

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() {
    let mut ui = match TerminalUI::from_config("double-pane-config.json") {
        Ok(ui) => ui,
        Err(e) => panic!("{e:?}"),
    };

    match ui.running_loop().await {
        Ok(_) => {}
        Err(e) => panic!("{e:?}"),
    };
}
