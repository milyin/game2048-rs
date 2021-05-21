use std::{any::Any, borrow::Cow, collections::HashMap};

use bindings::Windows::{
    Foundation::Numerics::Vector2,
    UI::{
        Colors,
        Composition::{CompositionShape, ContainerVisual, ShapeVisual},
    },
};
use float_ord::FloatOrd;
use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode};

use crate::{
    control::{Control, ControlHandle},
    globals::{compositor, get_next_id, send_panel_event, winrt_error},
    panel::{Handle, Panel, PanelEvent, PanelHandle},
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
#[builder(pattern = "owned", setter(into))]
pub struct ButtonParams {
    #[builder(default = "{true}")]
    enabled: bool,
    #[builder(private, setter(name = "panel_private"))]
    panel: Box<dyn Control>,
}

impl ButtonParamsBuilder {
    pub fn create(self) -> windows::Result<ButtonPanel> {
        match self.build() {
            Ok(params) => Ok(ButtonPanel::new(params)?),
            Err(e) => Err(winrt_error(e)()),
        }
    }
    pub fn panel(self, panel: impl Control + 'static) -> Self {
        let panel: Box<dyn Control + 'static> = Box::new(panel);
        self.panel_private(panel)
    }
    pub fn text(self, text: impl Into<Cow<'static, str>>) -> windows::Result<Self> {
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
        Self { 0: get_next_id() }
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
    pub fn new(params: ButtonParams) -> windows::Result<Self> {
        let handle = ButtonPanelHandle::new();
        let visual = compositor().CreateContainerVisual()?;
        let background = compositor().CreateShapeVisual()?;
        visual.Children()?.InsertAtBottom(background.clone())?;
        visual
            .Children()?
            .InsertAtTop(params.panel.visual().clone())?;
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
    /*   pub fn set_panel<P: Control + 'static>(&mut self, panel: P) -> windows::Result<()> {
        self.remove_panel()?;
        self.visual
            .Children()?
            .InsertAtTop(panel.visual().clone())?;
        self.params.panel = Some(Box::new(panel));
        Ok(())
    }*/
    pub fn panel(&mut self) -> windows::Result<&mut (dyn Control + 'static)> {
        Ok(&mut *self.params.panel)
    }
    fn press(&mut self) -> windows::Result<()> {
        if self.params.enabled {
            send_panel_event(self.handle.id(), ButtonPanelEvent::Pressed)?;
        }
        Ok(())
    }
    fn get_shape(&mut self, mode: ButtonMode) -> windows::Result<CompositionShape> {
        let size = self.background.Size()?;
        if let Some((shape_size, shape)) = self.shapes.get(&mode) {
            if *shape_size == size {
                return Ok(shape.clone());
            }
        }
        let shape = Self::create_shape(mode, &size)?;
        self.shapes.insert(mode, (size, shape.clone()));
        Ok(shape)
    }
    fn create_shape(mode: ButtonMode, size: &Vector2) -> windows::Result<CompositionShape> {
        let container_shape = compositor().CreateContainerShape()?;
        let round_rect_geometry = compositor().CreateRoundedRectangleGeometry()?;
        let offset = std::cmp::min(FloatOrd(size.X), FloatOrd(size.Y)).0 / 20.;
        round_rect_geometry.SetCornerRadius(Vector2 {
            X: offset,
            Y: offset,
        })?;
        round_rect_geometry.SetSize(Vector2 {
            X: size.X - offset * 2.,
            Y: size.Y - offset * 2.,
        })?;
        round_rect_geometry.SetOffset(Vector2 {
            X: offset,
            Y: offset,
        })?;
        let (border_color, border_thickness) = match mode {
            // ButtonMode::Norm => (Colors::black()?, 1.),
            // ButtonMode::Disabled => (Colors::gray()?, 1.),
            // ButtonMode::Focused => (Colors::black()?, 3.),
            ButtonMode::Norm => (Colors::White()?, 1.),
            ButtonMode::Disabled => (Colors::White()?, 1.),
            ButtonMode::Focused => (Colors::Black()?, 1.),
        };
        let fill_brush = compositor().CreateColorBrushWithColor(Colors::White()?)?;
        let stroke_brush = compositor().CreateColorBrushWithColor(border_color)?;
        let rect = compositor().CreateSpriteShapeWithGeometry(round_rect_geometry)?;
        rect.SetFillBrush(fill_brush)?;
        rect.SetStrokeBrush(stroke_brush)?;
        rect.SetStrokeThickness(border_thickness)?;
        rect.SetOffset(Vector2 { X: 0., Y: 0. })?;
        container_shape.Shapes()?.Append(rect)?;
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
    fn redraw_background(&mut self) -> windows::Result<()> {
        self.background.SetSize(self.visual.Size()?)?;
        self.background.Shapes()?.Clear()?;
        self.background
            .Shapes()?
            .Append(self.get_shape(self.get_mode())?)?;
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

    fn on_resize(&mut self, size: &Vector2) -> windows::Result<()> {
        self.visual.SetSize(self.visual.Parent()?.Size()?)?;
        self.redraw_background()?;
        self.panel()?.on_resize(size)
    }

    fn on_idle(&mut self) -> windows::Result<()> {
        self.panel()?.on_idle()
    }

    fn on_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
    ) -> windows::Result<bool> {
        if self.is_enabled()? && button == MouseButton::Left && state == ElementState::Pressed {
            self.set_focus()?;
            self.press()?;
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

    fn on_keyboard_input(&mut self, input: KeyboardInput) -> windows::Result<bool> {
        if self.is_focused()? && self.is_enabled()? {
            if input.state == ElementState::Pressed {
                if let Some(code) = input.virtual_keycode {
                    match code {
                        VirtualKeyCode::Escape => {
                            self.clear_focus()?;
                            return Ok(true);
                        }
                        VirtualKeyCode::Tab => {
                            // TODO: Check WindowEvent::ModifiersChanged modifiers for shift-tab
                            self.set_focus_to_next()?;
                            return Ok(true);
                        }
                        VirtualKeyCode::Return => {
                            self.press()?;
                            return Ok(true);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(false)
    }

    fn on_init(&mut self) -> windows::Result<()> {
        self.panel()?.on_init()
    }

    fn on_mouse_move(&mut self, position: &Vector2) -> windows::Result<()> {
        self.panel()?.on_mouse_move(position)
    }

    fn on_panel_event(&mut self, panel_event: &mut PanelEvent) -> windows::Result<()> {
        self.panel()?.on_panel_event(panel_event)
    }
}

impl Control for ButtonPanel {
    fn on_enable(&mut self, enable: bool) -> windows::Result<()> {
        self.params.enabled = enable;
        self.panel()?.on_enable(enable)
    }

    fn on_set_focus(&mut self) -> windows::Result<()> {
        self.focused = true;
        self.redraw_background()
    }

    fn as_panel(&self) -> &dyn Panel {
        self
    }

    fn is_enabled(&self) -> windows::Result<bool> {
        Ok(self.params.enabled)
    }

    fn is_focused(&self) -> windows::Result<bool> {
        Ok(self.focused)
    }

    fn on_clear_focus(&mut self) -> windows::Result<()> {
        self.focused = false;
        self.redraw_background()
    }
}
