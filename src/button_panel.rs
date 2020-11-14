use std::any::Any;

use bindings::windows::ui::composition::ContainerVisual;
use winit::event::{ElementState, MouseButton};

use crate::{
    control::{Control, ControlHandle},
    game_window::{
        winrt_error, GameWindow, Handle, Panel, PanelEvent, PanelEventProxy, PanelHandle,
    },
};

#[derive(PartialEq)]
pub enum ButtonPanelEvent {
    Pressed,
}
pub struct ButtonPanel {
    id: usize,
    subpabel: Option<Box<dyn Control>>,
    visual: ContainerVisual,
}

#[derive(Copy, Clone)]
pub struct ButtonPanelHandle {
    id: usize,
}

impl Handle for ButtonPanelHandle {
    fn id(&self) -> usize {
        self.id
    }
}

impl PanelHandle<ButtonPanel, ButtonPanelEvent> for ButtonPanelHandle {}

impl ControlHandle for ButtonPanelHandle {
    fn as_control<'a>(&self, root_panel: &'a mut dyn Panel) -> winrt::Result<&'a mut dyn Control> {
        Ok(self.at(root_panel)?)
    }
}

impl ButtonPanel {
    pub fn new(game_window: &mut GameWindow) -> winrt::Result<Self> {
        let compositor = game_window.compositor().clone();
        let visual = compositor.create_container_visual()?;
        Ok(Self {
            id: game_window.get_next_id(),
            subpabel: None,
            visual,
        })
    }
    pub fn handle(&self) -> ButtonPanelHandle {
        ButtonPanelHandle { id: self.id }
    }
    pub fn add_subpanel<P: Control + 'static>(&mut self, panel: P) -> winrt::Result<()> {
        self.visual
            .children()?
            .insert_at_top(panel.visual().clone())?;
        self.subpabel = Some(Box::new(panel));
        Ok(())
    }
    pub fn subpanel(&mut self) -> winrt::Result<&mut (dyn Control + 'static)> {
        self.subpabel
            .as_deref_mut()
            .ok_or(winrt_error("no panel in ButtonPanel"))
    }
}

impl Panel for ButtonPanel {
    fn id(&self) -> usize {
        self.id
    }
    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }

    fn on_resize(&mut self) -> winrt::Result<()> {
        self.visual.set_size(self.visual.parent()?.size()?)?;
        self.subpanel()?.on_resize()?;
        Ok(())
    }

    fn on_idle(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.subpanel()?.on_idle(proxy)
    }

    fn on_mouse_input(
        &mut self,
        _position: bindings::windows::foundation::numerics::Vector2,
        button: MouseButton,
        state: ElementState,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        if button == MouseButton::Left && state == ElementState::Pressed {
            proxy.send_panel_event(self.id, ButtonPanelEvent::Pressed)?;
        }
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id() {
            return Some(self.as_any_mut());
        } else if let Some(p) = self.subpabel.as_mut() {
            p.get_panel(id)
        } else {
            None
        }
    }
}

impl Control for ButtonPanel {
    fn on_enable(&mut self, enable: bool) -> winrt::Result<()> {
        self.subpanel()?.on_enable(enable)
    }

    fn on_set_focus(&mut self) -> winrt::Result<()> {
        todo!()
    }

    fn as_panel(&self) -> &dyn Panel {
        self
    }
}
