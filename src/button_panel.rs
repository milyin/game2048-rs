use bindings::windows::ui::composition::ContainerVisual;

use crate::game_window::{winrt_error, GameWindow, Panel, PanelEventProxy, PanelMessage};

pub struct ButtonPanel {
    id: usize,
    panel: Option<Box<dyn Panel>>,
    visual: ContainerVisual,
}

pub struct ButtonPanelHandle {
    id: usize,
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
        let msg = self.call_on_request(msg)?;
        self.panel()?.translate_message(msg)
    }
}
