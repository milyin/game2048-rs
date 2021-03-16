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

use crate::{
    interop::create_dispatcher_queue_controller_for_current_thread, ribbon_panel::RibbonPanel,
    window_target::CompositionDesktopWindowTargetSource,
};
use bindings::microsoft::graphics::canvas::CanvasDevice;
use bindings::windows::foundation::numerics::Vector2;
use bindings::windows::ui::composition::desktop::DesktopWindowTarget;
use bindings::windows::ui::composition::CompositionGraphicsDevice;
use bindings::windows::ui::composition::Compositor;
use bindings::windows::ui::composition::ContainerVisual;
use bindings::{
    microsoft::graphics::canvas::ui::composition::CanvasComposition,
    windows::system::DispatcherQueueController,
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::{event_loop::EventLoopProxy, window::Window};

pub struct PanelEvent {
    pub panel_id: usize,
    pub data: Option<Box<dyn Any>>,
}
pub trait Panel {
    fn id(&self) -> usize;
    fn visual(&self) -> ContainerVisual;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any>;
    fn on_init(&mut self) -> windows::Result<()>;
    fn on_resize(&mut self, size: &Vector2) -> windows::Result<()>;
    fn on_idle(&mut self) -> windows::Result<()>;
    fn on_mouse_move(&mut self, position: &Vector2) -> windows::Result<()>;
    fn on_mouse_input(&mut self, button: MouseButton, state: ElementState)
        -> windows::Result<bool>;
    fn on_keyboard_input(&mut self, input: KeyboardInput) -> windows::Result<bool>;
    fn on_panel_event(&mut self, panel_event: &mut PanelEvent) -> windows::Result<()>;
}

type RootPanel = crate::ribbon_panel::RibbonPanel;

thread_local! {
    static EVENT_LOOP: RefCell<Option<EventLoop<PanelEvent>>> = RefCell::new(Some(EventLoop::<PanelEvent>::with_user_event()));
    static EVENT_LOOP_PROXY: RefCell<EventLoopProxy<PanelEvent>> =
        EVENT_LOOP.with(|event_loop| RefCell::new(event_loop.borrow().as_ref().unwrap().create_proxy()));
    static WINDOW: Window = EVENT_LOOP.with(|event_loop| {
        let window = WindowBuilder::new().build(&event_loop.borrow().as_ref().unwrap()).unwrap();
        window.set_min_inner_size(Some(PhysicalSize::new(100, 100)));
        window
    });
    static TARGET: DesktopWindowTarget = WINDOW.with(|window| window.create_window_target(&compositor(), false).unwrap());
    static ROOT_VISUAL: ContainerVisual = WINDOW.with(|window| TARGET.with(|target| {
        let window_size = window.inner_size();
        let window_size = Vector2 {
            x: window_size.width as f32,
            y: window_size.height as f32,
        };
        let root = compositor().create_container_visual().unwrap();
        root.set_size(&window_size).unwrap();
        target.set_root(&root).unwrap();
        root
    }));
    static ROOT_PANEL: RefCell<RootPanel> = ROOT_VISUAL.with(|visual| {
        let panel = crate::ribbon_panel::RibbonParamsBuilder::default()
            .orientation(crate::ribbon_panel::RibbonOrientation::Stack).create().unwrap();
        visual.children().unwrap().insert_at_top(panel.visual()).unwrap();
        RefCell::new(panel)
    });
}

thread_local! {
    static LOCAL_POOL: RefCell<LocalPool> = RefCell::new(LocalPool::new());
    static LOCAL_SPAWNER: LocalSpawner = LOCAL_POOL.with(|pool| pool.borrow_mut().spawner());
}

pub fn send_panel_event<T: Any>(panel_id: usize, command: T) -> windows::Result<()> {
    EVENT_LOOP_PROXY.with(|proxy| {
        proxy
            .borrow()
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

pub trait Handle {
    fn id(&self) -> usize;
}

pub trait PanelHandle<PanelType: Any, PanelEventType: Any = ()>: Handle {
    fn at<'a>(&self, root_panel: &'a mut dyn Panel) -> windows::Result<&'a mut PanelType> {
        if let Some(p) = root_panel.find_panel(self.id()) {
            if let Some(p) = p.downcast_mut::<PanelType>() {
                return Ok(p);
            }
        }
        Err(winrt_error("Can't find panel")())
    }
    fn extract_event(&self, panel_event: &mut PanelEvent) -> Option<PanelEventType> {
        if panel_event.panel_id == self.id() {
            if let Some(data) = panel_event.data.take() {
                match data.downcast::<PanelEventType>() {
                    Ok(e) => Some(*e),
                    Err(data) => {
                        panel_event.data = Some(data);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub fn winrt_error<T: std::fmt::Display + 'static>(e: T) -> impl FnOnce() -> windows::Error {
    move || {
        const E_FAIL: windows::ErrorCode = windows::ErrorCode(0x80004005);
        windows::Error::new(E_FAIL, format!("{}", e).as_str())
    }
}

pub struct PanelEventProxy {
    proxy: EventLoopProxy<PanelEvent>,
}

impl PanelEventProxy {
    pub fn send_panel_event<T: Any>(&self, panel_id: usize, command: T) -> windows::Result<()> {
        self.proxy
            .send_event(PanelEvent {
                panel_id,
                data: Some(Box::new(command)),
            })
            .map_err(|e| winrt_error(e)())
    }
}

pub struct EmptyPanel {
    id: usize,
    visual: ContainerVisual,
}

impl EmptyPanel {
    pub fn new() -> windows::Result<Self> {
        let visual = compositor().create_container_visual()?;
        let id = get_next_id();
        Ok(Self { id, visual })
    }
}

impl Panel for EmptyPanel {
    fn id(&self) -> usize {
        self.id
    }
    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id() {
            Some(self.as_any_mut())
        } else {
            None
        }
    }

    fn on_init(&mut self) -> windows::Result<()> {
        Ok(())
    }

    fn on_resize(&mut self, _size: &Vector2) -> windows::Result<()> {
        Ok(())
    }

    fn on_idle(&mut self) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_move(&mut self, _position: &Vector2) -> windows::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: MouseButton,
        _state: ElementState,
    ) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(&mut self, _input: KeyboardInput) -> windows::Result<bool> {
        Ok(false)
    }

    fn on_panel_event(&mut self, _panel_event: &mut PanelEvent) -> windows::Result<()> {
        Ok(())
    }
}

pub struct MainWindow {}

impl MainWindow {
    pub fn run(self, panel: impl Panel + 'static) -> windows::Result<()> {
        ROOT_PANEL.with(|root_panel| {
            let mut root_panel = root_panel.borrow_mut();
            root_panel.push_cell(
                crate::ribbon_panel::RibbonCellParamsBuilder::default()
                    .panel(panel)
                    .create()?,
            )?;
            root_panel.on_init()
        })?;
        EVENT_LOOP.with(|event_loop| {
            event_loop
                .borrow_mut()
                .take()
                .unwrap()
                .run(move |mut evt, _, control_flow| {
                    // just to allow '?' usage
                    let mut run = || -> windows::Result<()> {
                        LOCAL_POOL.with(|pool| pool.borrow_mut().run_until_stalled());
                        *control_flow = ControlFlow::Wait;
                        ROOT_PANEL.with(|p| -> windows::Result<()> {
                            let mut panel = p.borrow_mut();
                            match &mut evt {
                                Event::WindowEvent { event, window_id } => match event {
                                    WindowEvent::Resized(size) => {
                                        let size = Vector2 {
                                            x: size.width as f32,
                                            y: size.height as f32,
                                        };
                                        ROOT_VISUAL.with(|root| root.set_size(&size))?;
                                        panel.on_resize(&size)?;
                                    }
                                    WindowEvent::CloseRequested => {
                                        if *window_id == WINDOW.with(|window| window.id()) {
                                            *control_flow = ControlFlow::Exit;
                                            // TODO: notify panels
                                        }
                                    }
                                    WindowEvent::KeyboardInput { input, .. } => {
                                        let _ = panel.on_keyboard_input(*input)?;
                                    }
                                    WindowEvent::CursorMoved { position, .. } => {
                                        let position = Vector2 {
                                            x: position.x as f32,
                                            y: position.y as f32,
                                        };
                                        panel.on_mouse_move(&position)?;
                                    }
                                    WindowEvent::MouseInput { state, button, .. } => {
                                        let _ = panel.on_mouse_input(*button, *state)?;
                                    }
                                    _ => {}
                                },
                                Event::MainEventsCleared => {
                                    panel.on_idle()?;
                                }
                                Event::UserEvent(ref mut panel_event) => {
                                    panel.on_panel_event(panel_event)?;
                                }
                                _ => {}
                            }
                            Ok(())
                        })?;
                        Ok(())
                    };
                    if let Err(e) = run() {
                        dbg!(&e);
                        *control_flow = ControlFlow::Exit;
                    }
                });
        })
    }
}
