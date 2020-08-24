mod interop;
mod numerics;
mod window_target;

use interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType};
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use bindings::microsoft::graphics::canvas::text::CanvasHorizontalAlignment;
use bindings::microsoft::graphics::canvas::text::CanvasTextFormat;
use bindings::microsoft::graphics::canvas::text::CanvasTextLayout;
use bindings::microsoft::graphics::canvas::text::CanvasVerticalAlignment;
use bindings::microsoft::graphics::canvas::ui::composition::CanvasComposition;
use bindings::microsoft::graphics::canvas::CanvasDevice;
use bindings::windows::foundation::numerics::Vector2;
use bindings::windows::foundation::numerics::Vector3;
use bindings::windows::foundation::{Rect, Size};
use bindings::windows::graphics::directx::DirectXAlphaMode;
use bindings::windows::graphics::directx::DirectXPixelFormat;
use bindings::windows::ui::composition::CompositionBorderMode;
use bindings::windows::ui::composition::CompositionGraphicsDevice;
use bindings::windows::ui::composition::CompositionShape;
use bindings::windows::ui::composition::Compositor;
use bindings::windows::ui::composition::ContainerVisual;
use bindings::windows::ui::composition::Visual;
use bindings::windows::ui::Color;
use bindings::windows::ui::ColorHelper;
use bindings::windows::ui::Colors;

use crate::window_target::CompositionDesktopWindowTargetSource;

use model::field::Field;
use model::field::Origin;
use model::field::Side;
use model::field::Side::Down;
use model::field::Side::Left;
use model::field::Side::Right;
use model::field::Side::Up;
use std::collections::HashMap;

const TILE_SIZE: Vector2 = Vector2 { x: 512., y: 512. };
const GAME_BOARD_MARGIN: Vector2 = Vector2 { x: 100.0, y: 100.0 };

pub struct Game {
    root: Visual,
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    field: Field,
    game_score: ContainerVisual,
    game_board: ContainerVisual,
    game_board_tiles: HashMap<(usize, usize), Visual>,
    merged_tiles: Vec<Visual>,
    tile_shapes: HashMap<u32, CompositionShape>,
    tile_text_layouts: HashMap<u32, CanvasTextLayout>,
}

impl Game {
    pub fn new(parent_visual: &ContainerVisual, parent_size: &Vector2) -> winrt::Result<Self> {
        let compositor = parent_visual.compositor()?;
        let root = compositor.create_sprite_visual()?;
        root.set_relative_size_adjustment(Vector2 { x: 1.0, y: 1.0 })?;
        root.set_size(parent_size)?;
        root.set_brush(compositor.create_color_brush_with_color(Colors::white()?)?)?;
        root.set_border_mode(CompositionBorderMode::Hard)?;
        parent_visual.children()?.insert_at_top(&root)?;

        let game_board = compositor.create_container_visual()?;
        game_board.set_relative_offset_adjustment(Vector3 {
            x: 0.5,
            y: 0.5,
            z: 0.0,
        })?;
        game_board.set_anchor_point(Vector2 { x: 0.5, y: 0.5 })?;
        root.children()?.insert_at_top(&game_board)?;

        let game_score = compositor.create_container_visual()?;
        game_score.set_anchor_point(Vector2 { x: 0., y: 0. })?;
        root.children()?.insert_at_top(&game_score)?;

        //#[rustfmt::skip]
        //let array =
        //    Array2::from_shape_vec((4, 3), vec![2, 4, 4, 2, 2, 4, 0, 2, 2, 0, 0, 2]).unwrap();
        //let mut field = Field::from_array(array);

        let mut field = Field::new(4, 4);
        field.append_tile();
        field.append_tile();
        field.fix_position();

        let canvas_device = CanvasDevice::get_shared_device()?;
        let composition_graphics_device =
            CanvasComposition::create_composition_graphics_device(&compositor, &canvas_device)?;

        let mut result = Self {
            root: root.into(),
            compositor,
            canvas_device,
            composition_graphics_device,
            field,
            game_score,
            game_board,
            game_board_tiles: HashMap::new(),
            merged_tiles: Vec::new(),
            tile_shapes: HashMap::new(),
            tile_text_layouts: HashMap::new(),
        };

        result.init_game_board()?;
        //        result.draw_score()?;

        Ok(result)
    }

    fn undo(&mut self) {
        if self.field.can_undo() {
            self.field.undo();
            self.init_game_board().unwrap();
        }
    }

    fn swipe(&mut self, side: Side) {
        if self.field.can_swipe(side) {
            self.field.swipe(side);
            self.field.append_tile();
            self.field.append_tile();
            self.animate_game_board().unwrap();
        }
    }

    fn on_left(&mut self) {
        self.swipe(Left);
    }

    fn on_right(&mut self) {
        self.swipe(Right);
    }

    fn on_up(&mut self) {
        self.swipe(Up);
    }

    fn on_down(&mut self) {
        self.swipe(Down);
    }

    fn on_undo(&mut self) {
        self.undo();
    }

    pub fn draw_score(&mut self) -> winrt::Result<()> {
        let device = CanvasDevice::get_shared_device()?;
        let composition_graphics_device =
            CanvasComposition::create_composition_graphics_device(&self.compositor, device)?;
        let surface = composition_graphics_device.create_drawing_surface(
            Size {
                width: 256.,
                height: 256.,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?;
        let ds = CanvasComposition::create_drawing_session(&surface)?;
        ds.clear(Colors::transparent()?)?;
        ds.draw_rounded_rectangle_with_color(
            Rect {
                x: 10.,
                y: 10.,
                width: 100.,
                height: 100.,
            },
            10.,
            10.,
            Colors::red()?,
        )?;
        let brush = self.compositor.create_surface_brush()?;
        brush.set_surface(surface)?;
        let visual = self.compositor.create_sprite_visual()?;
        visual.set_brush(brush)?;
        visual.set_size(Vector2 { x: 200., y: 200. })?;

        self.game_score.children()?.remove_all()?;
        self.game_score.children()?.insert_at_top(&visual)?;

        Ok(())
    }

    pub fn draw_game_board_border(&mut self) -> winrt::Result<()> {
        let rect_geom = self.compositor.create_rectangle_geometry()?;
        let rect_size = Vector2 {
            x: self.game_board.size()?.x - 2.,
            y: self.game_board.size()?.y - 2.,
        };
        rect_geom.set_size(&rect_size)?;
        let rect = self
            .compositor
            .create_sprite_shape_with_geometry(rect_geom)?;
        let brush = self
            .compositor
            .create_color_brush_with_color(Colors::blue()?)?;
        rect.set_stroke_brush(&brush)?;
        let visual = self.compositor.create_shape_visual()?;
        visual.set_size(self.game_board.size()?)?;
        visual.shapes()?.append(rect)?;
        visual.set_offset(Vector3 {
            x: 0.,
            y: 0.,
            z: 0.,
        })?;
        self.game_board.children()?.insert_at_top(&visual)?;
        Ok(())
    }

    pub fn on_parent_size_changed(&mut self, new_size: &Vector2) -> winrt::Result<()> {
        self.root.set_size(new_size)?;
        self.scale_game_board()
    }

    fn scale_game_board(&mut self) -> winrt::Result<()> {
        let board_size = self.game_board.size()?;
        let board_size = board_size + GAME_BOARD_MARGIN;

        let window_size = self.root.size()?;

        let window_ratio = window_size.x / window_size.y;
        let board_ratio = board_size.x / board_size.y;

        let scale_factor = if window_ratio > board_ratio {
            window_size.y / board_size.y
        } else {
            window_size.x / board_size.x
        };

        self.game_board.set_scale(Vector3 {
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
        self.game_board.children()?.insert_at_top(&tile_visual)?;
        let visual: Visual = tile_visual.into();
        Self::animated_appear_tile(&visual)?;
        Ok(visual)
    }

    fn hold_tile_visual(&mut self, x: usize, y: usize) -> winrt::Result<Visual> {
        Ok(self
            .game_board_tiles
            .remove(&(x, y))
            .expect("hold emplty tile"))
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
    ) -> winrt::Result<Visual> {
        let visual = self
            .game_board_tiles
            .remove(&(from_x, from_y))
            .expect("move emplty tile");
        Self::animated_move_tile(&visual, from_x, from_y, x, y)?;
        Ok(visual)
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
        let visual1 = self
            .game_board_tiles
            .remove(&(from_x1, from_y1))
            .expect("merge emplty tile");
        let visual2 = self
            .game_board_tiles
            .remove(&(from_x2, from_y2))
            .expect("merge emplty tile");
        Self::animated_move_tile(&visual1, from_x1, from_y1, x, y)?;
        Self::animated_move_tile(&visual2, from_x2, from_y2, x, y)?;
        self.merged_tiles.push(visual1);
        self.merged_tiles.push(visual2);
        self.create_tile_visual(x, y, n)
    }

    fn init_game_board(&mut self) -> winrt::Result<()> {
        self.game_board.set_size(Vector2 {
            x: self.field.width() as f32 * TILE_SIZE.x,
            y: self.field.height() as f32 * TILE_SIZE.y,
        })?;
        self.remove_merged_tiles()?;
        self.game_board.children()?.remove_all()?;
        //self.draw_game_board_border()?;
        self.game_board_tiles.clear();
        for x in 0..self.field.width() {
            for y in 0..self.field.height() {
                if let Some(tile) = self.field.get(x, y) {
                    let n = tile.get_n();
                    let tile = self.create_tile_visual(x, y, n)?;
                    self.game_board_tiles.insert((x, y), tile);
                }
            }
        }
        self.scale_game_board()
    }

    fn remove_merged_tiles(&mut self) -> winrt::Result<()> {
        while let Some(tile) = self.merged_tiles.pop() {
            self.game_board.children()?.remove(tile)?;
        }
        Ok(())
    }

    fn animate_game_board(&mut self) -> winrt::Result<()> {
        self.remove_merged_tiles()?;
        let mut new_board_tiles = HashMap::new();
        for x in 0..self.field.width() {
            for y in 0..self.field.height() {
                if let Some(tile) = self.field.get(x, y) {
                    let n = tile.get_n();
                    match tile.get_origin() {
                        Origin::Appear => {
                            new_board_tiles.insert((x, y), self.create_tile_visual(x, y, n)?);
                        }
                        Origin::Hold { .. } => {
                            new_board_tiles.insert((x, y), self.hold_tile_visual(x, y)?);
                        }
                        Origin::Moved(from_x, from_y) => {
                            new_board_tiles
                                .insert((x, y), self.move_tile_visual(from_x, from_y, x, y)?);
                        }
                        Origin::Merged((from_x1, from_y1), (from_x2, from_y2)) => {
                            new_board_tiles.insert(
                                (x, y),
                                self.merge_tile_visuals(
                                    from_x1, from_y1, from_x2, from_y2, x, y, n,
                                )?,
                            );
                        }
                    }
                }
            }
        }
        assert!(self.game_board_tiles.is_empty());
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

fn run() -> winrt::Result<()> {
    ro_initialize(RoInitType::MultiThreaded)?;
    let _controller = create_dispatcher_queue_controller_for_current_thread()?;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_title("2048");

    let compositor = Compositor::new()?;
    let target = window.create_window_target(&compositor, false)?;

    let root = compositor.create_container_visual()?;
    root.set_relative_size_adjustment(Vector2 { x: 0.0, y: 0.0 })?;
    target.set_root(&root)?;

    let window_size = window.inner_size();
    let window_size = Vector2 {
        x: window_size.width as f32,
        y: window_size.height as f32,
    };

    let mut game = Game::new(&root, &window_size)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let size = Vector2 {
                    x: size.width as f32,
                    y: size.height as f32,
                };
                game.on_parent_size_changed(&size).unwrap();
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let _point = Vector2 {
                    x: position.x as f32,
                    y: position.y as f32,
                };
            }
            //Event::WindowEvent { event: WindowEvent::KeyboardInput {input: {VirtualKeycode: Some(VirtualKeycode::Escape)}} => {},
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        input,
                        is_synthetic: _,
                    },
                ..
            } => {
                if input.state == ElementState::Pressed {
                    match input.virtual_keycode {
                        Some(VirtualKeyCode::Left) => game.on_left(),
                        Some(VirtualKeyCode::Right) => game.on_right(),
                        Some(VirtualKeyCode::Up) => game.on_up(),
                        Some(VirtualKeyCode::Down) => game.on_down(),
                        Some(VirtualKeyCode::Back) => game.on_undo(),
                        _ => (),
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, .. },
                ..
            } => if state == ElementState::Pressed {},
            _ => (),
        }
    });
}

fn main() {
    let result = run();

    // We do this for nicer HRESULT printing when errors occur.
    if let Err(error) = result {
        error.code().unwrap();
    }
}
