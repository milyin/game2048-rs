use std::any::Any;

use bindings::windows::{
    foundation::numerics::Vector2,
    ui::{
        composition::{CompositionShape, ContainerVisual, ShapeVisual},
        Colors,
    },
};

use crate::main_window::{Handle, Panel, PanelGlobals, PanelHandle, PanelManager};

pub struct BackgroundPanel {
    id: usize,
    globals: PanelGlobals,
    visual: ContainerVisual,
    background: ShapeVisual,
    panel: Option<Box<dyn Panel>>,
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
    pub fn new(panel_manager: &mut PanelManager) -> winrt::Result<Self> {
        let id = panel_manager.get_next_id();
        let globals = panel_manager.get_globals();
        let visual = globals.compositor().create_container_visual()?;
        let background = globals.compositor().create_shape_visual()?;
        visual.children()?.insert_at_bottom(background.clone())?;
        Ok(Self {
            id,
            globals,
            visual,
            background,
            panel: None,
        })
    }
    pub fn handle(&self) -> BackgroundPanelHandle {
        BackgroundPanelHandle { id: self.id }
    }
    pub fn add_panel<P: Panel + 'static>(&mut self, panel: P) -> winrt::Result<()> {
        self.visual
            .children()?
            .insert_at_top(panel.visual().clone())?;
        self.panel = Some(Box::new(panel));
        Ok(())
    }
    pub fn panel(&mut self) -> Option<&mut (dyn Panel + 'static)> {
        self.panel.as_deref_mut()
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

    fn get_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id() {
            return Some(self.as_any_mut());
        } else if let Some(p) = self.panel() {
            p.get_panel(id)
        } else {
            None
        }
    }

    fn on_resize(&mut self) -> winrt::Result<()> {
        self.visual.set_size(self.visual.parent()?.size()?)?;
        self.redraw_background()?;
        if let Some(p) = self.panel() {
            p.on_resize()?;
        }
        Ok(())
    }

    fn on_idle(&mut self, _proxy: &crate::main_window::PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        position: bindings::windows::foundation::numerics::Vector2,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<bool> {
        if let Some(p) = self.panel() {
            p.on_mouse_input(position, button, state, proxy)
        } else {
            Ok(false)
        }
    }

    fn on_keyboard_input(
        &mut self,
        input: winit::event::KeyboardInput,
        proxy: &crate::main_window::PanelEventProxy,
    ) -> winrt::Result<bool> {
        if let Some(p) = self.panel() {
            p.on_keyboard_input(input, proxy)
        } else {
            Ok(false)
        }
    }
}
