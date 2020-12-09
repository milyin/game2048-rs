use lazy_static::lazy_static;
use std::{
    any::Any,
    sync::{Arc, Mutex},
};

use crate::{
    interop::create_dispatcher_queue_controller_for_current_thread, interop::ro_initialize,
    interop::RoInitType, window_target::CompositionDesktopWindowTargetSource,
};
use bindings::microsoft::graphics::canvas::ui::composition::CanvasComposition;
use bindings::microsoft::graphics::canvas::CanvasDevice;
use bindings::windows::foundation::numerics::Vector2;
use bindings::windows::ui::composition::desktop::DesktopWindowTarget;
use bindings::windows::ui::composition::CompositionGraphicsDevice;
use bindings::windows::ui::composition::Compositor;
use bindings::windows::ui::composition::ContainerVisual;
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
    fn on_init(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()>;
    fn on_resize(&mut self, size: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()>;
    fn on_idle(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()>;
    fn on_mouse_move(&mut self, position: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()>;
    fn on_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool>;
    fn on_keyboard_input(
        &mut self,
        input: KeyboardInput,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool>;
    fn on_panel_event(
        &mut self,
        panel_event: &mut PanelEvent,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()>;
}

#[derive(Clone)]
pub struct PanelGlobals {
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    next_id: Arc<Mutex<usize>>,
}

lazy_static! {
    static ref PANEL_GLOBALS: PanelGlobals = {
        // Not sure is it good place to do this required initialization
        ro_initialize(RoInitType::MultiThreaded).unwrap();
        let _controller = create_dispatcher_queue_controller_for_current_thread().unwrap();

        let compositor = Compositor::new().unwrap();
        let canvas_device = CanvasDevice::get_shared_device().unwrap();
        let composition_graphics_device =
            CanvasComposition::create_composition_graphics_device(&compositor, &canvas_device)
                .unwrap();

        PanelGlobals {
            compositor,
            canvas_device,
            composition_graphics_device,
            next_id: Arc::new(Mutex::new(0)),
        }
    };
}

pub fn globals() -> &'static PanelGlobals {
    &PANEL_GLOBALS
}

impl PanelGlobals {
    pub fn compositor(&self) -> &Compositor {
        &self.compositor
    }
    pub fn canvas_device(&self) -> &CanvasDevice {
        &self.canvas_device
    }
    pub fn composition_graphics_device(&self) -> &CompositionGraphicsDevice {
        &self.composition_graphics_device
    }
    pub fn get_next_id(&self) -> usize {
        let mut guard = self.next_id.lock().unwrap();
        let next_id = *guard;
        *guard += 1;
        next_id
    }
}

pub trait Handle {
    fn id(&self) -> usize;
}

pub trait PanelHandle<PanelType: Any, PanelEventType: Any = ()>: Handle {
    fn at<'a>(&self, root_panel: &'a mut dyn Panel) -> winrt::Result<&'a mut PanelType> {
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

pub fn winrt_error<T: std::fmt::Display + 'static>(e: T) -> impl FnOnce() -> winrt::Error {
    move || {
        const E_FAIL: winrt::ErrorCode = winrt::ErrorCode(0x80004005);
        winrt::Error::new(E_FAIL, format!("{}", e).as_str())
    }
}

pub struct PanelEventProxy {
    proxy: EventLoopProxy<PanelEvent>,
}

impl PanelEventProxy {
    pub fn send_panel_event<T: Any>(&self, panel_id: usize, command: T) -> winrt::Result<()> {
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
    pub fn new() -> winrt::Result<Self> {
        let visual = globals().compositor().create_container_visual()?;
        let id = globals().get_next_id();
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

    fn on_init(&mut self, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }

    fn on_resize(&mut self, _size: &Vector2, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }

    fn on_idle(&mut self, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }

    fn on_mouse_move(
        &mut self,
        _position: &Vector2,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }

    fn on_mouse_input(
        &mut self,
        _button: MouseButton,
        _state: ElementState,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        Ok(false)
    }

    fn on_keyboard_input(
        &mut self,
        _input: KeyboardInput,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        Ok(false)
    }

    fn on_panel_event(
        &mut self,
        _panel_event: &mut PanelEvent,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }
}

pub struct MainWindow {
    root: ContainerVisual,
    _target: DesktopWindowTarget,
    event_loop: Option<EventLoop<PanelEvent>>, // enclosed to Option to extract it from structure before starting event loop
    proxy: Option<PanelEventProxy>,
    window: Window,
}

impl MainWindow {
    pub fn new() -> winrt::Result<Self> {
        let event_loop = EventLoop::<PanelEvent>::with_user_event();
        let proxy = PanelEventProxy {
            proxy: event_loop.create_proxy(),
        };
        let compositor = globals().compositor();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let target = window.create_window_target(&compositor, false)?;
        let root = compositor.create_container_visual()?;
        let window_size = window.inner_size();
        let window_size = Vector2 {
            x: window_size.width as f32,
            y: window_size.height as f32,
        };
        root.set_size(&window_size)?;
        target.set_root(&root)?;

        window.set_min_inner_size(Some(PhysicalSize::new(100, 100)));

        Ok(Self {
            root,
            _target: target,
            event_loop: Some(event_loop),
            proxy: Some(proxy),
            window,
        })
    }
    pub fn window(&mut self) -> &mut Window {
        &mut self.window
    }
    pub fn visual(&self) -> &ContainerVisual {
        &self.root
    }
    pub fn proxy(&self) -> winrt::Result<&PanelEventProxy> {
        if let Some(ref p) = self.proxy {
            Ok(p)
        } else {
            Err(winrt_error(
                "unexpected error: proxy should be in Window struct until event loop run",
            )())
        }
    }

    pub fn run(mut self, mut panel: impl Panel + 'static) -> winrt::Result<()> {
        let event_loop = self.event_loop.take().unwrap();
        let proxy = PanelEventProxy {
            proxy: event_loop.create_proxy(),
        };
        self.visual().children()?.insert_at_top(panel.visual())?;
        panel.on_init(&proxy)?;
        event_loop.run(move |mut evt, _, control_flow| {
            // just to allow '?' usage
            let mut run = || -> winrt::Result<()> {
                *control_flow = ControlFlow::Wait;
                match &mut evt {
                    Event::WindowEvent { event, window_id } => match event {
                        WindowEvent::Resized(size) => {
                            let size = Vector2 {
                                x: size.width as f32,
                                y: size.height as f32,
                            };
                            self.root.set_size(&size)?;
                            panel.on_resize(&size, &proxy)?;
                        }
                        WindowEvent::CloseRequested => {
                            if *window_id == self.window.id() {
                                *control_flow = ControlFlow::Exit;
                                // TODO: notify panels
                            }
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            let _ = panel.on_keyboard_input(*input, &proxy)?;
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let position = Vector2 {
                                x: position.x as f32,
                                y: position.y as f32,
                            };
                            panel.on_mouse_move(&position, &proxy)?;
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            let _ = panel.on_mouse_input(*button, *state, &proxy)?;
                        }
                        _ => {}
                    },
                    Event::MainEventsCleared => {
                        panel.on_idle(&proxy)?;
                    }
                    Event::UserEvent(ref mut panel_event) => {
                        panel.on_panel_event(panel_event, &proxy)?;
                    }
                    _ => {}
                }
                Ok(())
            };
            if let Err(e) = run() {
                dbg!(&e);
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}
