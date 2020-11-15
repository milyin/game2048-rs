use crate::game_window::{Handle, PanelHandle};

pub struct BackgroundPanel {
    id: usize,
}

impl Handle for BackgroundPanel {
    fn id(&self) -> usize {
        self.id
    }
}

impl PanelHandle<BackgroundPanel> for BackgroundPanel {}

impl BackgroundPanel {
    pub fn new() -> winrt::Result<Self> {
        Self {
            
        }
    }
}