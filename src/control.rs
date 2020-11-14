use winit::event::Event;

use crate::game_window::{Handle, Panel, PanelEvent, PanelEventProxy};

pub trait Control: Panel {
    fn on_enable(&mut self, enable: bool) -> winrt::Result<()>;
    fn on_set_focus(&mut self) -> winrt::Result<()>;
    fn as_panel(&self) -> &dyn Panel;
}

pub trait ControlHandle: Handle {
    fn as_control<'a>(&self, root_panel: &'a mut dyn Panel) -> winrt::Result<&'a mut dyn Control>;
}

pub struct ControlManager {
    controls: Vec<Box<dyn ControlHandle>>,
}

impl ControlManager {
    pub fn new() -> Self {
        ControlManager {
            controls: Vec::new(),
        }
    }
    pub fn add_control<T: ControlHandle + 'static>(&mut self, control_handle: T) {
        self.controls.push(Box::new(control_handle));
    }
    pub fn enable<T: ControlHandle>(
        &self,
        root_panel: &mut dyn Panel,
        control_handle: &T,
        enable: bool,
    ) -> winrt::Result<()> {
        for h in &self.controls {
            if h.id() == control_handle.id() {
                h.as_control(root_panel)?.on_enable(enable)?
            }
        }
        Ok(())
    }
    pub fn process_event(
        &mut self,
        evt: &Event<PanelEvent>,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }
}
