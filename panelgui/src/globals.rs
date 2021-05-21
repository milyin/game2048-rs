use bindings::Windows::Foundation::Numerics::Vector2;
use bindings::{
    Microsoft::Graphics::Canvas::{CanvasDevice, UI::Composition::CanvasComposition},
    Windows::{
        System::DispatcherQueueController,
        UI::Composition::{
            CompositionGraphicsDevice, Compositor, ContainerVisual, Desktop::DesktopWindowTarget,
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
use windows::HRESULT;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowBuilder},
};

use crate::{
    interop::create_dispatcher_queue_controller_for_current_thread,
    window_target::CompositionDesktopWindowTargetSource,
};
use crate::{
    panel::{Panel, PanelEvent},
    RibbonOrientation, RibbonParamsBuilder,
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
    local_pool: Option<LocalPool>,
    local_spawner: LocalSpawner,
}

impl Globals {
    fn new() -> windows::Result<Self> {
        let _controller = create_dispatcher_queue_controller_for_current_thread()?;
        let compositor = Compositor::new()?;
        let canvas_device = CanvasDevice::GetSharedDevice()?;
        let composition_graphics_device =
            CanvasComposition::CreateCompositionGraphicsDevice(&compositor, &canvas_device)?;
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
            X: window_size.width as f32,
            Y: window_size.height as f32,
        };
        let root_visual = compositor.CreateContainerVisual()?;
        root_visual.SetSize(window_size)?;
        target.SetRoot(&root_visual)?;
        let target = Some(target);
        let root_panel = None;
        let local_pool = LocalPool::new();
        let local_spawner = local_pool.spawner();
        let local_pool = Some(local_pool);
        Ok(Self {
            _controller,
            compositor,
            canvas_device,
            composition_graphics_device,
            next_id,
            event_loop,
            event_loop_proxy,
            window,
            target,
            root_visual,
            root_panel,
            local_pool,
            local_spawner,
        })
    }
}

thread_local! {
    static GLOBALS: RefCell<Option<Globals>> = RefCell::new(None);
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
    GLOBALS.with::<_, windows::Result<()>>(|globals| {
        *globals.borrow_mut() = Some(Globals::new()?);
        Ok(())
    })?;
    let root_panel = RibbonParamsBuilder::default()
        .orientation(RibbonOrientation::Stack)
        .create()?;
    globals_with(|globals| {
        globals
            .root_visual
            .Children()
            .unwrap()
            .InsertAtTop(root_panel.visual())?;
        globals.root_panel = Some(root_panel);
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
    globals_with_unwrap(|globals| globals.local_spawner.clone())
}

// pub fn spawn(func: impl FnOnce(root: &RefCell<>) -> F) where F: Future<Output = windows::Result<()>>
// {
//     spawner().
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
        const E_FAIL: HRESULT = HRESULT(0x80004005);
        windows::Error::new(E_FAIL, format!("{}", e).as_str())
    }
}

pub fn run(panel: impl Panel + 'static) -> ! {
    let event_loop =
        globals_with_unwrap(|globals| globals.event_loop.take().expect("Unexpected second run"));
    let mut root_panel =
        globals_with_unwrap(|globals| globals.root_panel.take().expect("Unexpected second run"));
    let mut local_pool =
        globals_with_unwrap(|globals| globals.local_pool.take().expect("Unexpected second run"));

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
            local_pool.run_until_stalled();
            *control_flow = ControlFlow::Wait;
            match &mut evt {
                Event::WindowEvent { event, window_id } => match event {
                    WindowEvent::Resized(size) => {
                        let size = Vector2 {
                            X: size.width as f32,
                            Y: size.height as f32,
                        };
                        root_visual.SetSize(&size)?;
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
                            X: position.x as f32,
                            Y: position.y as f32,
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
