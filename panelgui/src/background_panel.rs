use std::any::Any;

use bindings::windows::{
    foundation::numerics::Vector2,
    ui::{
        composition::{CompositionShape, ContainerVisual, ShapeVisual},
        Colors,
    },
};
use winit::event::{ElementState, KeyboardInput, MouseButton};

use crate::main_window::{Handle, Panel, PanelEventProxy, PanelGlobals, PanelHandle};

pub struct BackgroundPanel {
    id: usize,
    globals: PanelGlobals,
    visual: ContainerVisual,
    background: ShapeVisual,
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
    pub fn new(globals: &PanelGlobals) -> winrt::Result<Self> {
        let id = globals.get_next_id();
        let visual = globals.compositor().create_container_visual()?;
        let background = globals.compositor().create_shape_visual()?;
        visual.children()?.insert_at_bottom(background.clone())?;
        Ok(Self {
            id,
            globals: globals.clone(),
            visual,
            background,
        })
    }
    pub fn handle(&self) -> BackgroundPanelHandle {
        BackgroundPanelHandle { id: self.id }
    }
    fn redraw_background(&mut self) -> winrt::Result<()> {
        self.background.set_size(self.visual.size()?)?;
        self.background.shapes()?.clear()?;
        self.background
            .shapes()?
            .append(self.create_background_shape()?)?;
        Ok(())
    }
    fn create_background_shape(&self) -> winrt::Result<CompositionShape> {
        let container_shape = self.globals.compositor().create_container_shape()?;
        let rect_geometry = self.globals.compositor().create_rectangle_geometry()?;
        rect_geometry.set_size(self.background.size()?)?;
        let brush = self
            .globals
            .compositor()
            .create_color_brush_with_color(Colors::white()?)?;
        let rect = self
            .globals
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
