use std::sync::{Arc, Mutex};

use crate::renderer::TerminalRenderer;

#[derive(Clone)]
pub struct PaneUi {
    pane_id: String,
    renderer: Arc<Mutex<TerminalRenderer>>,
}

impl PaneUi {
    pub fn new(pane_id: String, renderer: Arc<Mutex<TerminalRenderer>>) -> Self {
        PaneUi { pane_id, renderer }
    }

    pub fn set_text(&self, text: String) {
        let mut renderer = self.renderer.lock().expect("failure to get renderer lock");
        renderer.update_pane(self.pane_id.clone(), |mut pane| {
            pane.set_text(text.clone());
            pane
        });
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Arc, Mutex};

    use crate::renderer::{TerminalPane, TerminalRenderer};
    use crate::sequencer::EscapeSequencer;

    pub(crate) fn renderer_with_panes(pane_ids: &[&str]) -> Arc<Mutex<TerminalRenderer>> {
        let mut renderer = TerminalRenderer::new(EscapeSequencer::new(80, 24));
        for pane_id in pane_ids {
            renderer.add_pane(TerminalPane::new(
                0,
                0,
                40,
                10,
                pane_id.to_string(),
                None,
                Some("initial".into()),
                None,
            ));
        }
        Arc::new(Mutex::new(renderer))
    }

    pub(crate) fn pane_lines(
        renderer: &Arc<Mutex<TerminalRenderer>>,
        pane_id: &str,
    ) -> Vec<String> {
        let renderer = renderer.lock().expect("failure to get renderer lock");
        let pane = renderer
            .get_panes()
            .iter()
            .map(|pane| pane.lock().expect("failure to get lock on pane"))
            .find(|pane| pane.id == pane_id)
            .expect("pane should exist")
            .clone();
        pane.lines().to_vec()
    }
}
