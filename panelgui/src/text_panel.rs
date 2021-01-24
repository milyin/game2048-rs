use std::borrow::Cow;

use bindings::{
    microsoft::graphics::canvas::{
        text::CanvasHorizontalAlignment, text::CanvasTextFormat, text::CanvasTextLayout,
        text::CanvasVerticalAlignment, ui::composition::CanvasComposition,
    },
    windows::{
        foundation::numerics::Vector2,
        foundation::Size,
        graphics::directx::DirectXAlphaMode,
        graphics::directx::DirectXPixelFormat,
        ui::composition::CompositionDrawingSurface,
        ui::{
            composition::{ContainerVisual, SpriteVisual},
            Color, Colors,
        },
    },
};

use crate::{
    control::{Control, ControlHandle},
    main_window::{globals, winrt_error, Handle, Panel, PanelEventProxy, PanelHandle},
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

#[derive(Builder)]
#[builder(pattern = "owned", setter(into))]
pub struct TextParams {
    #[builder(default = "{\"\".into()}")]
    text: Cow<'static, str>,
    #[builder(default = "{true}")]
    enabled: bool,
    #[builder(default = "{Colors::black().unwrap()}")]
    color: Color,
    #[builder(default = "{2.}")]
    font_scale: f32,
}

impl TextParamsBuilder {
    pub fn create(self) -> windows::Result<TextPanel> {
        match self.build() {
            Ok(settings) => Ok(TextPanel::new(settings)?),
            Err(e) => Err(winrt_error(e)()),
        }
    }
}

pub struct TextPanel {
    id: usize,
    params: TextParams,
    surface: Option<CompositionDrawingSurface>,
    visual: SpriteVisual,
}

impl TextPanel {
    pub fn new(params: TextParams) -> windows::Result<Self> {
        let id = globals().get_next_id();
        let visual = globals().compositor().create_sprite_visual()?;
        Ok(Self {
            id,
            params,
            surface: None,
            visual,
        })
    }
    pub fn handle(&self) -> TextPanelHandle {
        TextPanelHandle { id: self.id }
    }
    pub fn set_text<S: Into<Cow<'static, str>>>(&mut self, text: S) -> windows::Result<()> {
        self.params.text = text.into();
        self.redraw_text()
    }
    pub fn set_text_color(&mut self, color: Color) -> windows::Result<()> {
        self.params.color = color;
        self.redraw_text()
    }

    fn resize_surface(&mut self) -> windows::Result<()> {
        let size = self.visual.size()?;
        if size.x > 0. && size.y > 0. {
            let surface = globals()
                .composition_graphics_device()
                .create_drawing_surface(
                    Size {
                        width: size.x,
                        height: size.y,
                    },
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    DirectXAlphaMode::Premultiplied,
                )?;

            let brush = globals().compositor().create_surface_brush()?;
            brush.set_surface(surface.clone())?;
            self.surface = Some(surface);
            self.visual.set_brush(brush)?;
        }
        Ok(())
    }

    fn redraw_text(&self) -> windows::Result<()> {
        if let Some(ref surface) = self.surface {
            let ds = CanvasComposition::create_drawing_session(surface)?;
            ds.clear(Colors::transparent()?)?;

            let size = surface.size()?;
            let text_format = CanvasTextFormat::new()?;
            text_format.set_font_family("Arial")?;
            text_format.set_font_size(size.height / self.params.font_scale)?;
            let text: String = self.params.text.clone().into();
            let text_layout = CanvasTextLayout::create(
                globals().canvas_device(),
                text,
                text_format,
                size.width,
                size.height,
            )?;
            text_layout.set_vertical_alignment(CanvasVerticalAlignment::Center)?;
            text_layout.set_horizontal_alignment(CanvasHorizontalAlignment::Center)?;
            let color = if self.params.enabled {
                self.params.color.clone()
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

    fn on_resize(&mut self, size: &Vector2, _proxy: &PanelEventProxy) -> windows::Result<()> {
        self.visual.set_size(size)?;
        self.resize_surface()?;
        self.redraw_text()?;
        Ok(())
    }

    fn on_idle(&mut self, _proxy: &PanelEventProxy) -> windows::Result<()> {
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

    fn on_init(&mut self, proxy: &PanelEventProxy) -> windows::Result<()> {
        self.on_resize(&self.visual().parent()?.size()?, proxy)
    }

    fn on_mouse_move(
        &mut self,
        _position: &Vector2,
        _proxy: &PanelEventProxy,
    ) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: winit::event::MouseButton,
        _state: winit::event::ElementState,
        _proxy: &PanelEventProxy,
    ) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(
        &mut self,
        _input: winit::event::KeyboardInput,
        _proxy: &PanelEventProxy,
    ) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_panel_event(
        &mut self,
        _panel_event: &mut crate::main_window::PanelEvent,
        _proxy: &PanelEventProxy,
    ) -> windows::Result<()> {
        Ok(())
    }
}

impl Control for TextPanel {
    fn on_enable(&mut self, enable: bool) -> windows::Result<()> {
        self.params.enabled = enable;
        self.redraw_text()
    }

    fn on_set_focus(&mut self) -> windows::Result<()> {
        todo!()
    }

    fn as_panel(&self) -> &dyn Panel {
        self
    }
}
