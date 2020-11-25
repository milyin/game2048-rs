use std::borrow::Cow;

use bindings::{
    microsoft::graphics::canvas::{
        text::CanvasHorizontalAlignment, text::CanvasTextFormat, text::CanvasTextLayout,
        text::CanvasVerticalAlignment, ui::composition::CanvasComposition, CanvasDevice,
    },
    windows::{
        foundation::numerics::Vector2,
        foundation::Size,
        graphics::directx::DirectXAlphaMode,
        graphics::directx::DirectXPixelFormat,
        ui::composition::CompositionDrawingSurface,
        ui::composition::CompositionGraphicsDevice,
        ui::{
            composition::{Compositor, ContainerVisual, SpriteVisual},
            Color, Colors,
        },
    },
};

use crate::{
    control::{Control, ControlHandle},
    main_window::{Handle, Panel, PanelEventProxy, PanelGlobals, PanelHandle},
};

#[derive(Copy, Clone)]
pub struct TextPanelHandle {
    id: usize,
}

impl Handle for TextPanelHandle {
    fn id(&self) -> usize {
        self.id
    }
}

impl PanelHandle<TextPanel> for TextPanelHandle {}

impl ControlHandle for TextPanelHandle {
    fn as_control<'a>(&self, root_panel: &'a mut dyn Panel) -> Option<&'a mut dyn Control> {
        self.at(root_panel).ok().map(|p| p as &mut dyn Control)
    }
}

pub struct TextPanel {
    id: usize,
    text: Cow<'static, str>,
    enabled: bool,
    text_color: Color,
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    surface: Option<CompositionDrawingSurface>,
    visual: SpriteVisual,
}

impl TextPanel {
    pub fn new(globals: &PanelGlobals) -> winrt::Result<Self> {
        let compositor = globals.compositor().clone();
        let canvas_device = globals.canvas_device().clone();
        let composition_graphics_device = globals.composition_graphics_device().clone();
        let visual = compositor.create_sprite_visual()?;
        let surface = None;
        let enabled = true;
        let text_color = Colors::black()?;
        Ok(Self {
            id: globals.get_next_id(),
            text: "".into(),
            enabled,
            text_color,
            compositor,
            canvas_device,
            composition_graphics_device,
            visual,
            surface,
        })
    }
    pub fn handle(&self) -> TextPanelHandle {
        TextPanelHandle { id: self.id }
    }
    pub fn set_text<S: Into<Cow<'static, str>>>(&mut self, text: S) -> winrt::Result<()> {
        self.text = text.into();
        self.redraw_text()
    }
    pub fn set_text_color(&mut self, color: Color) -> winrt::Result<()> {
        self.text_color = color;
        self.redraw_text()
    }

    fn resize_surface(&mut self) -> winrt::Result<()> {
        let size = self.visual.size()?;
        if size.x > 0. && size.y > 0. {
            let surface = self.composition_graphics_device.create_drawing_surface(
                Size {
                    width: size.x,
                    height: size.y,
                },
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                DirectXAlphaMode::Premultiplied,
            )?;

            let brush = self.compositor.create_surface_brush()?;
            brush.set_surface(surface.clone())?;
            self.surface = Some(surface);
            self.visual.set_brush(brush)?;
        }
        Ok(())
    }

    fn redraw_text(&self) -> winrt::Result<()> {
        if let Some(ref surface) = self.surface {
            let ds = CanvasComposition::create_drawing_session(surface)?;
            ds.clear(Colors::transparent()?)?;

            let size = surface.size()?;
            let text_format = CanvasTextFormat::new()?;
            text_format.set_font_family("Arial")?;
            text_format.set_font_size(size.height / 2.)?;
            let text: String = self.text.clone().into();
            let text_layout = CanvasTextLayout::create(
                &self.canvas_device,
                text,
                text_format,
                size.width,
                size.height,
            )?;
            text_layout.set_vertical_alignment(CanvasVerticalAlignment::Center)?;
            text_layout.set_horizontal_alignment(CanvasHorizontalAlignment::Center)?;
            let color = if self.enabled {
                self.text_color.clone()
            } else {
                Colors::gray()?
            };

            ds.draw_text_layout_at_coords_with_color(text_layout, 0., 0., color)
        } else {
            Ok(())
            }
    }
}

impl Panel for TextPanel {
    fn id(&self) -> usize {
        self.id
    }
    fn visual(&self) -> ContainerVisual {
        self.visual.clone().into()
    }

    fn on_resize(&mut self, size: &Vector2, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.visual.set_size(size)?;
        self.resize_surface()?;
        self.redraw_text()?;
        Ok(())
    }

    fn on_idle(&mut self, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn std::any::Any> {
        if self.id == id {
            Some(self.as_any_mut())
        } else {
            None
        }
    }

    fn on_init(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.on_resize(&self.visual().parent()?.size()?, proxy)
    }

    fn on_mouse_move(
        &mut self,
        _position: &Vector2,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: winit::event::MouseButton,
        _state: winit::event::ElementState,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(
        &mut self,
        _input: winit::event::KeyboardInput,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        Ok(false)
    }

    fn on_panel_event(
        &mut self,
        _panel_event: &mut crate::main_window::PanelEvent,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }
}

impl Control for TextPanel {
    fn on_enable(&mut self, enable: bool) -> winrt::Result<()> {
        self.enabled = enable;
        self.redraw_text()
    }

    fn on_set_focus(&mut self) -> winrt::Result<()> {
        todo!()
    }

    fn as_panel(&self) -> &dyn Panel {
        self
    }
}
