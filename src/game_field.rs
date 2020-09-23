use std::collections::HashMap;

use bindings::microsoft::graphics::canvas::text::CanvasTextLayout;
use bindings::microsoft::graphics::canvas::ui::composition::CanvasComposition;
use bindings::microsoft::graphics::canvas::CanvasDevice;
use bindings::windows::ui::composition::Compositor;
use bindings::windows::ui::composition::Visual;
pub struct GameField {
    compositor: Compositor,
    tile_text_layouts: HashMap<u32, CanvasTextLayout>,
}

impl GameField {
    pub fn new(compositor: Compositor) -> Self {
        let canvas_device = CanvasDevice::get_shared_device()?;
        let composition_graphics_device =
            CanvasComposition::create_composition_graphics_device(&compositor, &canvas_device)?;
        Self {
            compositor,
            tile_text_layouts: HashMap::new(),
        }
    }
    fn create_tile_visual(&mut self, x: usize, y: usize, n: u32) -> winrt::Result<Visual> {
        let surface = self.composition_graphics_device.create_drawing_surface(
            Size {
                width: TILE_SIZE.x,
                height: TILE_SIZE.y,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?;
        let ds = CanvasComposition::create_drawing_session(&surface)?;
        ds.clear(Colors::transparent()?)?;

        ds.draw_text_layout_at_coords_with_color(
            self.get_tile_text_layout(n)?,
            0.,
            0.,
            Colors::white()?,
        )?;

        let brush = self.compositor.create_surface_brush()?;
        brush.set_surface(surface)?;
        let number = self.compositor.create_sprite_visual()?;
        number.set_brush(brush)?;
        number.set_size(TILE_SIZE)?;

        let tile_box = self.compositor.create_shape_visual()?;
        tile_box.set_size(TILE_SIZE)?;
        let shape = self.get_tile_shape(n)?;
        tile_box.shapes()?.append(shape)?;

        let tile_visual = self.compositor.create_container_visual()?;
        tile_visual.set_size(TILE_SIZE)?;
        tile_visual.children()?.insert_at_top(tile_box)?;
        tile_visual.children()?.insert_at_top(number)?;

        tile_visual.set_offset(Vector3 {
            x: TILE_SIZE.x * x as f32,
            y: TILE_SIZE.y * y as f32,
            z: 0.,
        })?;
        self.game_board.children()?.insert_at_top(&tile_visual)?;
        let visual: Visual = tile_visual.into();
        Self::animated_appear_tile(&visual)?;
        Ok(visual)
    }
}
