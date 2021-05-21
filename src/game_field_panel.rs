use lazy_static::lazy_static;
use panelgui::{
    canvas_device, composition_graphics_device, compositor, get_next_id, send_panel_event, Handle,
    Panel, PanelEvent, PanelHandle,
};
use std::collections::HashMap;

use bindings::{
    Microsoft::Graphics::Canvas::{
        CanvasDevice,
        Text::{
            CanvasHorizontalAlignment, CanvasTextFormat, CanvasTextLayout, CanvasVerticalAlignment,
        },
        UI::Composition::CanvasComposition,
    },
    Windows::{
        Foundation::{
            Numerics::{Vector2, Vector3},
            Size,
        },
        Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat},
        UI::{
            Color, ColorHelper, Colors,
            Composition::{
                CompositionBorderMode, CompositionGraphicsDevice, CompositionShape, Compositor,
                ContainerVisual, ShapeVisual, Visual,
            },
        },
    },
};
use float_ord::FloatOrd;
use model::field::{Field, Origin, Side};
use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode};

lazy_static! {
    static ref TILE_RECT_SIZE: Vector2 = Vector2 { X: 512., Y: 512. };
    static ref TILE_SIZE: Vector2 = &*TILE_RECT_SIZE / 1.1;
    static ref TILE_CORNER_RADIUS: Vector2 = Vector2 {
        X: TILE_SIZE.X / 20.,
        Y: TILE_SIZE.Y / 20.,
    };
    static ref TILE_OFFSET: Vector2 = (&*TILE_RECT_SIZE - &*TILE_SIZE) / 2.;
    static ref GAME_BOARD_MARGIN: Vector2 = Vector2 { X: 100.0, Y: 100.0 };
    static ref MIN_DRAG_MOUSE_MOVE: FloatOrd<f32> = FloatOrd(5.);
}

#[derive(PartialEq)]
pub enum GameFieldPanelEvent {
    Changed,
    UndoRequested,
    ResetRequested,
}

pub struct GameFieldPanel {
    id: usize,
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    root: ContainerVisual,
    game_board_container: ContainerVisual,
    game_board_tiles: HashMap<(usize, usize), (Visual, u32)>,
    removed_tiles: Vec<Visual>,
    tile_shapes: HashMap<u32, CompositionShape>,
    tile_text_layouts: HashMap<u32, CanvasTextLayout>,
    field: Field,
    score: u32,
    mouse_pos: Option<Vector2>,
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
    fn on_resize(&mut self, size: &Vector2) -> windows::Result<()> {
        self.visual().SetSize(size.clone())?;
        self.scale_game_board()
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn on_keyboard_input(&mut self, input: KeyboardInput) -> windows::Result<bool> {
        if input.state == ElementState::Pressed {
            if let Some(side) = match input.virtual_keycode {
                Some(VirtualKeyCode::Left) => Some(Side::Left),
                Some(VirtualKeyCode::Right) => Some(Side::Right),
                Some(VirtualKeyCode::Up) => Some(Side::Up),
                Some(VirtualKeyCode::Down) => Some(Side::Down),
                Some(VirtualKeyCode::A) => Some(Side::Left),
                Some(VirtualKeyCode::D) => Some(Side::Right),
                Some(VirtualKeyCode::W) => Some(Side::Up),
                Some(VirtualKeyCode::S) => Some(Side::Down),
                _ => None,
            } {
                self.swipe(side)?;
                return Ok(true);
            } else if input.virtual_keycode == Some(VirtualKeyCode::Back) {
                send_panel_event(self.id, GameFieldPanelEvent::UndoRequested)?;
                return Ok(true);
            } else if input.virtual_keycode == Some(VirtualKeyCode::R) {
                send_panel_event(self.id, GameFieldPanelEvent::ResetRequested)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn on_mouse_move(&mut self, position: &Vector2) -> windows::Result<()> {
        self.mouse_pos = Some(position.clone());
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
    ) -> windows::Result<bool> {
        let position = if let Some(ref posiition) = self.mouse_pos {
            posiition
        } else {
            return Ok(false);
        };
        if button != MouseButton::Left {
            return Ok(false);
        }

        if state == ElementState::Pressed {
            self.mouse_pressed_pos = Some(position.clone());
        } else if state == ElementState::Released {
            if let Some(prev_position) = self.mouse_pressed_pos.take() {
                let mut dx = position.X - prev_position.X;
                let mut dy = position.Y - prev_position.Y;
                let mut dx_abs = FloatOrd(dx.abs());
                let mut dy_abs = FloatOrd(dy.abs());
                if dx_abs < *MIN_DRAG_MOUSE_MOVE && dy_abs < *MIN_DRAG_MOUSE_MOVE {
                    let size = self.visual().Size()?;
                    let cx = size.X / 2.;
                    let cy = size.Y / 2.;
                    dx = position.X - cx;
                    dy = position.Y - cy;
                    dx_abs = FloatOrd(dx.abs());
                    dy_abs = FloatOrd(dy.abs());
                }
                if dx_abs > dy_abs {
                    if dx.is_sign_positive() {
                        self.swipe(Side::Right)?;
                    } else {
                        self.swipe(Side::Left)?;
                    }
                } else {
                    if dy.is_sign_positive() {
                        self.swipe(Side::Down)?;
                    } else {
                        self.swipe(Side::Up)?;
                    }
                }
            }
        }
        Ok(true)
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn std::any::Any> {
        if self.id == id {
            Some(self.as_any_mut())
        } else {
            None
        }
    }

    fn on_init(&mut self) -> windows::Result<()> {
        self.init_board()
    }

    fn on_idle(&mut self) -> windows::Result<()> {
        Ok(())
    }

    fn on_panel_event(&mut self, _panel_event: &mut PanelEvent) -> windows::Result<()> {
        Ok(())
    }
}

impl GameFieldPanel {
    pub fn new() -> windows::Result<Self> {
        let compositor = compositor().clone();
        let root = compositor.CreateSpriteVisual()?;
        root.SetOffset(Vector3 {
            X: 0.0,
            Y: 0.0,
            Z: 0.0,
        })?;

        root.SetBorderMode(CompositionBorderMode::Hard)?;

        let game_board_container = compositor.CreateContainerVisual()?;
        game_board_container.SetRelativeOffsetAdjustment(Vector3 {
            X: 0.5,
            Y: 0.5,
            Z: 0.0,
        })?;
        game_board_container.SetAnchorPoint(Vector2 { X: 0.5, Y: 0.5 })?;
        root.Children()?.InsertAtTop(&game_board_container)?;

        //#[rustfmt::skip]
        //let array =
        //    Array2::from_shape_vec((4, 3), vec![2, 4, 4, 2, 2, 4, 0, 2, 2, 0, 0, 2]).unwrap();
        //let mut field = Field::from_array(array);

        let (field, score) = Self::reset_field_and_score();

        Ok(Self {
            id: get_next_id(),
            compositor,
            canvas_device: canvas_device().clone(),
            composition_graphics_device: composition_graphics_device().clone(),
            root: root.into(),
            game_board_container,
            game_board_tiles: HashMap::new(),
            removed_tiles: Vec::new(),
            tile_shapes: HashMap::new(),
            tile_text_layouts: HashMap::new(),
            field,
            score,
            mouse_pos: None,
            mouse_pressed_pos: None,
        })
    }

    pub fn handle(&self) -> GameFieldHandle {
        GameFieldHandle { id: self.id }
    }

    pub fn get_score(&self) -> u32 {
        self.score
    }

    pub fn swipe(&mut self, side: Side) -> windows::Result<()> {
        if self.field.can_swipe(side) {
            self.score += self.field.swipe(side);
            self.field.append_tile();
            self.field.append_tile();
            self.animate_board()?;
            send_panel_event(self.id, GameFieldPanelEvent::Changed)?;
        }
        Ok(())
    }

    pub fn undo(&mut self) -> windows::Result<()> {
        if self.field.can_undo() {
            self.score -= self.field.undo();
            self.animate_board()?;
            self.field.hold_all(); // do not allow undo undo
            send_panel_event(self.id, GameFieldPanelEvent::Changed)?;
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

    pub fn reset(&mut self) -> windows::Result<()> {
        let (field, score) = Self::reset_field_and_score();
        self.field = field;
        self.score = score;
        self.animate_board()?;
        send_panel_event(self.id, GameFieldPanelEvent::Changed)?;
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.field.can_undo()
    }

    pub fn create_tile_shape(&self, color: Color) -> windows::Result<CompositionShape> {
        let round_rect_geometry = self.compositor.CreateRoundedRectangleGeometry()?;
        round_rect_geometry.SetCornerRadius(&*TILE_CORNER_RADIUS)?;
        round_rect_geometry.SetSize(&*TILE_SIZE)?;
        let brush = self.compositor.CreateColorBrushWithColor(color)?;
        let round_rect = self
            .compositor
            .CreateSpriteShapeWithGeometry(round_rect_geometry)?;
        round_rect.SetFillBrush(brush)?;
        round_rect.SetOffset(&*TILE_OFFSET)?;
        let shape: CompositionShape = round_rect.into();
        Ok(shape)
    }

    pub fn create_background_visual(&self) -> windows::Result<ShapeVisual> {
        let background_rect_geometry = self.compositor.CreateRoundedRectangleGeometry()?;
        background_rect_geometry.SetCornerRadius(&*TILE_CORNER_RADIUS)?;
        background_rect_geometry.SetSize(self.get_board_size())?;
        let brush = self
            .compositor
            .CreateColorBrushWithColor(Colors::DimGray()?)?;
        let background_rect = self
            .compositor
            .CreateSpriteShapeWithGeometry(background_rect_geometry)?;
        background_rect.SetFillBrush(brush)?;
        background_rect.SetOffset(Vector2 { X: 0., Y: 0. })?;
        let background = self.compositor.CreateShapeVisual()?;
        background.SetSize(self.get_board_size())?;
        background.Shapes()?.Append(background_rect)?;
        for x in 0..self.field.width() {
            for y in 0..self.field.height() {
                let shape = self.create_tile_shape(Colors::Gray()?)?;
                let mut offset = shape.Offset()?;
                offset.X += TILE_RECT_SIZE.X * x as f32 + TILE_OFFSET.X;
                offset.Y += TILE_RECT_SIZE.Y * y as f32 + TILE_OFFSET.Y;
                shape.SetOffset(offset)?;
                background.Shapes()?.Append(shape)?;
            }
        }
        Ok(background)
    }

    fn scale_game_board(&mut self) -> windows::Result<()> {
        let board_size = self.game_board_container.Size()?;
        let board_size = board_size + &*GAME_BOARD_MARGIN;

        let window_size = self.root.Size()?;

        let window_ratio = window_size.X / window_size.Y;
        let board_ratio = board_size.X / board_size.Y;

        let scale_factor = if window_ratio > board_ratio {
            window_size.Y / board_size.Y
        } else {
            window_size.X / board_size.X
        };

        self.game_board_container.SetScale(Vector3 {
            X: scale_factor,
            Y: scale_factor,
            Z: 1.0,
        })
    }

    pub fn get_tile_shape(&mut self, n: u32) -> windows::Result<CompositionShape> {
        if let Some(shape) = self.tile_shapes.get(&n) {
            Ok(shape.clone())
        } else {
            let shape = self.create_tile_shape(Self::get_tile_color(n)?)?;
            self.tile_shapes.insert(n, shape.clone());
            Ok(shape)
        }
    }

    pub fn get_tile_text_layout(&mut self, n: u32) -> windows::Result<CanvasTextLayout> {
        if let Some(text_layout) = self.tile_text_layouts.get(&n) {
            Ok(text_layout.clone())
        } else {
            let text_string: String = n.to_string();
            let text_format = CanvasTextFormat::new()?;
            text_format.SetFontFamily("Arial")?;
            text_format.SetFontSize(Self::get_tile_font_size(n))?;

            let text_layout = CanvasTextLayout::Create(
                &self.canvas_device,
                text_string,
                text_format,
                TILE_RECT_SIZE.X,
                TILE_RECT_SIZE.Y,
            )?;
            text_layout.SetVerticalAlignment(CanvasVerticalAlignment::Center)?;
            text_layout.SetHorizontalAlignment(CanvasHorizontalAlignment::Center)?;
            self.tile_text_layouts.insert(n, text_layout.clone());
            Ok(text_layout)
        }
    }

    fn create_tile_visual(&mut self, x: usize, y: usize, n: u32) -> windows::Result<Visual> {
        let surface = self.composition_graphics_device.CreateDrawingSurface(
            Size {
                Width: TILE_RECT_SIZE.X,
                Height: TILE_RECT_SIZE.Y,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?;
        let ds = CanvasComposition::CreateDrawingSession(&surface)?;
        ds.Clear(Colors::Transparent()?)?;

        ds.DrawTextLayoutAtCoordsWithColor(
            self.get_tile_text_layout(n)?,
            0.,
            0.,
            Self::get_tile_font_color(n)?,
        )?;

        let brush = self.compositor.CreateSurfaceBrush()?;
        brush.SetSurface(surface)?;
        let number = self.compositor.CreateSpriteVisual()?;
        number.SetBrush(brush)?;
        number.SetSize(&*TILE_RECT_SIZE)?;

        let tile_box = self.compositor.CreateShapeVisual()?;
        tile_box.SetSize(&*TILE_RECT_SIZE)?;
        let shape = self.get_tile_shape(n)?;
        tile_box.Shapes()?.Append(shape)?;

        let tile_visual = self.compositor.CreateContainerVisual()?;
        tile_visual.SetSize(&*TILE_RECT_SIZE)?;
        tile_visual.Children()?.InsertAtTop(tile_box)?;
        tile_visual.Children()?.InsertAtTop(number)?;

        tile_visual.SetOffset(Vector3 {
            X: TILE_RECT_SIZE.X * x as f32 + TILE_OFFSET.X,
            Y: TILE_RECT_SIZE.Y * y as f32 + TILE_OFFSET.Y,
            Z: 0.,
        })?;
        self.game_board_container
            .Children()?
            .InsertAtTop(&tile_visual)?;
        let visual: Visual = tile_visual.into();
        Self::animated_appear_tile(&visual)?;
        Ok(visual)
    }

    fn hold_tile_visual(&mut self, x: usize, y: usize, n: u32) -> windows::Result<Visual> {
        if let Some((visual, visual_n)) = self.game_board_tiles.remove(&(x, y)) {
            if n == visual_n {
                Ok(visual)
            } else {
                self.game_board_container.Children()?.Remove(visual)?;
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
    ) -> windows::Result<()> {
        let compositor = visual.Compositor()?;
        let animation = compositor.CreateVector3KeyFrameAnimation()?;
        let animate_from = Vector3 {
            X: TILE_RECT_SIZE.X * from_x as f32 + TILE_OFFSET.X,
            Y: TILE_RECT_SIZE.Y * from_y as f32 + TILE_OFFSET.Y,
            Z: 0.,
        };
        let animate_to = Vector3 {
            X: TILE_RECT_SIZE.X * x as f32 + TILE_OFFSET.X,
            Y: TILE_RECT_SIZE.Y * y as f32 + TILE_OFFSET.Y,
            Z: 0.,
        };
        animation.InsertKeyFrame(0.0, animate_from)?;
        animation.InsertKeyFrame(1.0, animate_to)?;
        visual.StartAnimation("Offset", animation)?;
        Ok(())
    }

    fn animated_appear_tile(visual: &Visual) -> windows::Result<()> {
        let compositor = visual.Compositor()?;

        let animation = compositor.CreateVector3KeyFrameAnimation()?;
        let animate_from = Vector3 {
            X: 0.,
            Y: 0.,
            Z: 0.,
        };
        let animate_to = Vector3 {
            X: 1.,
            Y: 1.,
            Z: 0.,
        };
        animation.InsertKeyFrame(0.0, animate_from)?;
        animation.InsertKeyFrame(1.0, animate_to)?;
        let size = visual.Size()?;
        visual.SetCenterPoint(Vector3 {
            X: size.X / 2.,
            Y: size.Y / 2.,
            Z: 0.,
        })?;
        visual.StartAnimation("Scale", animation)?;
        Ok(())
    }

    fn move_tile_visual(
        &mut self,
        from_x: usize,
        from_y: usize,
        x: usize,
        y: usize,
        n: u32,
    ) -> windows::Result<Visual> {
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
    ) -> windows::Result<()> {
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
    ) -> windows::Result<Visual> {
        self.move_tile_visual_then_drop(from_x2, from_y2, x, y)?;
        self.move_tile_visual(from_x1, from_y1, x, y, n)
    }

    fn garbage_collect_tiles(&mut self) -> windows::Result<()> {
        while let Some(tile) = self.removed_tiles.pop() {
            self.game_board_container.Children()?.Remove(tile)?;
        }
        Ok(())
    }

    fn get_board_size(&self) -> Vector2 {
        Vector2 {
            X: self.field.width() as f32 * TILE_RECT_SIZE.X,
            Y: self.field.height() as f32 * TILE_RECT_SIZE.Y,
        } + &*TILE_OFFSET * 2.
    }

    fn init_board(&mut self) -> windows::Result<()> {
        self.game_board_container.SetSize(self.get_board_size())?;
        self.game_board_container.Children()?.RemoveAll()?;
        self.game_board_container
            .Children()?
            .InsertAtBottom(self.create_background_visual()?)?;
        self.scale_game_board()?;
        self.animate_board()
    }

    fn animate_board(&mut self) -> windows::Result<()> {
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
            self.game_board_container.Children()?.Remove(tile)?;
        }
        self.game_board_tiles = new_board_tiles;
        Ok(())
    }

    fn get_tile_color(n: u32) -> windows::Result<Color> {
        match n {
            1 => Colors::Gray(),
            2 => ColorHelper::FromArgb(255, 238, 228, 218),
            4 => ColorHelper::FromArgb(255, 237, 224, 200),
            8 => ColorHelper::FromArgb(255, 242, 177, 121),
            16 => ColorHelper::FromArgb(255, 242, 177, 121),
            32 => ColorHelper::FromArgb(255, 246, 124, 95),
            64 => ColorHelper::FromArgb(255, 246, 124, 95),
            128 => ColorHelper::FromArgb(255, 237, 207, 114),
            256 => ColorHelper::FromArgb(255, 237, 207, 97),
            512 => ColorHelper::FromArgb(255, 237, 200, 80),
            1024 => ColorHelper::FromArgb(255, 237, 197, 63),
            2048 => ColorHelper::FromArgb(255, 237, 194, 46),
            _ => ColorHelper::FromArgb(255, 60, 58, 60),
        }
    }

    fn get_tile_font_color(n: u32) -> windows::Result<Color> {
        if n < 8 {
            Colors::DimGray()
        } else {
            Colors::WhiteSmoke()
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
