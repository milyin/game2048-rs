use std::any::Any;

use bindings::windows::{
    foundation::numerics::{Vector2, Vector3},
    ui::composition::ContainerVisual,
};

use crate::main_window::{EmptyPanel, Panel, PanelEventProxy, globals, winrt_error};

#[derive(PartialEq, Copy, Clone)]
pub enum RibbonOrientation {
    Horizontal,
    Vertical,
    Stack,
}
pub struct RibbonCell {
    panel: Box<dyn Panel>,
    container: ContainerVisual,
    ratio: f32,
    content_ratio: Vector2,
}

impl Default for RibbonCell {
    fn default() -> Self {
       RibbonCellParamsBuilder::default().panel(EmptyPanel::new().unwrap()).create().unwrap()
    }
}

impl RibbonCell {
    pub fn new(params: RibbonCellParams) -> winrt::Result<Self> {
        let container = globals().compositor().create_container_visual()?;
        container
            .children()?
            .insert_at_top(params.panel.visual().clone())?;
        Ok(Self {
            panel: params.panel,
            container,
            ratio: params.ratio,
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
    #[builder(default = "{Vector2 { x: 1.0, y: 1.0 }}")]
    content_ratio: Vector2,
}

impl RibbonCellParamsBuilder {
    pub fn create(self) -> winrt::Result<RibbonCell> {
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
    pub fn create(self) -> winrt::Result<RibbonPanel> {
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
    pub fn add_panel(self, panel: impl Panel + 'static) -> winrt::Result<Self> {
        Ok(self.add_cell(RibbonCellParamsBuilder::default().panel(panel).create()?))
    }
    pub fn add_panel_with_ratio(self, panel: impl Panel + 'static, ratio: f32) -> winrt::Result<Self> {
        Ok(self.add_cell(RibbonCellParamsBuilder::default().panel(panel).ratio(ratio).create()?))
    }
}

pub struct RibbonPanel {
    id: usize,
    params: RibbonParams,
    visual: ContainerVisual,
    mouse_position: Option<Vector2>,
}

impl RibbonPanel {
    pub fn new(params: RibbonParams) -> winrt::Result<Self> {
        let id = globals().get_next_id();
        let visual = globals().compositor().create_container_visual()?;
        for p in &params.cells {
            visual.children()?.insert_at_top(p.container.clone())?;
        }
        Ok(Self {
            id,
            params,
            visual,
            mouse_position: None,
        })
    }
    pub fn set_cell_at(&mut self, index: usize, cell: RibbonCell, proxy: &PanelEventProxy) -> winrt::Result<()> {
        if index >= self.params.cells.len() {
            return Err(winrt_error("Bad cell index")())
        }
        self.visual.children()?.insert_at_top(cell.container.clone())?;
        self.params.cells.insert(index, cell);
        self.resize_cells(proxy)?;
        Ok(())
    }
    pub fn push_cell(&mut self, cell: RibbonCell, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.visual.children()?.insert_at_top(cell.container.clone())?;
        self.params.cells.push(cell);
        self.resize_cells(proxy)?;
        Ok(())
    }
    pub fn pop_cell(&mut self, proxy: &PanelEventProxy) -> winrt::Result<RibbonCell> {
        if let Some(cell) = self.params.cells.pop() {
            self.visual.children()?.remove(&cell.container)?;
            self.resize_cells(proxy)?;
            Ok(cell)
        } else {
            Err(winrt_error("Ribbon is empty")())
        }
    }
    pub fn set_len(&mut self, new_len: usize) -> winrt::Result<()> {
        self.params.cells.resize_with(new_len, Default::default);
        Ok(())
    }
    fn resize_cells(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        let size = self.visual.size()?;
        let total = self.params.cells.iter().map(|c| c.ratio).sum::<f32>();
        let mut pos: f32 = 0.;
        for cell in &self.params.cells {
            if self.params.orientation == RibbonOrientation::Stack {
                let content_size = size.clone() * cell.content_ratio.clone();
                let content_offset = Vector3 {
                    x: (size.x - content_size.x) / 2.,
                    y: (size.y - content_size.y) / 2.,
                    z: 0.,
                };
                cell.container.set_size(&content_size)?;
                cell.container.set_offset(&content_offset)?;
            } else {
                let hor = self.params.orientation == RibbonOrientation::Horizontal;
                let share = if hor { size.x } else { size.y } * cell.ratio / total;
                let size = if hor {
                    Vector2 {
                        x: share,
                        y: size.y,
                    }
                } else {
                    Vector2 {
                        x: size.x,
                        y: share,
                    }
                };
                cell.container.set_size(&size)?;
                cell.container.set_offset(if hor {
                    Vector3 {
                        x: pos,
                        y: 0.,
                        z: 0.,
                    }
                } else {
                    Vector3 {
                        x: 0.,
                        y: pos,
                        z: 0.,
                    }
                })?;
                pos += share;
            }
        }
        for p in &mut self.params.cells {
            p.panel.on_resize(&p.container.size()?, proxy)?;
        }
        Ok(())
    }
    fn get_cell_by_mouse_position<'a>(
        &'a mut self,
        position: &Vector2,
    ) -> winrt::Result<Option<(Vector2, &'a mut RibbonCell)>> {
        // Scan in reverse order and exit immediately on topmost cell when in Stack mode
        for p in &mut self.params.cells.iter_mut().rev() {
            let offset = p.container.offset()?;
            let size = p.container.size()?;
            let position = Vector2 {
                x: position.x - offset.x,
                y: position.y - offset.y,
            };
            if position.x >= 0. && position.x < size.x && position.y >= 0. && position.y < size.y {
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
        self.id
    }
    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }

    fn on_resize(&mut self, size: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.visual.set_size(size)?;
        self.resize_cells(proxy)?;
        Ok(())
    }

    fn on_idle(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        for p in &mut self.params.cells {
            p.panel.on_idle(proxy)?;
        }
        Ok(())
    }

    fn on_mouse_move(&mut self, position: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.mouse_position = Some(position.clone());
        if let Some((position, cell)) = self.get_cell_by_mouse_position(position)? {
            cell.panel.on_mouse_move(&position, proxy)?;
        }
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        if let Some(position) = self.mouse_position.clone() {
            if let Some((_, cell)) = self.get_cell_by_mouse_position(&position)? {
                return cell.panel.on_mouse_input(button, state, proxy);
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

    fn on_keyboard_input(
        &mut self,
        input: winit::event::KeyboardInput,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        for p in &mut self.params.cells.iter_mut().rev() {
            if self.params.orientation == RibbonOrientation::Stack {
                return p.panel.on_keyboard_input(input, proxy);
            } else {
                if p.panel.on_keyboard_input(input, proxy)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn on_init(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.on_resize(&self.visual().parent()?.size()?, proxy)?;
        for p in &mut self.params.cells {
            p.panel.on_init(proxy)?;
        }
        Ok(())
    }

    fn on_panel_event(
        &mut self,
        panel_event: &mut crate::main_window::PanelEvent,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        for p in &mut self.params.cells {
            p.panel.on_panel_event(panel_event, proxy)?;
        }
        Ok(())
    }
}
