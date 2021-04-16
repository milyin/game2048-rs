use bindings::windows::foundation::numerics::Vector2;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

use crate::globals::{run_until_stalled, winrt_error};
use crate::{globals::globals_with, panel::Panel};
pub struct MainWindow {}

impl MainWindow {
    pub fn run(self, panel: impl Panel + 'static) -> windows::Result<()> {
        let event_loop = globals_with(|globals| {
            globals
                .event_loop
                .take()
                .ok_or_else(winrt_error("Unexpected second run"))
        })?;
        let mut root_panel = globals_with(|globals| {
            globals
                .root_panel
                .take()
                .ok_or_else(winrt_error("Unexpected second run"))
        })?;
        let root_visual = globals_with(|globals| Ok(globals.root_visual.clone()))?;
        root_panel.push_cell(
            crate::ribbon_panel::RibbonCellParamsBuilder::default()
                .panel(panel)
                .create()?,
        )?;
        root_panel.on_init()?;

        event_loop.run(move |mut evt, _, control_flow| {
            // just to allow '?' usage
            let mut run = || -> windows::Result<()> {
                run_until_stalled();
                *control_flow = ControlFlow::Wait;
                match &mut evt {
                    Event::WindowEvent { event, window_id } => match event {
                        WindowEvent::Resized(size) => {
                            let size = Vector2 {
                                x: size.width as f32,
                                y: size.height as f32,
                            };
                            root_visual.set_size(&size)?;
                            root_panel.on_resize(&size)?;
                        }
                        WindowEvent::CloseRequested => {
                            if *window_id == globals_with(|globals| Ok(globals.window.id()))? {
                                // TODO: notify panels
                                *control_flow = ControlFlow::Exit;
                                globals_with(|globals| {
                                    drop(globals.target.take());
                                    Ok(())
                                })?;
                            }
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            let _ = root_panel.on_keyboard_input(*input)?;
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let position = Vector2 {
                                x: position.x as f32,
                                y: position.y as f32,
                            };
                            root_panel.on_mouse_move(&position)?;
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            let _ = root_panel.on_mouse_input(*button, *state)?;
                        }
                        _ => {}
                    },
                    Event::MainEventsCleared => {
                        root_panel.on_idle()?;
                    }
                    Event::UserEvent(ref mut panel_event) => {
                        root_panel.on_panel_event(panel_event)?;
                    }
                    _ => {}
                }
                Ok(())
            };
            if let Err(e) = run() {
                dbg!(&e);
                globals_with(|globals| {
                    drop(globals.target.take());
                    Ok(())
                })
                .unwrap();
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}
