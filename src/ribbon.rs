use bindings::windows::{
    foundation::numerics::Vector3,
    ui::composition::{Compositor, ContainerVisual},
};

use crate::game_window::{GameWindow, Panel};

#[derive(PartialEq)]
pub enum RibbonOrientation {
    Horizontal,
    Vertical,
}

struct RibbonCell {
    panel: Box<dyn Panel>,
    container: ContainerVisual,
}

pub struct Ribbon {
    compositor: Compositor,
    orientation: RibbonOrientation,
    cells: Vec<RibbonCell>,
    ribbon: ContainerVisual,
}

impl Ribbon {
    pub fn new(game_window: &GameWindow, orientation: RibbonOrientation) -> winrt::Result<Self> {
        let compositor = game_window.compositor().clone();
        let ribbon = compositor.create_container_visual()?;
        Ok(Self {
            compositor,
            orientation,
            cells: Vec::new(),
            ribbon,
        })
    }
    pub fn add_panel<P: Panel + 'static>(&mut self, panel: P) -> winrt::Result<()> {
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
        };
        self.cells.push(cell);
        self.resize_cells()?;
        Ok(())
    }

    fn resize_cells(&mut self) -> winrt::Result<()> {
        let mut size = self.ribbon.size()?;
        let count = self.cells.len();
        let hor = self.orientation == RibbonOrientation::Horizontal;
        if hor {
            size.x /= count as f32
        } else {
            size.y /= count as f32;
        };

        for n in 0..count {
            let cell = self.cells.get(n).unwrap();
            cell.container.set_size(&size)?;
            cell.container.set_offset(if hor {
                Vector3 {
                    x: (n as f32) * size.x,
                    y: 0.,
                    z: 0.,
                }
            } else {
                Vector3 {
                    x: 0.,
                    y: (n as f32) * size.y,
                    z: 0.,
                }
            })?;
        }
        for p in &mut self.cells {
            p.panel.on_resize()?;
        }
        Ok(())
    }
}

impl Panel for Ribbon {
    fn visual(&self) -> bindings::windows::ui::composition::ContainerVisual {
        self.ribbon.clone()
    }

    fn on_resize(&mut self) -> winrt::Result<()> {
        self.ribbon.set_size(self.ribbon.parent()?.size()?)?;
        self.resize_cells()?;
        Ok(())
    }

    fn on_idle(
        &mut self,
        proxy: &winit::event_loop::EventLoopProxy<Box<dyn std::any::Any>>,
    ) -> winrt::Result<()> {
        for p in &mut self.cells {
            p.panel.on_idle(proxy)?;
        }
        Ok(())
    }

    fn on_user_event(
        &mut self,
        evt: Box<dyn std::any::Any>,
        proxy: &winit::event_loop::EventLoopProxy<Box<dyn std::any::Any>>,
    ) -> winrt::Result<Option<Box<dyn std::any::Any>>> {
        let mut evt = Some(evt);
        for p in &mut self.cells {
            if let Some(e) = p.panel.on_user_event(evt.unwrap(), proxy)? {
                evt = Some(e);
            } else {
                evt = None;
                break;
            }
        }
        Ok(evt)
    }
}
