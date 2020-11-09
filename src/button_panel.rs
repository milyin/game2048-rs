use bindings::windows::ui::composition::ContainerVisual;
use winit::event::{ElementState, MouseButton};

use crate::game_window::{
    winrt_error, GameWindow, Panel, PanelEvent, PanelEventProxy, PanelMessage,
};

#[derive(PartialEq)]
pub enum ButtonPanelEvent {
    Pressed,
}

pub struct ButtonPanel {
    id: usize,
    panel: Option<Box<dyn Panel>>,
    visual: ContainerVisual,
}

pub struct ButtonPanelHandle {
    id: usize,
}

impl ButtonPanelHandle {
    pub fn event(&self, event: PanelEvent) -> Option<ButtonPanelEvent> {
        if event.panel_id == self.id {
            event.data.downcast::<ButtonPanelEvent>().ok().map(|e| *e)
        } else {
            None
        }
    }
}

pub struct ButtonPanelProxy<'a> {
    handle: ButtonPanelHandle,
    root_panel: &'a mut dyn Panel,
}

impl ButtonPanel {
    pub fn new(game_window: &mut GameWindow) -> winrt::Result<Self> {
        let compositor = game_window.compositor().clone();
        let visual = compositor.create_container_visual()?;
        Ok(Self {
            id: game_window.get_next_id(),
            panel: None,
            visual,
        })
    }
    pub fn handle(&self) -> ButtonPanelHandle {
        ButtonPanelHandle { id: self.id }
    }
    pub fn add_panel<P: Panel + 'static>(&mut self, panel: P) -> winrt::Result<()> {
        self.visual
            .children()?
            .insert_at_top(panel.visual().clone())?;
        self.panel = Some(Box::new(panel));
        Ok(())
    }
    pub fn panel(&mut self) -> winrt::Result<&mut (dyn Panel + 'static)> {
        self.panel
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
        self.panel()?.on_resize()?;
        Ok(())
    }

    fn on_idle(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.panel()?.on_idle(proxy)
    }

    fn translate_message(&mut self, msg: PanelMessage) -> winrt::Result<PanelMessage> {
        let msg = self.translate_message_default(msg)?;
        self.panel()?.translate_message(msg)
    }

    fn on_request(
        &mut self,
        request: Box<dyn std::any::Any>,
    ) -> winrt::Result<Box<dyn std::any::Any>> {
        Ok(Box::new(()))
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
}
