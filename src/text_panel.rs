use std::borrow::Cow;

use bindings::windows::ui::composition::ContainerVisual;

use crate::game_window::{GameWindow, Panel};

#[derive(Clone)]
pub struct TextPanelHandle {
    id: usize,
}

pub struct TextPanel {
    id: usize,
    visual: ContainerVisual,
    text: Cow<'static, str>,
}

impl TextPanel {
    pub fn new(game_window: &mut GameWindow) -> winrt::Result<Self> {
        let visual = game_window.compositor().create_container_visual()?;
        Ok(Self {
            id: game_window.get_next_id(),
            visual,
            text: "".into(),
        })
    }
    pub fn set_text<S: Into<Cow<'static, str>>>(&mut self, text: S) {
        self.text = text.into();
    }
}

impl Panel for TextPanel {
    fn id(&self) -> usize {
        self.id
    }
    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }
}
