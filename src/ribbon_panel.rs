use bindings::windows::{
    foundation::numerics::{Vector2, Vector3},
    ui::composition::{Compositor, ContainerVisual},
};

use crate::game_window::{GameWindow, Panel, PanelEventProxy};

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
}

impl Ribbon {
    pub fn new(
        game_window: &mut GameWindow,
        orientation: RibbonOrientation,
    ) -> winrt::Result<Self> {
        let compositor = game_window.compositor().clone();
        let ribbon = compositor.create_container_visual()?;
        Ok(Self {
            id: game_window.get_next_id(),
            compositor,
            orientation,
            cells: Vec::new(),
            ribbon,
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
        for p in &mut self.cells {
            p.panel.on_resize()?;
        }
        Ok(())
    }
}

impl Panel for Ribbon {
    fn id(&self) -> usize {
        self.id
    }
    fn visual(&self) -> bindings::windows::ui::composition::ContainerVisual {
        self.ribbon.clone()
    }

    fn on_resize(&mut self) -> winrt::Result<()> {
        self.ribbon.set_size(self.ribbon.parent()?.size()?)?;
        self.resize_cells()?;
        Ok(())
    }

    fn on_idle(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        for p in &mut self.cells {
            p.panel.on_idle(proxy)?;
        }
        Ok(())
    }
    fn translate_panel_event(
        &mut self,
        evt: crate::game_window::PanelEvent,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<Option<crate::game_window::PanelEvent>> {
        let mut evt = Some(evt);
        for p in &mut self.cells {
            if let Some(e) = p.panel.translate_panel_event(evt.unwrap(), proxy)? {
                evt = Some(e);
            } else {
                evt = None;
                break;
            }
        }
        Ok(evt)
    }
}
