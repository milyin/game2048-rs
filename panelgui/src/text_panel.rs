use std::borrow::Cow;

use bindings::{
    Microsoft::Graphics::Canvas::{
        Text::CanvasHorizontalAlignment, Text::CanvasTextFormat, Text::CanvasTextLayout,
        Text::CanvasVerticalAlignment, UI::Composition::CanvasComposition,
    },
    Windows::{
        Foundation::Numerics::Vector2,
        Foundation::Size,
        Graphics::DirectX::DirectXAlphaMode,
        Graphics::DirectX::DirectXPixelFormat,
        UI::Composition::CompositionDrawingSurface,
        UI::{
            Color, Colors,
            Composition::{ContainerVisual, SpriteVisual},
        },
    },
};

use crate::{
    control::{Control, ControlHandle},
    globals::{
        canvas_device, composition_graphics_device, compositor, get_next_id, root_panel_with,
        winrt_error,
    },
    panel::{Handle, Panel, PanelEvent, PanelHandle},
};

#[derive(Copy, Clone)]
pub struct TextPanelHandle {
    id: usize,
}

impl TextPanelHandle {
    pub fn set_text_at<S: Into<Cow<'static, str>>>(
        &self,
        owner: &mut dyn Panel,
        text: S,
    ) -> windows::Result<()> {
        self.at(owner)?.set_text(text)
    }
    pub fn set_text<S: Into<Cow<'static, str>>>(&self, text: S) -> windows::Result<()> {
        root_panel_with(|panel| self.set_text_at(panel, text))
    }
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
    #[builder(default = "{Colors::Black().unwrap()}")]
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
        let id = get_next_id();
        let visual = compositor().CreateSpriteVisual()?;
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
        let size = self.visual.Size()?;
        if size.X > 0. && size.Y > 0. {
            let surface = composition_graphics_device().CreateDrawingSurface(
                Size {
                    Width: size.X,
                    Height: size.Y,
                },
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                DirectXAlphaMode::Premultiplied,
            )?;

            let brush = compositor().CreateSurfaceBrush()?;
            brush.SetSurface(surface.clone())?;
            self.surface = Some(surface);
            self.visual.SetBrush(brush)?;
        }
        Ok(())
    }

    fn redraw_text(&self) -> windows::Result<()> {
        if let Some(ref surface) = self.surface {
            let ds = CanvasComposition::CreateDrawingSession(surface)?;
            ds.Clear(Colors::Transparent()?)?;

            let size = surface.Size()?;
            let text_format = CanvasTextFormat::new()?;
            text_format.SetFontFamily("Arial")?;
            text_format.SetFontSize(size.Height / self.params.font_scale)?;
            let text: String = self.params.text.clone().into();
            let text_layout = CanvasTextLayout::Create(
                canvas_device(),
                text,
                text_format,
                size.Width,
                size.Height,
            )?;
            text_layout.SetVerticalAlignment(CanvasVerticalAlignment::Center)?;
            text_layout.SetHorizontalAlignment(CanvasHorizontalAlignment::Center)?;
            let color = if self.params.enabled {
                self.params.color.clone()
            } else {
                Colors::Gray()?
            };

            ds.DrawTextLayoutAtCoordsWithColor(text_layout, 0., 0., color)
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

    fn on_resize(&mut self, size: &Vector2) -> windows::Result<()> {
        self.visual.SetSize(size)?;
        self.resize_surface()?;
        self.redraw_text()?;
        Ok(())
    }

    fn on_idle(&mut self) -> windows::Result<()> {
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

    fn on_init(&mut self) -> windows::Result<()> {
        self.on_resize(&self.visual().Parent()?.Size()?)
    }

    fn on_mouse_move(&mut self, _position: &Vector2) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: winit::event::MouseButton,
        _state: winit::event::ElementState,
    ) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(&mut self, _input: winit::event::KeyboardInput) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_panel_event(&mut self, _panel_event: &mut PanelEvent) -> windows::Result<()> {
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
