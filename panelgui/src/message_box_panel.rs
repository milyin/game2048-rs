use std::borrow::Cow;

use enumflags2::BitFlags;

use bindings::windows::ui::{composition::ContainerVisual, Colors};
use winit::event::VirtualKeyCode;

use crate::{
    background_panel::BackgroundPanelBuilder,
    button_panel::{ButtonPanelBuilder, ButtonPanelEvent, ButtonPanelHandle},
    control::ControlManager,
    main_window::globals,
    main_window::winrt_error,
    main_window::{Handle, Panel, PanelHandle},
    ribbon_panel::RibbonOrientation,
    ribbon_panel::RibbonPanel,
    ribbon_panel::RibbonPanelBuilder,
    text_panel::TextPanelBuilder,
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

struct MessageBoxPanelInternals {
    pub root_panel: RibbonPanel,
    pub control_manager: ControlManager,
    pub handle_yes: ButtonPanelHandle,
    pub handle_no: ButtonPanelHandle,
    pub handle_ok: ButtonPanelHandle,
    pub handle_cancel: ButtonPanelHandle,
}

impl MessageBoxPanelInternals {
    fn new(
        button_flags: BitFlags<MessageBoxButton>,
        message: Cow<'static, str>,
    ) -> winrt::Result<Self> {
        let mut root_panel = RibbonPanelBuilder::default()
            .orientation(RibbonOrientation::Stack)
            .build()?;
        let background_panel = BackgroundPanelBuilder::default()
            .color(Colors::wheat()?)
            .round_corners(true)
            .build()?;
        root_panel.push_panel(background_panel, 1.0)?;
        let message_panel = TextPanelBuilder::default().text(message).build()?;
        let mut button_yes = ButtonPanelBuilder::default().build()?;
        let mut button_no = ButtonPanelBuilder::default().build()?;
        let mut button_ok = ButtonPanelBuilder::default().build()?;
        let mut button_cancel = ButtonPanelBuilder::default().build()?;
        let text_yes = TextPanelBuilder::default().text("Yes").build()?;
        let text_no = TextPanelBuilder::default().text("No").build()?;
        let text_ok = TextPanelBuilder::default().text("OK").build()?;
        let text_cancel = TextPanelBuilder::default().text("Cancel").build()?;
        let handle_yes = button_yes.handle();
        let handle_no = button_no.handle();
        let handle_ok = button_ok.handle();
        let handle_cancel = button_cancel.handle();
        button_yes.set_panel(text_yes)?;
        button_no.set_panel(text_no)?;
        button_ok.set_panel(text_ok)?;
        button_cancel.set_panel(text_cancel)?;
        let mut ribbon = RibbonPanelBuilder::default()
            .orientation(RibbonOrientation::Vertical)
            .build()?;
        ribbon.push_panel(message_panel, 1.0)?;
        let mut ribbon_buttons = RibbonPanelBuilder::default()
            .orientation(RibbonOrientation::Horizontal)
            .build()?;
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
            root_panel,
            control_manager,
            handle_yes,
            handle_no,
            handle_ok,
            handle_cancel,
        })
    }
}

#[derive(Builder)]
#[builder(build_fn(private, name = "build_default"), setter(skip))]
pub struct MessageBoxPanel {
    id: usize,
    #[builder(setter(into), default = "MessageBoxButton::Ok.into()")]
    button_flags: BitFlags<MessageBoxButton>,
    #[builder(setter(into, strip_option), default = "Some(\"\".into())")]
    message: Option<Cow<'static, str>>,
    visual: ContainerVisual,
    internals: Option<MessageBoxPanelInternals>,
}

impl MessageBoxPanelBuilder {
    pub fn build(&self) -> winrt::Result<MessageBoxPanel> {
        match self.build_default() {
            Ok(mut panel) => {
                panel.finish_build()?;
                Ok(panel)
            }
            Err(e) => Err(winrt_error(e)),
        }
    }
}

impl MessageBoxPanel {
    fn finish_build(&mut self) -> winrt::Result<()> {
        let internals =
            MessageBoxPanelInternals::new(self.button_flags, self.message.take().unwrap())?;
        self.visual = globals().compositor().create_container_visual()?;
        self.visual
            .children()?
            .insert_at_top(internals.root_panel.visual())?;
        self.internals = Some(internals);
        Ok(())
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
        } else if let Some(ref mut internals) = self.internals {
            internals.root_panel.find_panel(id)
        } else {
            None
        }
    }

    fn on_init(&mut self, proxy: &crate::main_window::PanelEventProxy) -> winrt::Result<()> {
        if let Some(ref mut internals) = self.internals {
            internals.root_panel.on_init(proxy)?;
        }
        Ok(())
    }

    fn on_resize(
        &mut self,
        size: &bindings::windows::foundation::numerics::Vector2,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<()> {
        self.visual().set_size(size.clone())?;
        if let Some(ref mut internals) = self.internals {
            internals.root_panel.on_resize(size, proxy)?;
        }
        Ok(())
    }

    fn on_idle(&mut self, proxy: &crate::main_window::PanelEventProxy) -> winrt::Result<()> {
        if let Some(ref mut internals) = self.internals {
            internals.root_panel.on_idle(proxy)?;
        }
        Ok(())
    }

    fn on_mouse_move(
        &mut self,
        position: &bindings::windows::foundation::numerics::Vector2,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<()> {
        if let Some(ref mut internals) = self.internals {
            internals.root_panel.on_mouse_move(position, proxy)?;
        }
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<bool> {
        if let Some(ref mut internals) = self.internals {
            internals.root_panel.on_mouse_input(button, state, proxy)
        } else {
            Ok(false)
        }
    }

    fn on_keyboard_input(
        &mut self,
        input: winit::event::KeyboardInput,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<bool> {
        if let Some(ref mut internals) = self.internals {
            if let Some(key) = input.virtual_keycode {
                if key == VirtualKeyCode::Escape {
                    proxy.send_panel_event(self.id, MessageBoxButton::Cancel)?;
                    return Ok(true);
                }
            }
            Ok(internals.root_panel.on_keyboard_input(input, proxy)?
                || internals.control_manager.process_keyboard_input(
                    input,
                    &mut internals.root_panel,
                    proxy,
                )?)
        } else {
            Ok(false)
        }
    }

    fn on_panel_event(
        &mut self,
        panel_event: &mut crate::main_window::PanelEvent,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<()> {
        if let Some(ref mut internals) = self.internals {
            internals.root_panel.on_panel_event(panel_event, proxy)?;
            if internals.handle_yes.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed) {
                proxy.send_panel_event(self.id, MessageBoxButton::Yes)?;
            }
            if internals.handle_no.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed) {
                proxy.send_panel_event(self.id, MessageBoxButton::No)?;
            }
            if internals.handle_cancel.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed)
            {
                proxy.send_panel_event(self.id, MessageBoxButton::Cancel)?;
            }
            if internals.handle_ok.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed) {
                proxy.send_panel_event(self.id, MessageBoxButton::Ok)?;
            } else {
                internals.control_manager.process_panel_event(
                    panel_event,
                    &mut internals.root_panel,
                    proxy,
                )?;
            }
        }
        Ok(())
    }
}
