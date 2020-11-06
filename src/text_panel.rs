use std::{any::Any, borrow::Cow};

use bindings::{
    microsoft::graphics::canvas::{
        text::CanvasHorizontalAlignment, text::CanvasTextFormat, text::CanvasTextLayout,
        text::CanvasVerticalAlignment, ui::composition::CanvasComposition, CanvasDevice,
    },
    windows::{
        foundation::Size,
        graphics::directx::DirectXAlphaMode,
        graphics::directx::DirectXPixelFormat,
        ui::composition::CompositionDrawingSurface,
        ui::composition::CompositionGraphicsDevice,
        ui::{
            composition::{Compositor, ContainerVisual, SpriteVisual},
            Colors,
        },
    },
};

use crate::game_window::{GameWindow, Panel, PanelEventProxy};

#[derive(Copy, Clone)]
pub struct TextPanelHandle {
    id: usize,
}

impl TextPanelHandle {
    pub fn with_proxy<'a>(&self, proxy: &'a PanelEventProxy) -> TextPanelProxy<'a> {
        TextPanelProxy {
            handle: self.clone(),
            proxy,
        }
    }
}

pub struct TextPanelProxy<'a> {
    handle: TextPanelHandle,
    proxy: &'a PanelEventProxy,
}

enum TextPanelCommand {
    SetText(Cow<'static, str>),
}

impl<'a> TextPanelProxy<'a> {
    pub fn set_text<S: Into<Cow<'static, str>>>(&self, text: S) -> winrt::Result<()> {
        self.proxy
            .send_command_to_panel(self.handle.id, TextPanelCommand::SetText(text.into()))
    }
}

pub struct TextPanel {
    id: usize,
    text: Cow<'static, str>,
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    surface: Option<CompositionDrawingSurface>,
    visual: SpriteVisual,
}

impl TextPanel {
    pub fn new(game_window: &mut GameWindow) -> winrt::Result<Self> {
        let compositor = game_window.compositor().clone();
        let canvas_device = game_window.canvas_device().clone();
        let composition_graphics_device = game_window.composition_graphics_device().clone();
        let visual = compositor.create_sprite_visual()?;
        let surface = None;
        Ok(Self {
            id: game_window.get_next_id(),
            text: "".into(),
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

            ds.draw_text_layout_at_coords_with_color(text_layout, 0., 0., Colors::red()?)
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

    fn on_command(&mut self, command: Box<dyn Any>) -> winrt::Result<()> {
        if let Ok(command) = command.downcast::<TextPanelCommand>() {
            match *command {
                TextPanelCommand::SetText(text) => self.set_text(text)?,
            }
        }
        Ok(())
    }

    fn on_resize(&mut self) -> winrt::Result<()> {
        self.visual.set_size(self.visual.parent()?.size()?);
        self.resize_surface()?;
        self.redraw_text()?;
        Ok(())
    }

    fn on_idle(&mut self, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }
}
