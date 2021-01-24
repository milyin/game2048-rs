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
#[builder(setter(into))]
pub struct MessageBoxParams {
    #[builder(default = "{MessageBoxButton::Ok.into()}")]
    button_flags: BitFlags<MessageBoxButton>,
    #[builder(default = "{\"\".into()}")]
    message: Cow<'static, str>,
}

impl MessageBoxParamsBuilder {
    pub fn create(&self) -> windows::Result<MessageBoxPanel> {
        match self.build() {
            Ok(settings) => Ok(MessageBoxPanel::new(settings)?),
            Err(e) => Err(winrt_error(e)()),
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
    pub fn new(params: MessageBoxParams) -> windows::Result<Self> {
        let id = globals().get_next_id();
        let background = BackgroundParamsBuilder::default()
            .color(Colors::wheat()?)
            .round_corners(true)
            .create()?;
        let message_panel = TextParamsBuilder::default()
            .text(params.message)
            .font_scale(3.)
            .create()?;
        let button_yes = ButtonParamsBuilder::default().text("Yes")?.create()?;
        let button_no = ButtonParamsBuilder::default().text("No")?.create()?;
        let button_ok = ButtonParamsBuilder::default().text("OK")?.create()?;
        let button_cancel = ButtonParamsBuilder::default().text("Cancel")?.create()?;
        let handle_yes = button_yes.handle();
        let handle_no = button_no.handle();
        let handle_ok = button_ok.handle();
        let handle_cancel = button_cancel.handle();
        let mut ribbon_buttons =
            RibbonParamsBuilder::default().orientation(RibbonOrientation::Horizontal);
        let mut control_manager = ControlManager::new();
        if params.button_flags.contains(MessageBoxButton::Yes) {
            ribbon_buttons = ribbon_buttons.add_panel(button_yes)?;
            control_manager.add_control(handle_yes.clone());
        }
        if params.button_flags.contains(MessageBoxButton::No) {
            ribbon_buttons = ribbon_buttons.add_panel(button_no)?;
            control_manager.add_control(handle_no.clone());
        }
        if params.button_flags.contains(MessageBoxButton::Ok) {
            ribbon_buttons = ribbon_buttons.add_panel(button_ok)?;
            control_manager.add_control(handle_ok.clone());
        }
        if params.button_flags.contains(MessageBoxButton::Cancel) {
            ribbon_buttons = ribbon_buttons.add_panel(button_cancel)?;
            control_manager.add_control(handle_cancel.clone());
        }
        let ribbon = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Vertical)
            .add_panel_with_ratio(message_panel, 1.5)?
            .add_panel(ribbon_buttons.create()?)?
            .create()?;
        let root_panel = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Stack)
            .add_panel(background)?
            .add_panel(ribbon)?
            .create()?;

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

    fn on_init(&mut self, proxy: &crate::main_window::PanelEventProxy) -> windows::Result<()> {
        self.root_panel.on_init(proxy)
    }

    fn on_resize(
        &mut self,
        size: &bindings::windows::foundation::numerics::Vector2,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> windows::Result<()> {
        self.visual().set_size(size.clone())?;
        self.root_panel.on_resize(size, proxy)
    }

    fn on_idle(&mut self, proxy: &crate::main_window::PanelEventProxy) -> windows::Result<()> {
        self.root_panel.on_idle(proxy)
    }

    fn on_mouse_move(
        &mut self,
        position: &bindings::windows::foundation::numerics::Vector2,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> windows::Result<()> {
        self.root_panel.on_mouse_move(position, proxy)
    }

    fn on_mouse_input(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> windows::Result<bool> {
        self.root_panel.on_mouse_input(button, state, proxy)
    }

    fn on_keyboard_input(
        &mut self,
        input: winit::event::KeyboardInput,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> windows::Result<bool> {
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
    ) -> windows::Result<()> {
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
