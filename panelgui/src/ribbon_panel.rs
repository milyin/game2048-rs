use std::any::Any;

use bindings::Windows::{
    Foundation::Numerics::{Vector2, Vector3},
    UI::Composition::ContainerVisual,
};

use crate::{
    globals::{compositor, get_next_id, winrt_error},
    panel::{EmptyPanel, Handle, Panel, PanelEvent, PanelHandle},
};

#[derive(PartialEq, Copy, Clone)]
pub enum RibbonOrientation {
    Horizontal,
    Vertical,
    Stack,
}
pub struct RibbonCell {
    panel: Box<dyn Panel>,
    container: ContainerVisual,
    limit: CellLimit,
    content_ratio: Vector2,
}

impl Default for RibbonCell {
    fn default() -> Self {
        RibbonCellParamsBuilder::default()
            .panel(EmptyPanel::new().unwrap())
            .create()
            .unwrap()
    }
}

impl RibbonCell {
    pub fn new(params: RibbonCellParams) -> windows::Result<Self> {
        let container = compositor().CreateContainerVisual()?;
        container
            .Children()?
            .InsertAtTop(params.panel.visual().clone())?;
        Ok(Self {
            panel: params.panel,
            container,
            limit: CellLimit {
                ratio: params.ratio,
                min_size: params.min_size,
                max_size: params.max_size,
            },
            content_ratio: params.content_ratio,
        })
    }
    pub fn panel(&self) -> &dyn Panel {
        &*self.panel
    }
}

#[derive(Builder)]
#[builder(pattern = "owned", setter(into))]
pub struct RibbonCellParams {
    #[builder(private, setter(name = "panel_private"))]
    panel: Box<dyn Panel>,
    #[builder(default = "{1.0}")]
    ratio: f32,
    #[builder(default = "{0.0}")]
    min_size: f32,
    #[builder(default = "{None}")]
    max_size: Option<f32>,
    #[builder(default = "{Vector2 { X: 1.0, Y: 1.0 }}")]
    content_ratio: Vector2,
}

impl RibbonCellParamsBuilder {
    pub fn create(self) -> windows::Result<RibbonCell> {
        match self.build() {
            Ok(params) => Ok(RibbonCell::new(params)?),
            Err(e) => Err(winrt_error(e)()),
        }
    }
    pub fn panel(self, panel: impl Panel + 'static) -> Self {
        let panel: Box<dyn Panel + 'static> = Box::new(panel);
        self.panel_private(panel)
    }
}

#[derive(Builder)]
#[builder(pattern = "owned", setter(into))]
pub struct RibbonParams {
    #[builder(default = "{RibbonOrientation::Stack}")]
    orientation: RibbonOrientation,
    #[builder(default = "{Vec::new()}")]
    cells: Vec<RibbonCell>,
}

impl RibbonParamsBuilder {
    pub fn create(self) -> windows::Result<RibbonPanel> {
        match self.build() {
            Ok(settings) => Ok(RibbonPanel::new(settings)?),
            Err(e) => Err(winrt_error(e)()),
        }
    }
    pub fn add_cell(mut self, cell: RibbonCell) -> Self {
        if self.cells.is_none() {
            self.cells = Some(Vec::new());
        }
        self.cells.as_mut().unwrap().push(cell);
        self
    }
    pub fn add_panel(self, panel: impl Panel + 'static) -> windows::Result<Self> {
        Ok(self.add_cell(RibbonCellParamsBuilder::default().panel(panel).create()?))
    }
    pub fn add_panel_with_ratio(
        self,
        panel: impl Panel + 'static,
        ratio: f32,
    ) -> windows::Result<Self> {
        Ok(self.add_cell(
            RibbonCellParamsBuilder::default()
                .panel(panel)
                .ratio(ratio)
                .create()?,
        ))
    }
}

pub struct RibbonPanel {
    handle: RibbonPanelHandle,
    params: RibbonParams,
    visual: ContainerVisual,
    mouse_position: Option<Vector2>,
}
#[derive(Copy, Clone, PartialEq)]
pub struct RibbonPanelHandle(usize);

impl RibbonPanelHandle {
    fn new() -> Self {
        Self { 0: get_next_id() }
    }
}

impl Handle for RibbonPanelHandle {
    fn id(&self) -> usize {
        self.0
    }
}

impl PanelHandle<RibbonPanel> for RibbonPanelHandle {}

#[derive(Copy, Clone, Debug)]
pub struct CellLimit {
    pub ratio: f32,
    pub min_size: f32,
    pub max_size: Option<f32>,
}

impl CellLimit {
    pub fn set_size(&mut self, size: f32) {
        self.min_size = size;
        self.max_size = Some(size);
    }
}

impl Default for CellLimit {
    fn default() -> Self {
        Self {
            ratio: 1.,
            min_size: 0.,
            max_size: None,
        }
    }
}

fn adjust_cells(limits: Vec<CellLimit>, mut target: f32) -> Vec<f32> {
    //dbg!(&target);
    let mut lock = Vec::with_capacity(limits.len());
    let mut result = Vec::with_capacity(limits.len());
    lock.resize(limits.len(), false);
    result.resize(limits.len(), 0.);

    let mut sum_ratio = limits
        .iter()
        .map(|c| {
            assert!(c.ratio > 0.);
            c.ratio
        })
        .sum::<f32>();
    loop {
        let mut new_target = target;
        let mut all_lock = true;
        for i in 0..limits.len() {
            if !lock[i] {
                let mut share = target * limits[i].ratio / sum_ratio;
                if share <= limits[i].min_size {
                    share = limits[i].min_size;
                    lock[i] = true;
                }
                if let Some(max_size) = limits[i].max_size {
                    if share > max_size {
                        share = max_size;
                        lock[i] = true;
                    }
                }
                if lock[i] {
                    new_target -= share;
                    sum_ratio -= limits[i].ratio;
                    lock[i] = true;
                } else {
                    all_lock = false;
                }
                result[i] = share;
            }
        }
        if all_lock || new_target == target {
            break;
        }
        target = if new_target > 0. { new_target } else { 0. };
    }
    //dbg!(&result);
    result
}

impl RibbonPanel {
    pub fn new(params: RibbonParams) -> windows::Result<Self> {
        let handle = RibbonPanelHandle::new();
        let visual = compositor().CreateContainerVisual()?;
        for p in &params.cells {
            visual.Children()?.InsertAtTop(p.container.clone())?;
        }
        Ok(Self {
            handle,
            params,
            visual,
            mouse_position: None,
        })
    }
    pub fn handle(&self) -> RibbonPanelHandle {
        self.handle.clone()
    }
    pub fn set_cell_at(&mut self, index: usize, cell: RibbonCell) -> windows::Result<()> {
        if index >= self.params.cells.len() {
            return Err(winrt_error("Bad cell index")());
        }
        self.visual
            .Children()?
            .InsertAtTop(cell.container.clone())?;
        self.params.cells.insert(index, cell);
        self.resize_cells()?;
        Ok(())
    }
    pub fn get_cell_limit(&self, index: usize) -> windows::Result<CellLimit> {
        if let Some(cell) = self.params.cells.get(index) {
            Ok(cell.limit)
        } else {
            Err(winrt_error("Wrong cell index")())
        }
    }
    pub fn set_cell_limit(&mut self, index: usize, limit: CellLimit) -> windows::Result<()> {
        if let Some(cell) = self.params.cells.get_mut(index) {
            cell.limit = limit;
            self.resize_cells()?;
            Ok(())
        } else {
            Err(winrt_error("Wrong cell index")())
        }
    }
    /*    pub fn get_mut_cell_at<'a>(
        &'a mut self,
        index: usize,
    ) -> windows::Result<Option<&'a mut RibbonCell>> {
        Ok(self.params.cells.get_mut(index))
    }*/
    pub fn push_cell(&mut self, cell: RibbonCell) -> windows::Result<()> {
        self.visual
            .Children()?
            .InsertAtTop(cell.container.clone())?;
        self.params.cells.push(cell);
        self.resize_cells()?;
        Ok(())
    }
    pub fn pop_cell(&mut self) -> windows::Result<RibbonCell> {
        if let Some(cell) = self.params.cells.pop() {
            self.visual.Children()?.Remove(&cell.container)?;
            self.resize_cells()?;
            Ok(cell)
        } else {
            Err(winrt_error("Ribbon is empty")())
        }
    }
    pub fn set_len(&mut self, new_len: usize) -> windows::Result<()> {
        self.params.cells.resize_with(new_len, Default::default);
        Ok(())
    }
    fn resize_cells(&mut self) -> windows::Result<()> {
        let size = self.visual.Size()?;
        if self.params.orientation == RibbonOrientation::Stack {
            for cell in &self.params.cells {
                let content_size = size.clone() * cell.content_ratio.clone();
                let content_offset = Vector3 {
                    X: (size.X - content_size.X) / 2.,
                    Y: (size.Y - content_size.Y) / 2.,
                    Z: 0.,
                };
                cell.container.SetSize(&content_size)?;
                cell.container.SetOffset(&content_offset)?;
            }
        } else {
            let limits = self
                .params
                .cells
                .iter()
                .map(|c| c.limit)
                .collect::<Vec<_>>();
            let hor = self.params.orientation == RibbonOrientation::Horizontal;
            let target = if hor { size.X } else { size.Y };
            let sizes = adjust_cells(limits, target);
            let mut pos: f32 = 0.;
            for i in 0..self.params.cells.len() {
                let size = if hor {
                    Vector2 {
                        X: sizes[i],
                        Y: size.Y,
                    }
                } else {
                    Vector2 {
                        X: size.X,
                        Y: sizes[i],
                    }
                };
                let cell = &mut self.params.cells[i];
                cell.container.SetSize(&size)?;
                cell.container.SetOffset(if hor {
                    Vector3 {
                        X: pos,
                        Y: 0.,
                        Z: 0.,
                    }
                } else {
                    Vector3 {
                        X: 0.,
                        Y: pos,
                        Z: 0.,
                    }
                })?;
                pos += sizes[i];
            }
        }
        for p in &mut self.params.cells {
            p.panel.on_resize(&p.container.Size()?)?;
        }
        Ok(())
    }
    fn get_cell_by_mouse_position<'a>(
        &'a mut self,
        position: &Vector2,
    ) -> windows::Result<Option<(Vector2, &'a mut RibbonCell)>> {
        // Scan in reverse order and exit immediately on topmost cell when in Stack mode
        for p in &mut self.params.cells.iter_mut().rev() {
            let offset = p.container.Offset()?;
            let size = p.container.Size()?;
            let position = Vector2 {
                X: position.X - offset.X,
                Y: position.Y - offset.Y,
            };
            if position.X >= 0. && position.X < size.X && position.Y >= 0. && position.Y < size.Y {
                return Ok(Some((position, p)));
            }
            if self.params.orientation == RibbonOrientation::Stack {
                return Ok(None);
            }
        }
        Ok(None)
    }
}

impl Panel for RibbonPanel {
    fn id(&self) -> usize {
        self.handle.id()
    }
    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }

    fn on_resize(&mut self, size: &Vector2) -> windows::Result<()> {
        self.visual.SetSize(size)?;
        self.resize_cells()?;
        Ok(())
    }

    fn on_idle(&mut self) -> windows::Result<()> {
        for p in &mut self.params.cells {
            p.panel.on_idle()?;
        }
        Ok(())
    }

    fn on_mouse_move(&mut self, position: &Vector2) -> windows::Result<()> {
        self.mouse_position = Some(position.clone());
        if let Some((position, cell)) = self.get_cell_by_mouse_position(position)? {
            cell.panel.on_mouse_move(&position)?;
        }
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
    ) -> windows::Result<bool> {
        if let Some(position) = self.mouse_position.clone() {
            if let Some((_, cell)) = self.get_cell_by_mouse_position(&position)? {
                return cell.panel.on_mouse_input(button, state);
            }
        }
        Ok(false)
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id() {
            Some(self.as_any_mut())
        } else {
            for p in &mut self.params.cells {
                if let Some(panel) = p.panel.find_panel(id) {
                    return Some(panel);
                }
            }
            None
        }
    }

    fn on_keyboard_input(&mut self, input: winit::event::KeyboardInput) -> windows::Result<bool> {
        for p in &mut self.params.cells.iter_mut().rev() {
            if self.params.orientation == RibbonOrientation::Stack {
                return p.panel.on_keyboard_input(input);
            } else {
                if p.panel.on_keyboard_input(input)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn on_init(&mut self) -> windows::Result<()> {
        self.on_resize(&self.visual().Parent()?.Size()?)?;
        for p in &mut self.params.cells {
            p.panel.on_init()?;
        }
        Ok(())
    }

    fn on_panel_event(&mut self, panel_event: &mut PanelEvent) -> windows::Result<()> {
        for p in &mut self.params.cells {
            p.panel.on_panel_event(panel_event)?;
        }
        Ok(())
    }
}
