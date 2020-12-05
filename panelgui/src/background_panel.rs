use std::any::Any;

use bindings::windows::{
    foundation::numerics::Vector2,
    ui::{
        composition::{CompositionShape, ContainerVisual, ShapeVisual},
        Color,
    },
};
use float_ord::FloatOrd;
use winit::event::{ElementState, KeyboardInput, MouseButton};

use crate::main_window::{globals, winrt_error, Handle, Panel, PanelEventProxy, PanelHandle};

#[derive(Builder)]
#[builder(build_fn(private, name = "build_default"), setter(into))]
pub struct Background {
    color: Color,
    round_corners: bool,
}

impl BackgroundBuilder {
    pub fn build(&self) -> winrt::Result<BackgroundPanel> {
        match self.build_default() {
            Ok(settings) => Ok(BackgroundPanel::new(settings)?),
            Err(e) => Err(winrt_error(e)),
        }
    }
}

pub struct BackgroundPanel {
    id: usize,
    background: Background,
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
    pub fn new(background: Background) -> winrt::Result<Self> {
        let id = globals().get_next_id();
        let visual = globals().compositor().create_container_visual()?;
        let background_shape = globals().compositor().create_shape_visual()?;
        visual
            .children()?
            .insert_at_bottom(background_shape.clone())?;
        Ok(Self {
            id,
            background,
            visual,
            background_shape,
        })
    }
    pub fn handle(&self) -> BackgroundPanelHandle {
        BackgroundPanelHandle { id: self.id }
    }
    pub fn set_color(&mut self, color: Color) -> winrt::Result<()> {
        self.background.color = color;
        self.redraw_background()
    }
    pub fn set_round_corners(&mut self, round_corners: bool) -> winrt::Result<()> {
        self.background.round_corners = round_corners;
        self.redraw_background()
    }
    fn redraw_background(&mut self) -> winrt::Result<()> {
        self.background_shape.set_size(self.visual.size()?)?;
        self.background_shape.shapes()?.clear()?;
        self.background_shape
            .shapes()?
            .append(self.create_background_shape()?)?;
        Ok(())
    }
    fn create_background_shape(&self) -> winrt::Result<CompositionShape> {
        let container_shape = globals().compositor().create_container_shape()?;
        let rect_geometry = globals().compositor().create_rounded_rectangle_geometry()?;
        rect_geometry.set_size(self.background_shape.size()?)?;
        if self.background.round_corners {
            let size = rect_geometry.size()?;
            let radius = std::cmp::min(FloatOrd(size.x), FloatOrd(size.y)).0 / 20.;
            rect_geometry.set_corner_radius(Vector2 {
                x: radius,
                y: radius,
            })?;
        } else {
            rect_geometry.set_corner_radius(Vector2 { x: 0., y: 0. })?;
        }
        let brush = globals()
            .compositor()
            .create_color_brush_with_color(self.background.color.clone())?;
        let rect = globals()
            .compositor()
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

    fn on_resize(&mut self, size: &Vector2, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.visual.set_size(size.clone())?;
        self.redraw_background()
    }

    fn on_idle(&mut self, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: MouseButton,
        _state: ElementState,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(
        &mut self,
        _input: KeyboardInput,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        Ok(false)
    }

    fn on_init(&mut self, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }

    fn on_mouse_move(
        &mut self,
        _position: &Vector2,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }

    fn on_panel_event(
        &mut self,
        _panel_event: &mut crate::main_window::PanelEvent,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }
}
