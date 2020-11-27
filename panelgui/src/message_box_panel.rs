use std::borrow::Cow;

use enumflags2::BitFlags;

use bindings::windows::ui::{composition::ContainerVisual, Colors};

use crate::{
    background_panel::BackgroundPanelBuilder,
    button_panel::{ButtonPanelBuilder, ButtonPanelEvent, ButtonPanelHandle},
    control::ControlManager,
    main_window::globals,
    main_window::{Handle, Panel, PanelHandle},
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

#[derive(Copy, Clone, BitFlags, PartialEq)]
pub enum MessageBoxButton {
    Ok = 0b1,
    Cancel = 0b10,
    Yes = 0b100,
    No = 0b1000,
}

impl PanelHandle<MessageBoxPanel, MessageBoxButton> for MessageBoxPanelHandle {}

pub struct MessageBoxPanel {
    id: usize,
    visual: ContainerVisual,
    root_panel: RibbonPanel,
    control_manager: ControlManager,
    handle_yes: ButtonPanelHandle,
    handle_no: ButtonPanelHandle,
    handle_ok: ButtonPanelHandle,
    handle_cancel: ButtonPanelHandle,
}

impl MessageBoxPanel {
    pub fn new<S: Into<Cow<'static, str>>>(
        message: S,
        button_flags: BitFlags<MessageBoxButton>,
    ) -> winrt::Result<Self> {
        let visual = globals().compositor().create_container_visual()?;
        let mut root_panel = RibbonPanel::new(RibbonOrientation::Stack)?;
        visual.children()?.insert_at_top(root_panel.visual())?;
        let mut background_panel = BackgroundPanelBuilder::default()
            .color(Colors::wheat()?)
            .round_corners(true)
            .build()?;
        background_panel.set_round_corners(true)?;
        background_panel.set_color(Colors::wheat()?)?;
        root_panel.push_panel(background_panel, 1.0)?;
        let mut message_panel = TextPanel::new()?;
        message_panel.set_text(message)?;
        let mut button_yes = ButtonPanelBuilder::default().build()?;
        let mut button_no = ButtonPanelBuilder::default().build()?;
        let mut button_ok = ButtonPanelBuilder::default().build()?;
        let mut button_cancel = ButtonPanelBuilder::default().build()?;
        let mut text_yes = TextPanel::new()?;
        let mut text_no = TextPanel::new()?;
        let mut text_ok = TextPanel::new()?;
        let mut text_cancel = TextPanel::new()?;
        let handle_yes = button_yes.handle();
        let handle_no = button_no.handle();
        let handle_ok = button_ok.handle();
        let handle_cancel = button_cancel.handle();
        text_yes.set_text("Yes")?;
        text_no.set_text("No")?;
        text_ok.set_text("OK")?;
        text_cancel.set_text("Cancel")?;
        button_yes.set_panel(text_yes)?;
        button_no.set_panel(text_no)?;
        button_ok.set_panel(text_ok)?;
        button_cancel.set_panel(text_cancel)?;
        let mut ribbon = RibbonPanel::new(RibbonOrientation::Vertical)?;
        ribbon.push_panel(message_panel, 1.0)?;
        let mut ribbon_buttons = RibbonPanel::new(RibbonOrientation::Horizontal)?;
        let mut control_manager = ControlManager::new();
        if button_flags.contains(MessageBoxButton::Yes) {
            ribbon_buttons.push_panel(button_yes, 1.0)?;
            control_manager.add_control(handle_yes.clone());
        }
        if button_flags.contains(MessageBoxButton::No) {
            ribbon_buttons.push_panel(button_no, 1.0)?;
            control_manager.add_control(handle_no.clone());
        }
        if button_flags.contains(MessageBoxButton::Ok) {
            ribbon_buttons.push_panel(button_ok, 1.0)?;
            control_manager.add_control(handle_ok.clone());
        }
        if button_flags.contains(MessageBoxButton::Cancel) {
            ribbon_buttons.push_panel(button_cancel, 1.0)?;
            control_manager.add_control(handle_cancel.clone());
        }
        ribbon.push_panel(ribbon_buttons, 1.0)?;
        root_panel.push_panel(ribbon, 1.0)?;

        Ok(Self {
            id: globals().get_next_id(),
            visual,
            root_panel,
            control_manager,
            handle_yes,
            handle_no,
            handle_ok,
            handle_cancel,
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
        Ok(self.root_panel.on_keyboard_input(input, proxy)?
            || self
                .control_manager
                .process_keyboard_input(input, &mut self.root_panel, proxy)?)
    }

    fn on_panel_event(
        &mut self,
        panel_event: &mut crate::main_window::PanelEvent,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<()> {
        self.root_panel.on_panel_event(panel_event, proxy)?;
        if self.handle_yes.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed) {
            proxy.send_panel_event(self.id, MessageBoxButton::Yes)?;
        }
        if self.handle_no.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed) {
            proxy.send_panel_event(self.id, MessageBoxButton::No)?;
        }
        if self.handle_cancel.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed) {
            proxy.send_panel_event(self.id, MessageBoxButton::Cancel)?;
        }
        if self.handle_ok.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed) {
            proxy.send_panel_event(self.id, MessageBoxButton::Ok)?;
        } else {
            self.control_manager
                .process_panel_event(panel_event, &mut self.root_panel, proxy)?;
        }
        Ok(())
    }
}
