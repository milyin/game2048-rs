use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};

use crate::main_window::{Handle, Panel, PanelEvent, PanelEventProxy, PanelManager};

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
    pub fn process_event(
        &mut self,
        evt: &mut Event<PanelEvent>,
        panel_manager: &mut PanelManager,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        match evt {
            Event::UserEvent(ref mut e) => {
                for h in &self.controls {
                    if h.id() == e.panel_id {
                        if let Some(data) = e.data.take() {
                            match data.downcast::<ControlEvent>() {
                                Ok(control_event) => {
                                    match *control_event {
                                        ControlEvent::FocusNext => {
                                            self.focus_next(panel_manager, h.id())?;
                                        }
                                        ControlEvent::FocusPrev => {
                                            self.focus_prev(panel_manager, h.id())?;
                                        }
                                        ControlEvent::FocusSet => {
                                            self.focus_set(panel_manager, h.id())?;
                                        }
                                        ControlEvent::FocusClear => {
                                            self.focus_clear(panel_manager)?;
                                        }
                                        ControlEvent::Enable(enable) => {
                                            self.enable(panel_manager, h.id(), enable)?;
                                        }
                                    }
                                    return Ok(true);
                                }
                                Err(data) => {
                                    e.data = Some(data);
                                }
                            }
                        }
                    }
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        input,
                        is_synthetic: _,
                    },
                ..
            } => {
                // TODO: process Shift-Tab
                if input.state == ElementState::Pressed {
                    if let Some(virtual_keycode) = input.virtual_keycode {
                        if virtual_keycode == VirtualKeyCode::Tab {
                            if let Some(panel_id) = self.get_focused_panel_id(panel_manager)? {
                                self.focus_next(panel_manager, panel_id)?;
                            } else if let Some(panel_id) =
                                self.get_first_enabled_panel_id(panel_manager)?
                            {
                                self.focus_set(panel_manager, panel_id)?;
                            }
                            return Ok(true);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn get_focused_panel_id(
        &self,
        panel_manager: &mut PanelManager,
    ) -> winrt::Result<Option<usize>> {
        for h in &self.controls {
            if let Some(c) = h.as_control(panel_manager.root_panel()?) {
                if c.is_enabled()? && c.is_focused()? {
                    return Ok(Some(c.id()));
                }
            }
        }
        Ok(None)
    }

    fn get_first_enabled_panel_id(
        &self,
        panel_manager: &mut PanelManager,
    ) -> winrt::Result<Option<usize>> {
        for h in &self.controls {
            if let Some(c) = h.as_control(panel_manager.root_panel()?) {
                if c.is_enabled()? {
                    return Ok(Some(c.id()));
                }
            }
        }
        Ok(None)
    }

    fn enable(
        &self,
        panel_manager: &mut PanelManager,
        panel_id: usize,
        enable: bool,
    ) -> winrt::Result<()> {
        let mut focus_next = false;
        for h in &self.controls {
            if let Some(c) = h.as_control(panel_manager.root_panel()?) {
                if c.id() == panel_id {
                    if c.is_focused()? && !enable {
                        focus_next = true;
                    }
                    c.on_enable(enable)?;
                }
            }
        }
        if focus_next {
            self.focus_next(panel_manager, panel_id)?;
        }
        Ok(())
    }

    fn focus_clear(&self, panel_manager: &mut PanelManager) -> winrt::Result<bool> {
        for h in &self.controls {
            if let Some(c) = h.as_control(panel_manager.root_panel()?) {
                c.on_clear_focus()?;
            }
        }
        Ok(true)
    }

    fn focus_set(&self, panel_manager: &mut PanelManager, panel_id: usize) -> winrt::Result<()> {
        self.focus_clear(panel_manager)?;
        for h in &self.controls {
            if let Some(c) = h.as_control(panel_manager.root_panel()?) {
                if c.id() == panel_id {
                    return c.on_set_focus();
                }
            }
        }
        Ok(())
    }

    fn focus_next_impl<'a>(
        iter: impl Iterator<Item = &'a Box<dyn ControlHandle>> + Clone,
        panel_manager: &'a mut PanelManager,
        panel_id: usize,
    ) -> winrt::Result<()> {
        let mut found = false;
        let iter_first = iter.clone();
        for h in iter {
            if let Some(c) = h.as_control(panel_manager.root_panel()?) {
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
            if let Some(c) = h.as_control(panel_manager.root_panel()?) {
                if c.is_enabled()? {
                    return c.on_set_focus();
                }
            }
        }
        Ok(())
    }

    fn focus_next(&self, panel_manager: &mut PanelManager, panel_id: usize) -> winrt::Result<()> {
        self.focus_clear(panel_manager)?;
        Self::focus_next_impl(self.controls.iter(), panel_manager, panel_id)
    }

    fn focus_prev(&self, panel_manager: &mut PanelManager, panel_id: usize) -> winrt::Result<()> {
        self.focus_clear(panel_manager)?;
        Self::focus_next_impl(self.controls.iter().rev(), panel_manager, panel_id)
    }
}
