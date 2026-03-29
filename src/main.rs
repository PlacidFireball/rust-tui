pub(crate) mod config;
pub(crate) mod renderer;
pub(crate) mod sequencer;
pub(crate) mod ui;

use crate::ui::TerminalUI;

fn main() {
    let mut ui = match TerminalUI::from_config("double-pane-config.json") {
        Ok(ui) => ui,
        Err(e) => panic!("{e:?}"),
    };

    ui.running_loop();
}
