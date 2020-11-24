use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

use crate::main_window::{Handle, Panel, PanelEvent, PanelEventProxy};

pub enum ControlEvent {
    Enable(bool),
    FocusNext,
    FocusPrev,
    FocusSet,
    FocusClear,
}

pub trait Control: Panel {
    fn is_enabled(&self) -> winrt::Result<bool> {
        Ok(true)
    }
    fn is_focused(&self) -> winrt::Result<bool> {
        Ok(false)
    }
    fn on_enable(&mut self, _enable: bool) -> winrt::Result<()> {
        Ok(())
    }
    fn on_set_focus(&mut self) -> winrt::Result<()> {
        Ok(())
    }
    fn on_clear_focus(&mut self) -> winrt::Result<()> {
        Ok(())
    }
    fn as_panel(&self) -> &dyn Panel;
    fn set_focus_to_next(&self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        proxy.send_panel_event(self.id(), ControlEvent::FocusNext)
    }
    fn set_focus_to_prev(&self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        proxy.send_panel_event(self.id(), ControlEvent::FocusPrev)
    }
    fn set_focus(&self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        proxy.send_panel_event(self.id(), ControlEvent::FocusSet)
    }
    fn clear_focus(&self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        proxy.send_panel_event(self.id(), ControlEvent::FocusClear)
    }
    fn enable(&self, proxy: &PanelEventProxy, enable: bool) -> winrt::Result<()> {
        proxy.send_panel_event(self.id(), ControlEvent::Enable(enable))
    }
}

pub trait ControlHandle: Handle {
    fn as_control<'a>(&self, root_panel: &'a mut dyn Panel) -> Option<&'a mut dyn Control>;
}

type ControlHandles = Vec<Box<dyn ControlHandle>>;

pub struct ControlManager {
    controls: ControlHandles,
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

    pub fn process_panel_event(
        &mut self,
        panel_event: &mut PanelEvent,
        root_panel: &mut dyn Panel,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        for h in &self.controls {
            if h.id() == panel_event.panel_id {
                if let Some(data) = panel_event.data.take() {
                    match data.downcast::<ControlEvent>() {
                        Ok(control_event) => {
                            match *control_event {
                                ControlEvent::FocusNext => {
                                    self.focus_next(root_panel, h.id())?;
                                }
                                ControlEvent::FocusPrev => {
                                    self.focus_prev(root_panel, h.id())?;
                                }
                                ControlEvent::FocusSet => {
                                    self.focus_set(root_panel, h.id())?;
                                }
                                ControlEvent::FocusClear => {
                                    self.focus_clear(root_panel)?;
                                }
                                ControlEvent::Enable(enable) => {
                                    self.enable(root_panel, h.id(), enable)?;
                                }
                            }
                            return Ok(true);
                        }
                        Err(data) => {
                            panel_event.data = Some(data);
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    pub fn process_keyboard_input(
        &mut self,
        input: KeyboardInput,
        root_panel: &mut dyn Panel,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        // TODO: process Shift-Tab
        if input.state == ElementState::Pressed {
            if let Some(virtual_keycode) = input.virtual_keycode {
                if virtual_keycode == VirtualKeyCode::Tab {
                    if let Some(panel_id) = self.get_focused_panel_id(root_panel)? {
                        self.focus_next(root_panel, panel_id)?;
                    } else if let Some(panel_id) = self.get_first_enabled_panel_id(root_panel)? {
                        self.focus_set(root_panel, panel_id)?;
                    }
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn get_focused_panel_id(&self, root_panel: &mut dyn Panel) -> winrt::Result<Option<usize>> {
        for h in &self.controls {
            if let Some(c) = h.as_control(root_panel) {
                if c.is_enabled()? && c.is_focused()? {
                    return Ok(Some(c.id()));
                }
            }
        }
        Ok(None)
    }

    fn get_first_enabled_panel_id(
        &self,
        root_panel: &mut dyn Panel,
    ) -> winrt::Result<Option<usize>> {
        for h in &self.controls {
            if let Some(c) = h.as_control(root_panel) {
                if c.is_enabled()? {
                    return Ok(Some(c.id()));
                }
            }
        }
        Ok(None)
    }

    fn enable(
        &self,
        root_panel: &mut dyn Panel,
        panel_id: usize,
        enable: bool,
    ) -> winrt::Result<()> {
        let mut focus_next = false;
        for h in &self.controls {
            if let Some(c) = h.as_control(root_panel) {
                if c.id() == panel_id {
                    if c.is_focused()? && !enable {
                        focus_next = true;
                    }
                    c.on_enable(enable)?;
                }
            }
        }
        if focus_next {
            self.focus_next(root_panel, panel_id)?;
        }
        Ok(())
    }

    fn focus_clear(&self, root_panel: &mut dyn Panel) -> winrt::Result<()> {
        for h in &self.controls {
            if let Some(c) = h.as_control(root_panel) {
                c.on_clear_focus()?;
            }
        }
        Ok(())
    }

    fn focus_set(&self, root_panel: &mut dyn Panel, panel_id: usize) -> winrt::Result<()> {
        self.focus_clear(root_panel)?;
        for h in &self.controls {
            if let Some(c) = h.as_control(root_panel) {
                if c.id() == panel_id {
                    return c.on_set_focus();
                }
            }
        }
        Ok(())
    }

    fn focus_next_impl<'a>(
        iter: impl Iterator<Item = &'a Box<dyn ControlHandle>> + Clone,
        root_panel: &'a mut dyn Panel,
        panel_id: usize,
    ) -> winrt::Result<()> {
        let mut found = false;
        let iter_first = iter.clone();
        for h in iter {
            if let Some(c) = h.as_control(root_panel) {
                if found {
                    if c.is_enabled()? {
                        return c.on_set_focus();
                    }
                } else if c.id() == panel_id {
                    found = true;
                }
            }
        }
        for h in iter_first {
            if let Some(c) = h.as_control(root_panel) {
                if c.is_enabled()? {
                    return c.on_set_focus();
                }
            }
        }
        Ok(())
    }

    fn focus_next(&self, root_panel: &mut dyn Panel, panel_id: usize) -> winrt::Result<()> {
        self.focus_clear(root_panel)?;
        Self::focus_next_impl(self.controls.iter(), root_panel, panel_id)
    }

    fn focus_prev(&self, root_panel: &mut dyn Panel, panel_id: usize) -> winrt::Result<()> {
        self.focus_clear(root_panel)?;
        Self::focus_next_impl(self.controls.iter().rev(), root_panel, panel_id)
    }
}
