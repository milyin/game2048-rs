use std::borrow::Cow;

use enumflags2::BitFlags;

use bindings::windows::ui::composition::ContainerVisual;

use crate::{
    background_panel::BackgroundPanel,
    button_panel::ButtonPanel,
    main_window::{Handle, Panel, PanelGlobals, PanelHandle},
    ribbon_panel::RibbonOrientation,
    ribbon_panel::RibbonPanel,
    text_panel::TextPanel,
};

pub struct MessageBoxPanelHandle(usize);

impl Handle for MessageBoxPanelHandle {
    fn id(&self) -> usize {
        self.0
    }
}

impl PanelHandle<MessageBoxPanel> for MessageBoxPanelHandle {}

#[derive(Copy, Clone, BitFlags)]
pub enum MessageBoxButton {
    Ok = 0b1,
    Cancel = 0b10,
    Yes = 0b100,
    No = 0b1000,
}

pub struct MessageBoxPanel {
    id: usize,
    visual: ContainerVisual,
    root_panel: RibbonPanel,
}

impl MessageBoxPanel {
    pub fn new<S: Into<Cow<'static, str>>>(
        globals: &PanelGlobals,
        message: S,
        button_flags: BitFlags<MessageBoxButton>,
    ) -> winrt::Result<Self> {
        let globals = globals.clone();
        let visual = globals.compositor().create_container_visual()?;
        let mut root_panel = RibbonPanel::new(&globals, RibbonOrientation::Stack)?;
        visual.children()?.insert_at_top(root_panel.visual())?;
        let background_panel = BackgroundPanel::new(&globals)?;
        root_panel.push_panel(background_panel, 1.0)?;
        let mut message_panel = TextPanel::new(&globals)?;
        message_panel.set_text(message)?;
        let mut button_yes = ButtonPanel::new(&globals)?;
        let mut button_no = ButtonPanel::new(&globals)?;
        let mut button_ok = ButtonPanel::new(&globals)?;
        let mut button_cancel = ButtonPanel::new(&globals)?;
        let mut text_yes = TextPanel::new(&globals)?;
        let mut text_no = TextPanel::new(&globals)?;
        let mut text_ok = TextPanel::new(&globals)?;
        let mut text_cancel = TextPanel::new(&globals)?;
        text_yes.set_text("Yes")?;
        text_no.set_text("No")?;
        text_ok.set_text("OK")?;
        text_cancel.set_text("Cancel")?;
        button_yes.add_panel(text_yes)?;
        button_no.add_panel(text_no)?;
        button_ok.add_panel(text_ok)?;
        button_cancel.add_panel(text_cancel)?;
        let mut ribbon = RibbonPanel::new(&globals, RibbonOrientation::Vertical)?;
        ribbon.push_panel(message_panel, 1.0)?;
        let mut ribbon_buttons = RibbonPanel::new(&globals, RibbonOrientation::Horizontal)?;
        if button_flags.contains(MessageBoxButton::Yes) {
            ribbon_buttons.push_panel(button_yes, 1.0)?;
        }
        if button_flags.contains(MessageBoxButton::No) {
            ribbon_buttons.push_panel(button_no, 1.0)?;
        }
        if button_flags.contains(MessageBoxButton::Ok) {
            ribbon_buttons.push_panel(button_ok, 1.0)?;
        }
        if button_flags.contains(MessageBoxButton::Cancel) {
            ribbon_buttons.push_panel(button_cancel, 1.0)?;
        }
        ribbon.push_panel(ribbon_buttons, 1.0)?;
        root_panel.push_panel(ribbon, 1.0)?;
        Ok(Self {
            id: globals.get_next_id(),
            visual,
            root_panel,
        })
    }
    pub fn handle(&self) -> MessageBoxPanelHandle {
        MessageBoxPanelHandle(self.id)
    }
}

impl Panel for MessageBoxPanel {
    fn id(&self) -> usize {
        self.id
    }

    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn std::any::Any> {
        if id == self.id {
            Some(self.as_any_mut())
        } else {
            self.root_panel.find_panel(id)
        }
    }

    fn on_init(&mut self, proxy: &crate::main_window::PanelEventProxy) -> winrt::Result<()> {
        self.root_panel.on_init(proxy)
    }

    fn on_resize(
        &mut self,
        size: &bindings::windows::foundation::numerics::Vector2,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<()> {
        self.visual().set_size(size.clone())?;
        self.root_panel.on_resize(size, proxy)
    }

    fn on_idle(&mut self, proxy: &crate::main_window::PanelEventProxy) -> winrt::Result<()> {
        self.root_panel.on_idle(proxy)
    }

    fn on_mouse_move(
        &mut self,
        position: &bindings::windows::foundation::numerics::Vector2,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<()> {
        self.root_panel.on_mouse_move(position, proxy)
    }

    fn on_mouse_input(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<bool> {
        self.root_panel.on_mouse_input(button, state, proxy)
    }

    fn on_keyboard_input(
        &mut self,
        input: winit::event::KeyboardInput,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<bool> {
        self.root_panel.on_keyboard_input(input, proxy)
    }

    fn on_panel_event(
        &mut self,
        panel_event: &mut crate::main_window::PanelEvent,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<()> {
        self.root_panel.on_panel_event(panel_event, proxy)
    }
}
