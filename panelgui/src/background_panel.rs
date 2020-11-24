use std::any::Any;

use bindings::windows::{
    foundation::numerics::Vector2,
    ui::{
        composition::{CompositionShape, ContainerVisual, ShapeVisual},
        Colors,
    },
};
use winit::event::{ElementState, KeyboardInput, MouseButton};

use crate::main_window::{winrt_error, Handle, Panel, PanelEventProxy, PanelGlobals, PanelHandle};

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
    pub fn panel(&mut self) -> winrt::Result<&mut (dyn Panel + 'static)> {
        self.panel
            .as_deref_mut()
            .ok_or(winrt_error("Panel not added"))
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
        } else if let Ok(p) = self.panel() {
            p.find_panel(id)
        } else {
            None
        }
    }

    fn on_resize(&mut self, size: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.visual.set_size(size.clone())?;
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
        self.panel()?.on_mouse_input(button, state, proxy)
    }

    fn on_keyboard_input(
        &mut self,
        input: KeyboardInput,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        self.panel()?.on_keyboard_input(input, proxy)
    }

    fn on_init(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.panel()?.on_init(proxy)
    }

    fn on_mouse_move(&mut self, position: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.panel()?.on_mouse_move(position, proxy)
    }

    fn on_panel_event(
        &mut self,
        panel_event: &mut crate::main_window::PanelEvent,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        self.panel()?.on_panel_event(panel_event, proxy)
    }
}
