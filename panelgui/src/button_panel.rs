use std::{any::Any, borrow::Cow, collections::HashMap};

use bindings::windows::{
    foundation::numerics::Vector2,
    ui::{
        composition::{CompositionShape, ContainerVisual, ShapeVisual},
        Colors,
    },
};
use float_ord::FloatOrd;
use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode};

use crate::{
    control::{Control, ControlHandle},
    main_window::{globals, winrt_error, Handle, Panel, PanelEvent, PanelEventProxy, PanelHandle},
    text_panel::TextParamsBuilder,
};

#[derive(PartialEq)]
pub enum ButtonPanelEvent {
    Pressed,
}
#[derive(Eq, PartialEq, Hash, Copy, Clone)]
enum ButtonMode {
    Norm,
    Disabled,
    Focused,
}
#[derive(Builder)]
#[builder(pattern="owned", setter(into))]
pub struct ButtonParams {
    #[builder(default = "true")]
    enabled: bool,
    #[builder(private,setter(name="panel_private"))]
    panel: Box<dyn Control>,
}

impl ButtonParamsBuilder {
    pub fn create(self) -> winrt::Result<ButtonPanel> {
        match self.build() {
            Ok(params) => Ok(ButtonPanel::new(params)?),
            Err(e) => Err(winrt_error(e)()),
        }
    }
    pub fn panel(self, panel: impl Control + 'static) -> Self {
        let panel: Box<dyn Control + 'static> = Box::new(panel);
        self.panel_private(panel)
    }
    pub fn text(self, text: impl Into<Cow<'static, str>>) -> winrt::Result<Self> {
        Ok(self.panel(TextParamsBuilder::default().text(text).create()?))
    }
}

pub struct ButtonPanel {
    handle: ButtonPanelHandle,
    visual: ContainerVisual,
    background: ShapeVisual,
    shapes: HashMap<ButtonMode, (Vector2, CompositionShape)>,
    focused: bool,
    params: ButtonParams,
}

#[derive(Copy, Clone, PartialEq)]
pub struct ButtonPanelHandle(usize);

impl ButtonPanelHandle {
    fn new() -> Self {
        Self {
            0: globals().get_next_id(),
        }
    }
}

impl Handle for ButtonPanelHandle {
    fn id(&self) -> usize {
        self.0
    }
}

impl PanelHandle<ButtonPanel, ButtonPanelEvent> for ButtonPanelHandle {}

impl ControlHandle for ButtonPanelHandle {
    fn as_control<'a>(&self, root_panel: &'a mut dyn Panel) -> Option<&'a mut dyn Control> {
        self.at(root_panel).ok().map(|p| p as &mut dyn Control)
    }
}
impl ButtonPanel {
    pub fn new(params: ButtonParams) -> winrt::Result<Self> {
        let handle = ButtonPanelHandle::new();
        let visual = globals().compositor().create_container_visual()?;
        let background = globals().compositor().create_shape_visual()?;
        visual.children()?.insert_at_bottom(background.clone())?;
        visual.children()?.insert_at_top(params.panel.visual().clone())?;
        Ok(Self {
            handle,
            params,
            visual,
            background,
            shapes: HashMap::new(),
            focused: false,
        })
    }
    pub fn handle(&self) -> ButtonPanelHandle {
        self.handle
    }
/*   pub fn set_panel<P: Control + 'static>(&mut self, panel: P) -> winrt::Result<()> {
        self.remove_panel()?;
        self.visual
            .children()?
            .insert_at_top(panel.visual().clone())?;
        self.params.panel = Some(Box::new(panel));
        Ok(())
    }*/
    pub fn panel(&mut self) -> winrt::Result<&mut (dyn Control + 'static)> {
        Ok(&mut *self.params.panel)
    }
    fn press(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        if self.params.enabled {
            proxy.send_panel_event(self.handle.id(), ButtonPanelEvent::Pressed)?;
        }
        Ok(())
    }
    fn get_shape(&mut self, mode: ButtonMode) -> winrt::Result<CompositionShape> {
        let size = self.background.size()?;
        if let Some((shape_size, shape)) = self.shapes.get(&mode) {
            if *shape_size == size {
                return Ok(shape.clone());
            }
        }
        let shape = Self::create_shape(mode, &size)?;
        self.shapes.insert(mode, (size, shape.clone()));
        Ok(shape)
    }
    fn create_shape(mode: ButtonMode, size: &Vector2) -> winrt::Result<CompositionShape> {
        let container_shape = globals().compositor().create_container_shape()?;
        let round_rect_geometry = globals().compositor().create_rounded_rectangle_geometry()?;
        let offset = std::cmp::min(FloatOrd(size.x), FloatOrd(size.y)).0 / 20.;
        round_rect_geometry.set_corner_radius(Vector2 {
            x: offset,
            y: offset,
        })?;
        round_rect_geometry.set_size(Vector2 {
            x: size.x - offset * 2.,
            y: size.y - offset * 2.,
        })?;
        round_rect_geometry.set_offset(Vector2 {
            x: offset,
            y: offset,
        })?;
        let (border_color, border_thickness) = match mode {
            // ButtonMode::Norm => (Colors::black()?, 1.),
            // ButtonMode::Disabled => (Colors::gray()?, 1.),
            // ButtonMode::Focused => (Colors::black()?, 3.),
            ButtonMode::Norm => (Colors::white()?, 1.),
            ButtonMode::Disabled => (Colors::white()?, 1.),
            ButtonMode::Focused => (Colors::black()?, 1.),
        };
        let fill_brush = globals()
            .compositor()
            .create_color_brush_with_color(Colors::white()?)?;
        let stroke_brush = globals()
            .compositor()
            .create_color_brush_with_color(border_color)?;
        let rect = globals()
            .compositor()
            .create_sprite_shape_with_geometry(round_rect_geometry)?;
        rect.set_fill_brush(fill_brush)?;
        rect.set_stroke_brush(stroke_brush)?;
        rect.set_stroke_thickness(border_thickness)?;
        rect.set_offset(Vector2 { x: 0., y: 0. })?;
        container_shape.shapes()?.append(rect)?;
        let shape = container_shape.into();
        Ok(shape)
    }
    fn get_mode(&self) -> ButtonMode {
        if self.params.enabled {
            if self.focused {
                ButtonMode::Focused
            } else {
                ButtonMode::Norm
            }
        } else {
            ButtonMode::Disabled
        }
    }
    fn redraw_background(&mut self) -> winrt::Result<()> {
        self.background.set_size(self.visual.size()?)?;
        self.background.shapes()?.clear()?;
        self.background
            .shapes()?
            .append(self.get_shape(self.get_mode())?)?;
        Ok(())
    }
}

impl Panel for ButtonPanel {
    fn id(&self) -> usize {
        self.handle.id()
    }
    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }

    fn on_resize(&mut self, size: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.visual.set_size(self.visual.parent()?.size()?)?;
        self.redraw_background()?;
        self.panel()?.on_resize(size, proxy)
    }

    fn on_idle(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.panel()?.on_idle(proxy)
    }

    fn on_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        if self.is_enabled()? && button == MouseButton::Left && state == ElementState::Pressed {
            self.set_focus(proxy)?;
            self.press(proxy)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id() {
            return Some(self.as_any_mut());
        } else {
            self.params.panel.find_panel(id)
        }
    }

    fn on_keyboard_input(
        &mut self,
        input: KeyboardInput,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        if self.is_focused()? && self.is_enabled()? {
            if input.state == ElementState::Pressed {
                if let Some(code) = input.virtual_keycode {
                    match code {
                        VirtualKeyCode::Escape => {
                            self.clear_focus(proxy)?;
                            return Ok(true);
                        }
                        VirtualKeyCode::Tab => {
                            // TODO: Check WindowEvent::ModifiersChanged modifiers for shift-tab
                            self.set_focus_to_next(proxy)?;
                            return Ok(true);
                        }
                        VirtualKeyCode::Return => {
                            self.press(proxy)?;
                            return Ok(true);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(false)
    }

    fn on_init(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.panel()?.on_init(proxy)
    }

    fn on_mouse_move(&mut self, position: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.panel()?.on_mouse_move(position, proxy)
    }

    fn on_panel_event(
        &mut self,
        panel_event: &mut PanelEvent,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        self.panel()?.on_panel_event(panel_event, proxy)
    }
}

impl Control for ButtonPanel {
    fn on_enable(&mut self, enable: bool) -> winrt::Result<()> {
        self.params.enabled = enable;
        self.panel()?.on_enable(enable)
    }

    fn on_set_focus(&mut self) -> winrt::Result<()> {
        self.focused = true;
        self.redraw_background()
    }

    fn as_panel(&self) -> &dyn Panel {
        self
    }

    fn is_enabled(&self) -> winrt::Result<bool> {
        Ok(self.params.enabled)
    }

    fn is_focused(&self) -> winrt::Result<bool> {
        Ok(self.focused)
    }

    fn on_clear_focus(&mut self) -> winrt::Result<()> {
        self.focused = false;
        self.redraw_background()
    }
}
