use std::collections::HashMap;

use bindings::{
    microsoft::graphics::canvas::text::CanvasHorizontalAlignment,
    microsoft::graphics::canvas::text::CanvasTextFormat,
    microsoft::graphics::canvas::text::CanvasTextLayout,
    microsoft::graphics::canvas::text::CanvasVerticalAlignment,
    microsoft::graphics::canvas::ui::composition::CanvasComposition,
    microsoft::graphics::canvas::CanvasDevice, windows::foundation::numerics::Vector2,
    windows::foundation::numerics::Vector3, windows::foundation::Size,
    windows::graphics::directx::DirectXAlphaMode, windows::graphics::directx::DirectXPixelFormat,
    windows::ui::composition::CompositionBorderMode,
    windows::ui::composition::CompositionGraphicsDevice,
    windows::ui::composition::CompositionShape, windows::ui::composition::Compositor,
    windows::ui::composition::ContainerVisual, windows::ui::composition::Visual,
    windows::ui::Color, windows::ui::ColorHelper, windows::ui::Colors,
};
use float_ord::FloatOrd;
use model::field::{Field, Origin, Side};
use panelgui::main_window::{Handle, Panel, PanelEventProxy, PanelHandle, PanelManager};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

const TILE_SIZE: Vector2 = Vector2 { x: 512., y: 512. };
const GAME_BOARD_MARGIN: Vector2 = Vector2 { x: 100.0, y: 100.0 };
const MIN_DRAG_MOUSE_MOVE: FloatOrd<f32> = FloatOrd(5.);

#[derive(PartialEq)]
pub enum GameFieldPanelEvent {
    Changed,
}

pub struct GameFieldPanel {
    id: usize,
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    root: ContainerVisual,
    game_board_visual: ContainerVisual,
    game_board_tiles: HashMap<(usize, usize), (Visual, u32)>,
    removed_tiles: Vec<Visual>,
    tile_shapes: HashMap<u32, CompositionShape>,
    tile_text_layouts: HashMap<u32, CanvasTextLayout>,
    field: Field,
    score: u32,
    mouse_pressed_pos: Option<Vector2>,
}

#[derive(Copy, Clone)]
pub struct GameFieldHandle {
    id: usize,
}

impl Handle for GameFieldHandle {
    fn id(&self) -> usize {
        self.id
    }
}

impl PanelHandle<GameFieldPanel, GameFieldPanelEvent> for GameFieldHandle {}

impl Panel for GameFieldPanel {
    fn id(&self) -> usize {
        self.id
    }
    fn visual(&self) -> ContainerVisual {
        self.root.clone()
    }
    fn on_resize(&mut self) -> winrt::Result<()> {
        self.visual().set_size(self.visual().parent()?.size()?)?;
        self.scale_game_board()
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn on_keyboard_input(
        &mut self,
        input: KeyboardInput,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        if input.state == ElementState::Pressed {
            if let Some(side) = match input.virtual_keycode {
                Some(VirtualKeyCode::Left) => Some(Side::Left),
                Some(VirtualKeyCode::Right) => Some(Side::Right),
                Some(VirtualKeyCode::Up) => Some(Side::Up),
                Some(VirtualKeyCode::Down) => Some(Side::Down),
                _ => None,
            } {
                self.swipe(side, proxy)?;
                return Ok(true);
            } else if input.virtual_keycode == Some(VirtualKeyCode::Back) {
                self.undo(proxy)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn on_mouse_input(
        &mut self,
        position: Vector2,
        _button: winit::event::MouseButton,
        state: ElementState,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        if state == ElementState::Pressed {
            self.mouse_pressed_pos = Some(position);
        } else if state == ElementState::Released {
            if let Some(prev_position) = self.mouse_pressed_pos.take() {
                let mut dx = position.x - prev_position.x;
                let mut dy = position.y - prev_position.y;
                let mut dx_abs = FloatOrd(dx.abs());
                let mut dy_abs = FloatOrd(dy.abs());
                if dx_abs < MIN_DRAG_MOUSE_MOVE && dy_abs < MIN_DRAG_MOUSE_MOVE {
                    let size = self.visual().size()?;
                    let cx = size.x / 2.;
                    let cy = size.y / 2.;
                    dx = position.x - cx;
                    dy = position.y - cy;
                    dx_abs = FloatOrd(dx.abs());
                    dy_abs = FloatOrd(dy.abs());
                }
                if dx_abs > dy_abs {
                    if dx.is_sign_positive() {
                        self.swipe(Side::Right, proxy)?;
                    } else {
                        self.swipe(Side::Left, proxy)?;
                    }
                } else {
                    if dy.is_sign_positive() {
                        self.swipe(Side::Down, proxy)?;
                    } else {
                        self.swipe(Side::Up, proxy)?;
                    }
                }
            }
        }
        Ok(true)
    }
}

impl GameFieldPanel {
    pub fn new(game_window: &mut PanelManager) -> winrt::Result<Self> {
        let compositor = game_window.compositor().clone();
        let root = compositor.create_sprite_visual()?;
        root.set_offset(Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })?;
        //        root.set_brush(compositor.create_color_brush_with_color(Colors::red()?)?)?;
        root.set_border_mode(CompositionBorderMode::Hard)?;

        let game_board_visual = compositor.create_container_visual()?;
        game_board_visual.set_relative_offset_adjustment(Vector3 {
            x: 0.5,
            y: 0.5,
            z: 0.0,
        })?;
        game_board_visual.set_anchor_point(Vector2 { x: 0.5, y: 0.5 })?;
        root.children()?.insert_at_top(&game_board_visual)?;

        //#[rustfmt::skip]
        //let array =
        //    Array2::from_shape_vec((4, 3), vec![2, 4, 4, 2, 2, 4, 0, 2, 2, 0, 0, 2]).unwrap();
        //let mut field = Field::from_array(array);

        let (field, score) = Self::reset_field_and_score();

        let mut game_field = Self {
            id: game_window.get_next_id(),
            compositor,
            canvas_device: game_window.canvas_device().clone(),
            composition_graphics_device: game_window.composition_graphics_device().clone(),
            root: root.into(),
            game_board_visual,
            game_board_tiles: HashMap::new(),
            removed_tiles: Vec::new(),
            tile_shapes: HashMap::new(),
            tile_text_layouts: HashMap::new(),
            field,
            score,
            mouse_pressed_pos: None,
        };
        game_field.animate_field()?; // TODO: separate 'new' and 'OnInit'
        Ok(game_field)
    }

    pub fn handle(&self) -> GameFieldHandle {
        GameFieldHandle { id: self.id }
    }

    pub fn get_score(&self) -> u32 {
        self.score
    }

    pub fn swipe(&mut self, side: Side, proxy: &PanelEventProxy) -> winrt::Result<()> {
        if self.field.can_swipe(side) {
            self.score += self.field.swipe(side);
            self.field.append_tile();
            self.field.append_tile();
            self.animate_field()?;
            proxy.send_panel_event(self.id, GameFieldPanelEvent::Changed)?;
        }
        Ok(())
    }

    pub fn undo(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        if self.field.can_undo() {
            self.score -= self.field.undo();
            self.animate_field()?;
            proxy.send_panel_event(self.id, GameFieldPanelEvent::Changed)?;
        }
        Ok(())
    }

    fn reset_field_and_score() -> (Field, u32) {
        let mut field = Field::new(4, 4);
        field.append_tile();
        field.append_tile();
        field.hold_all();
        (field, 0)
    }

    pub fn reset(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        let (field, score) = Self::reset_field_and_score();
        self.field = field;
        self.score = score;
        self.animate_field()?;
        proxy.send_panel_event(self.id, GameFieldPanelEvent::Changed)?;
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.field.can_undo()
    }

    fn scale_game_board(&mut self) -> winrt::Result<()> {
        let board_size = self.game_board_visual.size()?;
        let board_size = board_size + GAME_BOARD_MARGIN;

        let window_size = self.root.size()?;

        let window_ratio = window_size.x / window_size.y;
        let board_ratio = board_size.x / board_size.y;

        let scale_factor = if window_ratio > board_ratio {
            window_size.y / board_size.y
        } else {
            window_size.x / board_size.x
        };

        self.game_board_visual.set_scale(Vector3 {
            x: scale_factor,
            y: scale_factor,
            z: 1.0,
        })
    }

    pub fn get_tile_shape(&mut self, n: u32) -> winrt::Result<CompositionShape> {
        if let Some(shape) = self.tile_shapes.get(&n) {
            Ok(shape.clone())
        } else {
            let container_shape = self.compositor.create_container_shape()?;
            let round_rect_geometry = self.compositor.create_rounded_rectangle_geometry()?;
            round_rect_geometry.set_corner_radius(Vector2 {
                x: TILE_SIZE.x / 20.,
                y: TILE_SIZE.y / 20.,
            })?;
            let round_rect_size = TILE_SIZE / 1.1;
            round_rect_geometry.set_size(&round_rect_size)?;
            let brush = self
                .compositor
                .create_color_brush_with_color(Self::get_tile_color(n)?)?;
            let round_rect = self
                .compositor
                .create_sprite_shape_with_geometry(round_rect_geometry)?;
            round_rect.set_fill_brush(brush)?;
            round_rect.set_offset((TILE_SIZE - round_rect_size) / 2.)?;
            container_shape.shapes()?.append(round_rect)?;
            let shape: CompositionShape = container_shape.into();
            self.tile_shapes.insert(n, shape.clone());
            Ok(shape)
        }
    }

    pub fn get_tile_text_layout(&mut self, n: u32) -> winrt::Result<CanvasTextLayout> {
        if let Some(text_layout) = self.tile_text_layouts.get(&n) {
            Ok(text_layout.clone())
        } else {
            let text_string: String = n.to_string();
            let text_format = CanvasTextFormat::new()?;
            text_format.set_font_family("Arial")?;
            text_format.set_font_size(Self::get_tile_font_size(n))?;

            let text_layout = CanvasTextLayout::create(
                &self.canvas_device,
                text_string,
                text_format,
                TILE_SIZE.x,
                TILE_SIZE.y,
            )?;
            text_layout.set_vertical_alignment(CanvasVerticalAlignment::Center)?;
            text_layout.set_horizontal_alignment(CanvasHorizontalAlignment::Center)?;
            self.tile_text_layouts.insert(n, text_layout.clone());
            Ok(text_layout)
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
        self.game_board_visual
            .children()?
            .insert_at_top(&tile_visual)?;
        let visual: Visual = tile_visual.into();
        Self::animated_appear_tile(&visual)?;
        Ok(visual)
    }

    fn hold_tile_visual(&mut self, x: usize, y: usize, n: u32) -> winrt::Result<Visual> {
        if let Some((visual, visual_n)) = self.game_board_tiles.remove(&(x, y)) {
            if n == visual_n {
                Ok(visual)
            } else {
                self.game_board_visual.children()?.remove(visual)?;
                self.create_tile_visual(x, y, n)
            }
        } else {
            self.create_tile_visual(x, y, n)
        }
    }

    fn animated_move_tile(
        visual: &Visual,
        from_x: usize,
        from_y: usize,
        x: usize,
        y: usize,
    ) -> winrt::Result<()> {
        let compositor = visual.compositor()?;
        let animation = compositor.create_vector3_key_frame_animation()?;
        let animate_from = Vector3 {
            x: TILE_SIZE.x * from_x as f32,
            y: TILE_SIZE.y * from_y as f32,
            z: 0.,
        };
        let animate_to = Vector3 {
            x: TILE_SIZE.x * x as f32,
            y: TILE_SIZE.y * y as f32,
            z: 0.,
        };
        animation.insert_key_frame(0.0, animate_from)?;
        animation.insert_key_frame(1.0, animate_to)?;
        visual.start_animation("Offset", animation)?;
        Ok(())
    }

    fn animated_appear_tile(visual: &Visual) -> winrt::Result<()> {
        let compositor = visual.compositor()?;

        let animation = compositor.create_vector3_key_frame_animation()?;
        let animate_from = Vector3 {
            x: 0.,
            y: 0.,
            z: 0.,
        };
        let animate_to = Vector3 {
            x: 1.,
            y: 1.,
            z: 0.,
        };
        animation.insert_key_frame(0.0, animate_from)?;
        animation.insert_key_frame(1.0, animate_to)?;
        let size = visual.size()?;
        visual.set_center_point(Vector3 {
            x: size.x / 2.,
            y: size.y / 2.,
            z: 0.,
        })?;
        visual.start_animation("Scale", animation)?;
        Ok(())
    }

    fn move_tile_visual(
        &mut self,
        from_x: usize,
        from_y: usize,
        x: usize,
        y: usize,
        n: u32,
    ) -> winrt::Result<Visual> {
        if let Some((visual, visual_n)) = self.game_board_tiles.remove(&(from_x, from_y)) {
            Self::animated_move_tile(&visual, from_x, from_y, x, y)?;
            if n == visual_n {
                Ok(visual)
            } else {
                self.removed_tiles.push(visual);
                self.create_tile_visual(x, y, n)
            }
        } else {
            self.create_tile_visual(x, y, n)
        }
    }

    fn move_tile_visual_then_drop(
        &mut self,
        from_x: usize,
        from_y: usize,
        x: usize,
        y: usize,
    ) -> winrt::Result<()> {
        if let Some((visual, _)) = self.game_board_tiles.remove(&(from_x, from_y)) {
            Self::animated_move_tile(&visual, from_x, from_y, x, y)?;
            self.removed_tiles.push(visual);
        }
        Ok(())
    }

    fn merge_tile_visuals(
        &mut self,
        from_x1: usize,
        from_y1: usize,
        from_x2: usize,
        from_y2: usize,
        x: usize,
        y: usize,
        n: u32,
    ) -> winrt::Result<Visual> {
        self.move_tile_visual_then_drop(from_x2, from_y2, x, y)?;
        self.move_tile_visual(from_x1, from_y1, x, y, n)
    }

    fn garbage_collect_tiles(&mut self) -> winrt::Result<()> {
        while let Some(tile) = self.removed_tiles.pop() {
            self.game_board_visual.children()?.remove(tile)?;
        }
        Ok(())
    }

    fn animate_field(&mut self) -> winrt::Result<()> {
        self.game_board_visual.set_size(Vector2 {
            x: self.field.width() as f32 * TILE_SIZE.x,
            y: self.field.height() as f32 * TILE_SIZE.y,
        })?;
        self.scale_game_board()?;

        self.garbage_collect_tiles()?;
        let mut new_board_tiles = HashMap::new();
        for x in 0..self.field.width() {
            for y in 0..self.field.height() {
                if let Some(tile) = self.field.get(x, y) {
                    let n = tile.get_n();
                    let visual = match tile.get_origin() {
                        Origin::Appear => self.create_tile_visual(x, y, n),
                        Origin::Hold { .. } => self.hold_tile_visual(x, y, n),
                        Origin::Moved(from_x, from_y) => {
                            self.move_tile_visual(from_x, from_y, x, y, n)
                        }
                        Origin::Merged((from_x1, from_y1), (from_x2, from_y2)) => {
                            self.merge_tile_visuals(from_x1, from_y1, from_x2, from_y2, x, y, n)
                        }
                    }?;
                    let mut remove_key = None;
                    for (key, (tile, _)) in &self.game_board_tiles {
                        if *tile == visual {
                            remove_key = Some(*key);
                            break;
                        }
                    }
                    if let Some(key) = remove_key {
                        self.game_board_tiles.remove(&key);
                    }
                    new_board_tiles.insert((x, y), (visual, n));
                }
            }
        }
        for (tile, _) in self.game_board_tiles.values() {
            self.game_board_visual.children()?.remove(tile)?;
        }
        self.game_board_tiles = new_board_tiles;
        Ok(())
    }

    fn get_tile_color(n: u32) -> winrt::Result<Color> {
        match n {
            1 => Colors::gray(),
            2 => ColorHelper::from_argb(255, 238, 228, 218),
            4 => ColorHelper::from_argb(255, 237, 224, 200),
            8 => ColorHelper::from_argb(255, 242, 177, 121),
            16 => ColorHelper::from_argb(255, 242, 177, 121),
            32 => ColorHelper::from_argb(255, 246, 124, 95),
            64 => ColorHelper::from_argb(255, 246, 124, 95),
            128 => ColorHelper::from_argb(255, 237, 207, 114),
            256 => ColorHelper::from_argb(255, 237, 207, 97),
            512 => ColorHelper::from_argb(255, 237, 200, 80),
            1024 => ColorHelper::from_argb(255, 237, 197, 63),
            2048 => ColorHelper::from_argb(255, 237, 194, 46),
            _ => ColorHelper::from_argb(255, 60, 58, 60),
        }
    }

    fn get_tile_font_size(n: u32) -> f32 {
        if n < 1000 {
            256.
        } else if n < 10000 {
            180.
        } else if n < 100000 {
            150.
        } else {
            120.
        }
    }
}
