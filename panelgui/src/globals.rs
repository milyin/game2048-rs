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
use lazy_static::lazy_static;
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
    event_loop::{EventLoop, EventLoopProxy},
    window::{Window, WindowBuilder},
};

use crate::panel::{Panel, PanelEvent};
use crate::{
    interop::create_dispatcher_queue_controller_for_current_thread,
    window_target::CompositionDesktopWindowTargetSource,
};

type RootPanel = crate::ribbon_panel::RibbonPanel;

pub struct Globals {
    pub event_loop: Option<EventLoop<PanelEvent>>,
    event_loop_proxy: EventLoopProxy<PanelEvent>,
    pub window: Window,
    pub root_visual: ContainerVisual,
    pub root_panel: Option<RootPanel>,
    pub target: Option<DesktopWindowTarget>,
}

impl Globals {
    fn new() -> windows::Result<Self> {
        let event_loop = EventLoop::<PanelEvent>::with_user_event();
        let event_loop_proxy = event_loop.create_proxy();
        let window = WindowBuilder::new()
            .build(&event_loop)
            .map_err(|e| winrt_error(e.to_string())())?;
        let event_loop = Some(event_loop);
        let target = window.create_window_target(compositor(), false)?;
        let window_size = window.inner_size();
        let window_size = Vector2 {
            x: window_size.width as f32,
            y: window_size.height as f32,
        };
        let root_visual = compositor().create_container_visual()?;
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
            event_loop,
            event_loop_proxy,
            window,
            target: Some(target),
            root_visual,
            root_panel,
        })
    }
}

impl Drop for Globals {
    fn drop(&mut self) {
        //        drop(self.target.take())
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

lazy_static! {
    static ref CONTROLLER: DispatcherQueueController =
        create_dispatcher_queue_controller_for_current_thread().unwrap();
    static ref COMPOSITOR: Compositor = {
        &*CONTROLLER;
        Compositor::new().unwrap()
    };
    static ref CANVAS_DEVICE: CanvasDevice = {
        &*CONTROLLER;
        CanvasDevice::get_shared_device().unwrap()
    };
    static ref COMPOSITION_GRAPHICS_DEVICE: CompositionGraphicsDevice =
        CanvasComposition::create_composition_graphics_device(&*COMPOSITOR, &*CANVAS_DEVICE)
            .unwrap();
    static ref NEXT_ID: Arc<AtomicUsize> = Arc::new(0.into());
}

pub fn compositor() -> &'static Compositor {
    &COMPOSITOR
}
pub fn canvas_device() -> &'static CanvasDevice {
    &CANVAS_DEVICE
}
pub fn composition_graphics_device() -> &'static CompositionGraphicsDevice {
    &COMPOSITION_GRAPHICS_DEVICE
}
pub fn get_next_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}

pub fn winrt_error<T: std::fmt::Display + 'static>(e: T) -> impl FnOnce() -> windows::Error {
    move || {
        const E_FAIL: windows::ErrorCode = windows::ErrorCode(0x80004005);
        windows::Error::new(E_FAIL, format!("{}", e).as_str())
    }
}
