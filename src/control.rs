use winit::event::Event;

use crate::game_window::{Handle, Panel, PanelEvent, PanelEventProxy, PanelManager};

pub trait Control: Panel {
    fn on_enable(&mut self, enable: bool) -> winrt::Result<()>;
    fn on_set_focus(&mut self) -> winrt::Result<()>;
    fn as_panel(&self) -> &dyn Panel;
}

pub trait ControlHandle: Handle {
    fn as_control<'a>(&self, root_panel: &'a mut dyn Panel) -> Option<&'a mut dyn Control>;
}

pub struct ControlManager {
    controls: Vec<Box<dyn ControlHandle>>,
}

pub struct ControlManagerWith<'a, 'b> {
    control_manager: &'a mut ControlManager,
    panel_manager: &'b mut PanelManager,
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
    pub fn with<'a, 'b>(
        &'a mut self,
        panel_manager: &'b mut PanelManager,
    ) -> ControlManagerWith<'a, 'b> {
        ControlManagerWith {
            control_manager: self,
            panel_manager,
        }
    }
    pub fn process_event(
        &mut self,
        evt: &Event<PanelEvent>,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }
}

impl<'a, 'b> ControlManagerWith<'a, 'b> {
    pub fn enable<T: ControlHandle>(
        &mut self,
        control_handle: T,
        enable: bool,
    ) -> winrt::Result<()> {
        for h in &self.control_manager.controls {
            if h.id() == control_handle.id() {
                if let Some(c) = h.as_control(self.panel_manager.root_panel()?) {
                    c.on_enable(enable)?
                }
            }
        }
        Ok(())
    }
}
