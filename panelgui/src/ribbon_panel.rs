use std::any::Any;

use bindings::windows::{
    foundation::numerics::{Vector2, Vector3},
    ui::composition::{Compositor, ContainerVisual},
};

use crate::main_window::{Panel, PanelEventProxy, PanelGlobals};

#[derive(PartialEq)]
pub enum RibbonOrientation {
    Horizontal,
    Vertical,
}

struct RibbonCell {
    panel: Box<dyn Panel>,
    container: ContainerVisual,
    ratio: f32,
}

pub struct Ribbon {
    id: usize,
    compositor: Compositor,
    orientation: RibbonOrientation,
    cells: Vec<RibbonCell>,
    ribbon: ContainerVisual,
    mouse_position: Option<Vector2>,
}

impl Ribbon {
    pub fn new(globals: &PanelGlobals, orientation: RibbonOrientation) -> winrt::Result<Self> {
        let compositor = globals.compositor().clone();
        let ribbon = compositor.create_container_visual()?;
        Ok(Self {
            id: globals.get_next_id(),
            compositor,
            orientation,
            cells: Vec::new(),
            ribbon,
            mouse_position: None,
        })
    }
    pub fn add_panel<P: Panel + 'static>(&mut self, panel: P, ratio: f32) -> winrt::Result<()> {
        let container = self.compositor.create_container_visual()?;
        container
            .children()?
            .insert_at_top(panel.visual().clone())?;
        self.ribbon
            .children()?
            .insert_at_bottom(container.clone())?;
        let cell = RibbonCell {
            panel: Box::new(panel),
            container,
            ratio,
        };
        self.cells.push(cell);
        self.resize_cells()?;
        Ok(())
    }

    fn resize_cells(&mut self) -> winrt::Result<()> {
        let size = self.ribbon.size()?;
        let hor = self.orientation == RibbonOrientation::Horizontal;
        let total = self.cells.iter().map(|c| c.ratio).sum::<f32>();
        let mut pos: f32 = 0.;
        for cell in &self.cells {
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
        Ok(())
    }

    fn get_cell_by_mouse_position<'a>(
        &'a mut self,
        position: &Vector2,
    ) -> winrt::Result<Option<(Vector2, &'a mut RibbonCell)>> {
        for p in &mut self.cells {
            let offset = p.container.offset()?;
            let size = p.container.size()?;
            let position = Vector2 {
                x: position.x - offset.x,
                y: position.y - offset.y,
            };
            if position.x >= 0. && position.x < size.x && position.y >= 0. && position.y < size.y {
                return Ok(Some((position, p)));
            }
        }
        Ok(None)
    }
}

impl Panel for Ribbon {
    fn id(&self) -> usize {
        self.id
    }
    fn visual(&self) -> bindings::windows::ui::composition::ContainerVisual {
        self.ribbon.clone()
    }

    fn on_resize(&mut self, size: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.ribbon.set_size(size)?;
        self.resize_cells()?;
        for p in &mut self.cells {
            p.panel.on_resize(&p.container.size()?, proxy)?;
        }
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
        for p in &mut self.cells {
            if p.panel.on_keyboard_input(input, proxy)? {
                return Ok(true);
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
