use std::any::Any;

use bindings::Windows::{
    Foundation::Numerics::Vector2,
    UI::{
        Color, Colors,
        Composition::{CompositionShape, ContainerVisual, ShapeVisual},
    },
};
use float_ord::FloatOrd;
use winit::event::{ElementState, KeyboardInput, MouseButton};

use crate::{
    globals::{compositor, get_next_id, winrt_error},
    panel::{Handle, Panel, PanelEvent, PanelHandle},
};

#[derive(Builder)]
#[builder(setter(into))]
pub struct BackgroundParams {
    #[builder(default = "{Colors::White().unwrap()}")]
    color: Color,
    #[builder(default = "{false}")]
    round_corners: bool,
}

impl Default for BackgroundParams {
    fn default() -> Self {
        Self {
            color: Colors::Transparent().unwrap(),
            round_corners: false,
        }
    }
}

impl BackgroundParamsBuilder {
    pub fn create(&self) -> windows::Result<BackgroundPanel> {
        match self.build() {
            Ok(params) => Ok(BackgroundPanel::new(params)?),
            Err(e) => Err(winrt_error(e)()),
        }
    }
}

pub struct BackgroundPanel {
    id: usize,
    params: BackgroundParams,
    visual: ContainerVisual,
    background_shape: ShapeVisual,
}

pub struct BackgroundPanelHandle {
    id: usize,
}

impl Handle for BackgroundPanelHandle {
    fn id(&self) -> usize {
        self.id
    }
}

impl PanelHandle<BackgroundPanel> for BackgroundPanelHandle {}

impl BackgroundPanel {
    pub fn new(params: BackgroundParams) -> windows::Result<Self> {
        let id = get_next_id();
        let visual = compositor().CreateContainerVisual()?;
        let background_shape = compositor().CreateShapeVisual()?;
        visual
            .Children()?
            .InsertAtBottom(background_shape.clone())?;
        Ok(Self {
            id,
            params,
            visual,
            background_shape,
        })
    }
    pub fn handle(&self) -> BackgroundPanelHandle {
        BackgroundPanelHandle { id: self.id }
    }
    pub fn set_color(&mut self, color: Color) -> windows::Result<()> {
        self.params.color = color;
        self.redraw_background()
    }
    pub fn set_round_corners(&mut self, round_corners: bool) -> windows::Result<()> {
        self.params.round_corners = round_corners;
        self.redraw_background()
    }
    fn redraw_background(&mut self) -> windows::Result<()> {
        self.background_shape.SetSize(self.visual.Size()?)?;
        self.background_shape.Shapes()?.Clear()?;
        self.background_shape
            .Shapes()?
            .Append(self.create_background_shape()?)?;
        Ok(())
    }
    fn create_background_shape(&self) -> windows::Result<CompositionShape> {
        let container_shape = compositor().CreateContainerShape()?;
        let rect_geometry = compositor().CreateRoundedRectangleGeometry()?;
        rect_geometry.SetSize(self.background_shape.Size()?)?;
        if self.params.round_corners {
            let size = rect_geometry.Size()?;
            let radius = std::cmp::min(FloatOrd(size.X), FloatOrd(size.Y)).0 / 20.;
            rect_geometry.SetCornerRadius(Vector2 {
                X: radius,
                Y: radius,
            })?;
        } else {
            rect_geometry.SetCornerRadius(Vector2 { X: 0., Y: 0. })?;
        }
        let brush = compositor().CreateColorBrushWithColor(self.params.color.clone())?;
        let rect = compositor().CreateSpriteShapeWithGeometry(rect_geometry)?;
        rect.SetFillBrush(brush)?;
        rect.SetOffset(Vector2 { X: 0., Y: 0. })?;
        container_shape.Shapes()?.Append(rect)?;
        let shape = container_shape.into();
        Ok(shape)
    }
}

impl Panel for BackgroundPanel {
    fn id(&self) -> usize {
        self.id
    }

    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id() {
            return Some(self.as_any_mut());
        } else {
            None
        }
    }

    fn on_resize(&mut self, size: &Vector2) -> windows::Result<()> {
        self.visual.SetSize(size.clone())?;
        self.redraw_background()
    }

    fn on_idle(&mut self) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: MouseButton,
        _state: ElementState,
    ) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(&mut self, _input: KeyboardInput) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_init(&mut self) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_move(&mut self, _position: &Vector2) -> windows::Result<()> {
        Ok(())
    }

    fn on_panel_event(&mut self, _panel_event: &mut PanelEvent) -> windows::Result<()> {
        Ok(())
    }
}
