use std::any::Any;

use bindings::windows::{
    foundation::numerics::Vector2,
    ui::{
        composition::{CompositionShape, ContainerVisual, ShapeVisual},
        Color, Colors,
    },
};
use float_ord::FloatOrd;
use winit::event::{ElementState, KeyboardInput, MouseButton};

use crate::main_window::{Handle, Panel, PanelEventProxy, PanelHandle, compositor, get_next_id, winrt_error};

#[derive(Builder)]
#[builder(setter(into))]
pub struct BackgroundParams {
    #[builder(default = "{Colors::white().unwrap()}")]
    color: Color,
    #[builder(default = "{false}")]
    round_corners: bool,
}

impl Default for BackgroundParams {
    fn default() -> Self {
        Self {
            color: Colors::transparent().unwrap(),
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
        let visual = compositor().create_container_visual()?;
        let background_shape = compositor().create_shape_visual()?;
        visual
            .children()?
            .insert_at_bottom(background_shape.clone())?;
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
        self.background_shape.set_size(self.visual.size()?)?;
        self.background_shape.shapes()?.clear()?;
        self.background_shape
            .shapes()?
            .append(self.create_background_shape()?)?;
        Ok(())
    }
    fn create_background_shape(&self) -> windows::Result<CompositionShape> {
        let container_shape = compositor().create_container_shape()?;
        let rect_geometry = compositor().create_rounded_rectangle_geometry()?;
        rect_geometry.set_size(self.background_shape.size()?)?;
        if self.params.round_corners {
            let size = rect_geometry.size()?;
            let radius = std::cmp::min(FloatOrd(size.x), FloatOrd(size.y)).0 / 20.;
            rect_geometry.set_corner_radius(Vector2 {
                x: radius,
                y: radius,
            })?;
        } else {
            rect_geometry.set_corner_radius(Vector2 { x: 0., y: 0. })?;
        }
        let brush = 
            compositor()
            .create_color_brush_with_color(self.params.color.clone())?;
        let rect = 
            compositor()
            .create_sprite_shape_with_geometry(rect_geometry)?;
        rect.set_fill_brush(brush)?;
        rect.set_offset(Vector2 { x: 0., y: 0. })?;
        container_shape.shapes()?.append(rect)?;
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

    fn on_resize(&mut self, size: &Vector2, _proxy: &PanelEventProxy) -> windows::Result<()> {
        self.visual.set_size(size.clone())?;
        self.redraw_background()
    }

    fn on_idle(&mut self, _proxy: &PanelEventProxy) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: MouseButton,
        _state: ElementState,
        _proxy: &PanelEventProxy,
    ) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(
        &mut self,
        _input: KeyboardInput,
        _proxy: &PanelEventProxy,
    ) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_init(&mut self, _proxy: &PanelEventProxy) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_move(
        &mut self,
        _position: &Vector2,
        _proxy: &PanelEventProxy,
    ) -> windows::Result<()> {
        Ok(())
    }

    fn on_panel_event(
        &mut self,
        _panel_event: &mut crate::main_window::PanelEvent,
        _proxy: &PanelEventProxy,
    ) -> windows::Result<()> {
        Ok(())
    }
}
