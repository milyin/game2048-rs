use std::{any::Any, borrow::Cow};

use bindings::windows::ui::composition::ContainerVisual;
use winit::event_loop::EventLoopProxy;

use crate::game_window::{GameWindow, Panel, PanelEvent, PanelEventProxy};

#[derive(Copy, Clone)]
pub struct TextPanelHandle {
    id: usize,
}

impl TextPanelHandle {
    pub fn with_proxy<'a>(&self, proxy: &'a PanelEventProxy) -> TextPanelProxy<'a> {
        TextPanelProxy {
            handle: self.clone(),
            proxy,
        }
    }
}

pub struct TextPanelProxy<'a> {
    handle: TextPanelHandle,
    proxy: &'a PanelEventProxy,
}

enum TextPanelCommand {
    SetText(Cow<'static, str>),
}

impl<'a> TextPanelProxy<'a> {
    pub fn set_text<S: Into<Cow<'static, str>>>(&self, text: S) -> winrt::Result<()> {
        self.proxy
            .send_command_to_panel(self.handle.id, TextPanelCommand::SetText(text.into()))
    }
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
    pub fn handle(&self) -> TextPanelHandle {
        TextPanelHandle { id: self.id }
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

    fn on_command(&mut self, command: Box<dyn Any>) -> winrt::Result<()> {
        if let Ok(command) = command.downcast::<TextPanelCommand>() {
            match *command {
                TextPanelCommand::SetText(text) => self.set_text(text),
            }
        }
        Ok(())
    }
}
