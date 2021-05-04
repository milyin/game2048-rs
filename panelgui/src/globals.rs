use bindings::{
    microsoft::graphics::canvas::{ui::composition::CanvasComposition, CanvasDevice},
    windows::{
        system::DispatcherQueueController,
        ui::composition::{
            desktop::DesktopWindowTarget, CompositionGraphicsDevice, Compositor, ContainerVisual,
        },
    },
};
use futures::executor::{LocalPool, LocalSpawner};
use std::{
    any::Any,
    cell::RefCell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use windows::foundation::numerics::Vector2;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowBuilder},
};

use crate::panel::{Panel, PanelEvent};
use crate::{
    interop::create_dispatcher_queue_controller_for_current_thread,
    window_target::CompositionDesktopWindowTargetSource,
};

type RootPanel = crate::ribbon_panel::RibbonPanel;

pub struct Globals {
    _controller: DispatcherQueueController,
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    next_id: Arc<AtomicUsize>,
    event_loop: Option<EventLoop<PanelEvent>>,
    event_loop_proxy: EventLoopProxy<PanelEvent>,
    window: Window,
    root_visual: ContainerVisual,
    root_panel: Option<RootPanel>,
    target: Option<DesktopWindowTarget>,
}

impl Globals {
    fn new() -> windows::Result<Self> {
        let _controller = create_dispatcher_queue_controller_for_current_thread().unwrap();
        let compositor = Compositor::new().unwrap();
        let canvas_device = CanvasDevice::get_shared_device().unwrap();
        let composition_graphics_device =
            CanvasComposition::create_composition_graphics_device(&compositor, &canvas_device)
                .unwrap();
        let next_id = Arc::new(0.into());
        let event_loop = EventLoop::<PanelEvent>::with_user_event();
        let event_loop_proxy = event_loop.create_proxy();
        let window = WindowBuilder::new()
            .build(&event_loop)
            .map_err(|e| winrt_error(e.to_string())())?;
        let event_loop = Some(event_loop);
        let target = window.create_window_target(&compositor, false)?;
        let window_size = window.inner_size();
        let window_size = Vector2 {
            x: window_size.width as f32,
            y: window_size.height as f32,
        };
        let root_visual = compositor.create_container_visual()?;
        root_visual.set_size(window_size)?;
        let root_panel = crate::ribbon_panel::RibbonParamsBuilder::default()
            .orientation(crate::ribbon_panel::RibbonOrientation::Stack)
            .create()?;
        root_visual
            .children()
            .unwrap()
            .insert_at_top(root_panel.visual())?;
        target.set_root(&root_visual)?;
        let root_panel = Some(root_panel);
        Ok(Self {
            _controller,
            compositor,
            canvas_device,
            composition_graphics_device,
            next_id,
            event_loop,
            event_loop_proxy,
            window,
            target: Some(target),
            root_visual,
            root_panel,
        })
    }
}

thread_local! {
    static GLOBALS: RefCell<Option<Globals>> = RefCell::new(None);
}

thread_local! {
    static LOCAL_POOL: RefCell<LocalPool> = RefCell::new(LocalPool::new());
    static LOCAL_SPAWNER: LocalSpawner = LOCAL_POOL.with(|pool| pool.borrow_mut().spawner());
}

pub fn globals_with<F, T>(f: F) -> windows::Result<T>
where
    F: FnOnce(&mut Globals) -> windows::Result<T>,
{
    GLOBALS.with(|globals| {
        f(globals
            .borrow_mut()
            .as_mut()
            .ok_or_else(winrt_error("Globals not initialized"))?)
    })
}

pub fn globals_with_unwrap<F, T>(f: F) -> T
where
    F: FnOnce(&mut Globals) -> T,
{
    GLOBALS.with(|globals| {
        f(globals
            .borrow_mut()
            .as_mut()
            .expect("Globals not initialized"))
    })
}

pub fn init_window() -> windows::Result<()> {
    GLOBALS.with(|globals| {
        *globals.borrow_mut() = Some(Globals::new()?);
        Ok(())
    })
}

pub fn send_panel_event<T: Any>(panel_id: usize, command: T) -> windows::Result<()> {
    globals_with(|globals| {
        globals
            .event_loop_proxy
            .send_event(PanelEvent {
                panel_id,
                data: Some(Box::new(command)),
            })
            .map_err(|e| winrt_error(e)())
    })
}

pub fn spawner() -> LocalSpawner {
    LOCAL_SPAWNER.with(|spawner| spawner.clone())
}

pub fn run_until_stalled() {
    LOCAL_POOL.with(|pool| pool.borrow_mut().run_until_stalled());
}
// pub fn spawn(func: impl FnOnce(root: &RefCell<>) -> F) where F: Future<Output = windows::Result<()>>
// {
// }

pub fn compositor() -> Compositor {
    globals_with_unwrap(|globals| globals.compositor.clone())
}
pub fn canvas_device() -> CanvasDevice {
    globals_with_unwrap(|globals| globals.canvas_device.clone())
}
pub fn composition_graphics_device() -> CompositionGraphicsDevice {
    globals_with_unwrap(|globals| globals.composition_graphics_device.clone())
}
pub fn get_next_id() -> usize {
    globals_with_unwrap(|globals| globals.next_id.fetch_add(1, Ordering::SeqCst))
}

pub fn winrt_error<T: std::fmt::Display + 'static>(e: T) -> impl FnOnce() -> windows::Error {
    move || {
        const E_FAIL: windows::ErrorCode = windows::ErrorCode(0x80004005);
        windows::Error::new(E_FAIL, format!("{}", e).as_str())
    }
}

pub fn run(panel: impl Panel + 'static) -> ! {
    let event_loop =
        globals_with_unwrap(|globals| globals.event_loop.take().expect("Unexpected second run"));
    let mut root_panel =
        globals_with_unwrap(|globals| globals.root_panel.take().expect("Unexpected second run"));

    let root_visual = globals_with_unwrap(|globals| globals.root_visual.clone());

    root_panel
        .push_cell(
            crate::ribbon_panel::RibbonCellParamsBuilder::default()
                .panel(panel)
                .create()
                .expect("Error:"),
        )
        .expect("Error:");
    root_panel.on_init().expect("Error:");

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
