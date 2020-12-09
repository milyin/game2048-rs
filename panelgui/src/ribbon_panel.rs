use std::any::Any;

use bindings::windows::{
    foundation::numerics::{Vector2, Vector3},
    ui::composition::ContainerVisual,
};

use crate::main_window::{globals, winrt_error, Panel, PanelEventProxy};

#[derive(PartialEq, Copy, Clone)]
pub enum RibbonOrientation {
    Horizontal,
    Vertical,
    Stack,
}

struct RibbonCell {
    panel: Box<dyn Panel>,
    container: ContainerVisual,
    ratio: f32,
    content_ratio: Vector2,
}

#[derive(Builder)]
#[builder(build_fn(private, name = "build_default"), setter(into))]
pub struct RibbonParams {
    #[builder(default = "RibbonOrientation::Stack")]
    orientation: RibbonOrientation,
}

impl RibbonParamsBuilder {
    pub fn build(&self) -> winrt::Result<RibbonPanel> {
        match self.build_default() {
            Ok(settings) => Ok(RibbonPanel::new(settings)?),
            Err(e) => Err(winrt_error(e)()),
        }
    }
}

pub struct RibbonPanel {
    id: usize,
    params: RibbonParams,
    cells: Vec<RibbonCell>,
    visual: ContainerVisual,
    mouse_position: Option<Vector2>,
}

impl RibbonPanel {
    pub fn new(params: RibbonParams) -> winrt::Result<Self> {
        let id = globals().get_next_id();
        let visual = globals().compositor().create_container_visual()?;
        Ok(Self {
            id,
            params,
            cells: Vec::new(),
            visual,
            mouse_position: None,
        })
    }
    pub fn push_panel<P: Panel + 'static>(&mut self, panel: P, ratio: f32) -> winrt::Result<()> {
        self.push_panel_sized(panel, ratio, Vector2 { x: 1., y: 1. })
    }
    pub fn push_panel_sized<P: Panel + 'static>(
        &mut self,
        panel: P,
        ratio: f32,
        content_ratio: Vector2,
    ) -> winrt::Result<()> {
        let container = globals().compositor().create_container_visual()?;
        container
            .children()?
            .insert_at_top(panel.visual().clone())?;
        self.visual.children()?.insert_at_top(container.clone())?;
        let cell = RibbonCell {
            panel: Box::new(panel),
            container,
            ratio,
            content_ratio,
        };
        self.cells.push(cell);
        self.resize_cells()?;
        Ok(())
    }
    pub fn pop_panel(&mut self) -> winrt::Result<Box<dyn Panel>> {
        if let Some(cell) = self.cells.pop() {
            self.visual.children()?.remove(cell.container)?;
            self.resize_cells()?;
            Ok(cell.panel)
        } else {
            Err(winrt_error("Ribbon is empty")())
        }
    }

    fn resize_cells(&mut self) -> winrt::Result<()> {
        let size = self.visual.size()?;
        let total = self.cells.iter().map(|c| c.ratio).sum::<f32>();
        let mut pos: f32 = 0.;
        for cell in &self.cells {
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
        Ok(())
    }

    pub fn adjust_cells(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        for p in &mut self.cells {
            p.panel.on_resize(&p.container.size()?, proxy)?;
        }
        Ok(())
    }

    fn get_cell_by_mouse_position<'a>(
        &'a mut self,
        position: &Vector2,
    ) -> winrt::Result<Option<(Vector2, &'a mut RibbonCell)>> {
        // Scan in reverse order and exit immediately on topmost cell when in Stack mode
        for p in &mut self.cells.iter_mut().rev() {
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
        self.resize_cells()?;
        self.adjust_cells(proxy)?;
        Ok(())
    }

    fn on_idle(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        for p in &mut self.cells {
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
            for p in &mut self.cells {
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
        for p in &mut self.cells.iter_mut().rev() {
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
        for p in &mut self.cells {
            p.panel.on_init(proxy)?;
        }
        Ok(())
    }

    fn on_panel_event(
        &mut self,
        panel_event: &mut crate::main_window::PanelEvent,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        for p in &mut self.cells {
            p.panel.on_panel_event(panel_event, proxy)?;
        }
        Ok(())
    }
}
