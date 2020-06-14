mod interop;
mod numerics;
mod window_target;
use ndarray::Array2;

use interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType};
use winit::{
    event::{ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use bindings::windows::foundation::numerics::Vector2;
use bindings::windows::foundation::numerics::Vector3;
use bindings::windows::ui::composition::CompositionBorderMode;
use bindings::windows::ui::composition::CompositionShape;
use bindings::windows::ui::composition::Compositor;
use bindings::windows::ui::composition::ContainerVisual;
use bindings::windows::ui::composition::ShapeVisual;
use bindings::windows::ui::composition::Visual;
use bindings::windows::ui::Color;
use bindings::windows::ui::ColorHelper;
use bindings::windows::ui::Colors;

use crate::window_target::CompositionDesktopWindowTargetSource;

use model::field::Field;
use model::field::Side::Down;
use model::field::Side::Left;
use model::field::Side::Right;
use model::field::Side::Up;
use std::collections::HashMap;

const tile_size: Vector2 = Vector2 { x: 100., y: 100. };
const game_board_margin: Vector2 = Vector2 { x: 100.0, y: 100.0 };

pub struct Game {
    root: Visual,
    compositor: Compositor,
    field: Field,
    game_board: ContainerVisual,
    game_board_tiles: HashMap<(usize, usize), ShapeVisual>,
    tile_shapes: HashMap<u32, CompositionShape>,
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

        //#[rustfmt::skip]
        //let array =
        //    Array2::from_shape_vec((4, 3), vec![2, 4, 4, 2, 2, 4, 0, 2, 2, 0, 0, 2]).unwrap();
        //let mut field = Field::from_array(array);

        let mut field = Field::new(16, 16);
        field.random_fill();

        let mut result = Self {
            root: root.into(),
            compositor,
            field,
            game_board,
            game_board_tiles: HashMap::new(),
            tile_shapes: HashMap::new(),
        };

        result.init_game_board();

        Ok(result)
    }

    fn on_left_pressed(&mut self) {
        self.field.swipe(Left);
        self.field.append_tile();
        self.init_game_board();
        dbg!(&self.field);
    }

    fn on_right_pressed(&mut self) {
        self.field.swipe(Right);
        self.field.append_tile();
        self.init_game_board();
        dbg!(&self.field);
    }

    fn on_up_pressed(&mut self) {
        self.field.swipe(Up);
        self.field.append_tile();
        self.init_game_board();
        dbg!(&self.field);
    }

    fn on_down_pressed(&mut self) {
        self.field.swipe(Down);
        self.field.append_tile();
        self.init_game_board();
        dbg!(&self.field);
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
        rect.set_stroke_brush(&brush);
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
        let board_size = board_size + game_board_margin;

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

    fn init_game_board(&mut self) -> winrt::Result<()> {
        self.game_board.set_size(Vector2 {
            x: self.field.width() as f32 * tile_size.x,
            y: self.field.height() as f32 * tile_size.y,
        })?;
        self.game_board.children()?.remove_all();
        //        self.draw_game_board_border()?;
        self.game_board_tiles.clear();
        for x in 0..self.field.width() {
            for y in 0..self.field.height() {
                if let Some(tile) = self.field.get(x, y) {
                    let visual = self.compositor.create_shape_visual()?;
                    visual.set_size(tile_size)?;
                    let shape = self.get_tile_shape(tile.into())?;
                    visual.shapes()?.append(shape)?;
                    visual.set_offset(Vector3 {
                        x: tile_size.x * x as f32,
                        y: tile_size.y * y as f32,
                        z: 0.,
                    })?;

                    //                    let surface_brush = self.compositor.create_surface_brush()?;

                    //                  let surface = CompositionDrawingSurface::default();

                    self.game_board.children()?.insert_at_top(&visual);
                    self.game_board_tiles.insert((x, y), visual);
                }
            }
        }
        self.scale_game_board()
    }

    fn get_tile_color(n: u32) -> winrt::Result<Color> {
        match n {
            1 => Colors::gray(),
            2 => ColorHelper::from_argb(255, 238, 228, 218),
            4 => ColorHelper::from_argb(255, 237, 224, 200),
            _ => ColorHelper::from_argb(255, 60, 58, 60),
        }
    }

    pub fn get_tile_shape(&mut self, n: u32) -> winrt::Result<CompositionShape> {
        if let Some(shape) = self.tile_shapes.get(&n) {
            Ok(shape.clone())
        } else {
            let container_shape = self.compositor.create_container_shape()?;
            let round_rect_geometry = self.compositor.create_rounded_rectangle_geometry()?;
            round_rect_geometry.set_corner_radius(Vector2 { x: 5., y: 5. })?;
            let round_rect_size = tile_size / 1.1;
            round_rect_geometry.set_size(&round_rect_size)?;
            let brush = self
                .compositor
                .create_color_brush_with_color(Self::get_tile_color(n)?)?;
            let round_rect = self
                .compositor
                .create_sprite_shape_with_geometry(round_rect_geometry)?;
            round_rect.set_fill_brush(brush)?;
            round_rect.set_offset((tile_size - round_rect_size) / 2.)?;
            container_shape.shapes()?.append(round_rect)?;
            let shape: CompositionShape = container_shape.into();
            self.tile_shapes.insert(n, shape.clone());
            Ok(shape)
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
                let point = Vector2 {
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
                        Some(VirtualKeyCode::Left) => game.on_left_pressed(),
                        Some(VirtualKeyCode::Right) => game.on_right_pressed(),
                        Some(VirtualKeyCode::Up) => game.on_up_pressed(),
                        Some(VirtualKeyCode::Down) => game.on_down_pressed(),
                        _ => (),
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
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
