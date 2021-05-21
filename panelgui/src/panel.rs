use std::any::Any;

use bindings::Windows::Foundation::Numerics::Vector2;
use bindings::Windows::UI::Composition::ContainerVisual;
use winit::event::{ElementState, KeyboardInput, MouseButton};

use crate::globals::{compositor, get_next_id, winrt_error};

pub struct PanelEvent {
    pub panel_id: usize,
    pub data: Option<Box<dyn Any>>,
}
pub trait Panel {
    fn id(&self) -> usize;
    fn visual(&self) -> ContainerVisual;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any>;
    fn on_init(&mut self) -> windows::Result<()>;
    fn on_resize(&mut self, size: &Vector2) -> windows::Result<()>;
    fn on_idle(&mut self) -> windows::Result<()>;
    fn on_mouse_move(&mut self, position: &Vector2) -> windows::Result<()>;
    fn on_mouse_input(&mut self, button: MouseButton, state: ElementState)
        -> windows::Result<bool>;
    fn on_keyboard_input(&mut self, input: KeyboardInput) -> windows::Result<bool>;
    fn on_panel_event(&mut self, panel_event: &mut PanelEvent) -> windows::Result<()>;
}

pub trait Handle {
    fn id(&self) -> usize;
}

pub trait PanelHandle<PanelType: Any, PanelEventType: Any = ()>: Handle {
    fn at<'a>(&self, root_panel: &'a mut dyn Panel) -> windows::Result<&'a mut PanelType> {
        if let Some(p) = root_panel.find_panel(self.id()) {
            if let Some(p) = p.downcast_mut::<PanelType>() {
                return Ok(p);
            }
        }
        Err(winrt_error("Can't find panel")())
    }
    fn extract_event(&self, panel_event: &mut PanelEvent) -> Option<PanelEventType> {
        if panel_event.panel_id == self.id() {
            if let Some(data) = panel_event.data.take() {
                match data.downcast::<PanelEventType>() {
                    Ok(e) => Some(*e),
                    Err(data) => {
                        panel_event.data = Some(data);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct EmptyPanel {
    id: usize,
    visual: ContainerVisual,
}

impl EmptyPanel {
    pub fn new() -> windows::Result<Self> {
        let visual = compositor().CreateContainerVisual()?;
        let id = get_next_id();
        Ok(Self { id, visual })
    }
}

impl Panel for EmptyPanel {
    fn id(&self) -> usize {
        self.id
    }
    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id() {
            Some(self.as_any_mut())
        } else {
            None
        }
    }

    fn on_init(&mut self) -> windows::Result<()> {
        Ok(())
    }

    fn on_resize(&mut self, _size: &Vector2) -> windows::Result<()> {
        Ok(())
    }

    fn on_idle(&mut self) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_move(&mut self, _position: &Vector2) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: MouseButton,
        _state: ElementState,
    ) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(&mut self, _input: KeyboardInput) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_panel_event(&mut self, _panel_event: &mut PanelEvent) -> windows::Result<()> {
        Ok(())
    }
}
