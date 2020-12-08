use std::borrow::Cow;

use enumflags2::BitFlags;

use bindings::windows::ui::{composition::ContainerVisual, Colors};
use winit::event::VirtualKeyCode;

use crate::{
    background_panel::BackgroundParamsBuilder,
    button_panel::{ButtonPanelEvent, ButtonPanelHandle, ButtonParamsBuilder},
    control::ControlManager,
    main_window::globals,
    main_window::winrt_error,
    main_window::{Handle, Panel, PanelHandle},
    ribbon_panel::RibbonOrientation,
    ribbon_panel::RibbonPanel,
    ribbon_panel::RibbonParamsBuilder,
    text_panel::TextParamsBuilder,
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

#[derive(Builder)]
#[builder(build_fn(private, name = "build_default"), setter(into))]
pub struct MessageBoxParams {
    #[builder(default = "MessageBoxButton::Ok.into()")]
    button_flags: BitFlags<MessageBoxButton>,
    #[builder(default = "\"\".into()")]
    message: Cow<'static, str>,
}

impl MessageBoxParamsBuilder {
    pub fn build(&self) -> winrt::Result<MessageBoxPanel> {
        match self.build_default() {
            Ok(settings) => Ok(MessageBoxPanel::new(settings)?),
            Err(e) => Err(winrt_error(e)),
        }
    }
}

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
    pub fn new(params: MessageBoxParams) -> winrt::Result<Self> {
        let id = globals().get_next_id();
        let mut root_panel = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Stack)
            .build()?;
        let background_panel = BackgroundParamsBuilder::default()
            .color(Colors::wheat()?)
            .round_corners(true)
            .build()?;
        root_panel.push_panel(background_panel, 1.0)?;
        let message_panel = TextParamsBuilder::default().text(params.message).build()?;
        let mut button_yes = ButtonParamsBuilder::default().build()?;
        let mut button_no = ButtonParamsBuilder::default().build()?;
        let mut button_ok = ButtonParamsBuilder::default().build()?;
        let mut button_cancel = ButtonParamsBuilder::default().build()?;
        let text_yes = TextParamsBuilder::default().text("Yes").build()?;
        let text_no = TextParamsBuilder::default().text("No").build()?;
        let text_ok = TextParamsBuilder::default().text("OK").build()?;
        let text_cancel = TextParamsBuilder::default().text("Cancel").build()?;
        let handle_yes = button_yes.handle();
        let handle_no = button_no.handle();
        let handle_ok = button_ok.handle();
        let handle_cancel = button_cancel.handle();
        button_yes.set_panel(text_yes)?;
        button_no.set_panel(text_no)?;
        button_ok.set_panel(text_ok)?;
        button_cancel.set_panel(text_cancel)?;
        let mut ribbon = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Vertical)
            .build()?;
        ribbon.push_panel(message_panel, 1.0)?;
        let mut ribbon_buttons = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Horizontal)
            .build()?;
        let mut control_manager = ControlManager::new();
        if params.button_flags.contains(MessageBoxButton::Yes) {
            ribbon_buttons.push_panel(button_yes, 1.0)?;
            control_manager.add_control(handle_yes.clone());
        }
        if params.button_flags.contains(MessageBoxButton::No) {
            ribbon_buttons.push_panel(button_no, 1.0)?;
            control_manager.add_control(handle_no.clone());
        }
        if params.button_flags.contains(MessageBoxButton::Ok) {
            ribbon_buttons.push_panel(button_ok, 1.0)?;
            control_manager.add_control(handle_ok.clone());
        }
        if params.button_flags.contains(MessageBoxButton::Cancel) {
            ribbon_buttons.push_panel(button_cancel, 1.0)?;
            control_manager.add_control(handle_cancel.clone());
        }
        ribbon.push_panel(ribbon_buttons, 1.0)?;
        root_panel.push_panel(ribbon, 1.0)?;

        let visual = globals().compositor().create_container_visual()?;
        visual.children()?.insert_at_top(root_panel.visual())?;
        Ok(Self {
            id,
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
        if let Some(key) = input.virtual_keycode {
            if key == VirtualKeyCode::Escape {
                proxy.send_panel_event(self.id, MessageBoxButton::Cancel)?;
                return Ok(true);
            }
        }
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
            let _ = self.control_manager.process_panel_event(
                panel_event,
                &mut self.root_panel,
                proxy,
            )?;
        }
        Ok(())
    }
}
