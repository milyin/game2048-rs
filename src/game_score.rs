use bindings::{
    microsoft::graphics::canvas::CanvasDevice, windows::ui::composition::CompositionGraphicsDevice,
    windows::ui::composition::Compositor, windows::ui::composition::ContainerVisual,
};

use crate::game_window::{GameWindow, Panel};

struct GameScore {
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    root: ContainerVisual,
    score: usize,
}

impl Panel for GameWindow {
    fn visual(&self) -> bindings::windows::ui::composition::ContainerVisual {
        self.root.clone()
    }
}

impl GameScore {
    fn new(game_window: &mut GameWindow) -> winrt::Result<Self> {
        let compositor = game_window.compositor().clone();
        let root = compositor.create_sprite_visual()?;
        let canvas_device = game_window.canvas_device().clone();
        let composition_graphics_device = game_window.composition_graphics_device().clone();
        Ok(Self {
            compositor,
            canvas_device,
            composition_graphics_device,
            root: root.into(),
            score: 0,
        })
    }
}
