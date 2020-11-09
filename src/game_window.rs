use std::any::Any;

use crate::window_target::CompositionDesktopWindowTargetSource;
use bindings::microsoft::graphics::canvas::ui::composition::CanvasComposition;
use bindings::microsoft::graphics::canvas::CanvasDevice;
use bindings::windows::foundation::numerics::Vector2;
use bindings::windows::ui::composition::desktop::DesktopWindowTarget;
use bindings::windows::ui::composition::CompositionGraphicsDevice;
use bindings::windows::ui::composition::Compositor;
use bindings::windows::ui::composition::ContainerVisual;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::{event_loop::EventLoopProxy, window::Window};

pub struct PanelEvent {
    pub panel_id: usize,
    pub data: Box<dyn Any>,
}
pub struct PanelMessage {
    pub panel_id: usize,
    pub request: Option<Box<dyn Any>>,
    pub response: Option<Box<dyn Any>>,
}

pub trait Panel {
    fn id(&self) -> usize;
    fn visual(&self) -> ContainerVisual;
    fn on_resize(&mut self) -> winrt::Result<()> {
        self.on_resize_default()
    }
    fn on_idle(&mut self, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }
    fn on_request(&mut self, request: Box<dyn Any>) -> winrt::Result<Box<dyn Any>> {
        Ok(Box::new(()))
    }
    fn on_mouse_input(
        &mut self,
        position: Vector2,
        button: MouseButton,
        state: ElementState,
    ) -> winrt::Result<()> {
        dbg!(position);
        Ok(())
    }
    fn translate_message(&mut self, msg: PanelMessage) -> winrt::Result<PanelMessage> {
        self.translate_message_default(msg)
    }
    //
    // default implementations to reuse in impls
    //
    fn translate_message_default(&mut self, msg: PanelMessage) -> winrt::Result<PanelMessage> {
        let mut msg = msg;
        if msg.panel_id == self.id() {
            if let Some(request) = msg.request.take() {
                msg.response = Some(self.on_request(request)?);
            }
        }
        Ok(msg)
    }
    fn on_resize_default(&mut self) -> winrt::Result<()> {
        self.visual().set_size(self.visual().parent()?.size()?)
    }
}

pub fn request_panel<RESP: Any>(
    root_panel: &mut dyn Panel,
    panel_id: usize,
    request: impl Any,
) -> winrt::Result<Option<RESP>> {
    let request = Some(Box::new(request) as Box<dyn Any>);
    let response = None;
    let mut msg = PanelMessage {
        panel_id,
        request,
        response,
    };
    msg = root_panel.translate_message(msg)?;
    if let Some(resp) = msg.response {
        if let Ok(resp) = resp.downcast::<RESP>() {
            Ok(Some(*resp))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

pub fn winrt_error<T: std::fmt::Display>(e: T) -> winrt::Error {
    winrt::Error::new(winrt::ErrorCode(0), format!("{}", e).as_str())
}

pub struct PanelEventProxy {
    proxy: EventLoopProxy<PanelEvent>,
}

impl PanelEventProxy {
    pub fn send_panel_event<T: Any>(&self, panel_id: usize, command: T) -> winrt::Result<()> {
        self.proxy
            .send_event(PanelEvent {
                panel_id,
                data: Box::new(command),
            })
            .map_err(winrt_error)
    }
}

pub struct EmptyPanel {
    id: usize,
    visual: ContainerVisual,
}

impl EmptyPanel {
    pub fn new(game_window: &mut GameWindow) -> winrt::Result<Self> {
        let visual = game_window.compositor().create_container_visual()?;
        let id = game_window.get_next_id();
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
}

pub struct GameWindow {
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    root: ContainerVisual,
    _target: DesktopWindowTarget,
    event_loop: Option<EventLoop<PanelEvent>>, // enclosed to Option to extract it from structure before starting event loop
    window: Window,
    panel: Option<Box<dyn Panel>>,
    next_id: usize,
}

impl GameWindow {
    pub fn new() -> winrt::Result<Self> {
        let event_loop = EventLoop::<PanelEvent>::with_user_event();
        let compositor = Compositor::new()?;
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

        let canvas_device = CanvasDevice::get_shared_device()?;
        let composition_graphics_device =
            CanvasComposition::create_composition_graphics_device(&compositor, &canvas_device)?;

        Ok(Self {
            compositor,
            canvas_device,
            composition_graphics_device,
            root,
            _target: target,
            event_loop: Some(event_loop),
            window,
            panel: None,
            next_id: 0,
        })
    }
    pub fn window(&mut self) -> &mut Window {
        &mut self.window
    }
    pub fn compositor(&self) -> &Compositor {
        &self.compositor
    }
    pub fn canvas_device(&self) -> &CanvasDevice {
        &self.canvas_device
    }
    pub fn composition_graphics_device(&self) -> &CompositionGraphicsDevice {
        &self.composition_graphics_device
    }
    pub fn visual(&self) -> &ContainerVisual {
        &self.root
    }
    pub fn get_next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn set_panel<P: Panel + 'static>(&mut self, panel: P) -> winrt::Result<()> {
        self.visual().children()?.insert_at_top(panel.visual())?;
        self.panel = Some(Box::new(panel));
        Ok(())
    }

    pub fn root_panel(&mut self) -> Option<&mut (dyn Panel + 'static)> {
        self.panel.as_deref_mut()
    }

    pub fn run<F>(mut self, mut event_handler: F)
    where
        F: 'static
            + FnMut(Event<'_, PanelEvent>, &mut Panel, &PanelEventProxy) -> winrt::Result<()>,
    {
        let event_loop = self.event_loop.take().unwrap();
        let mut panel = self.panel.take().unwrap();
        panel.on_resize().unwrap();
        let proxy = PanelEventProxy {
            proxy: event_loop.create_proxy(),
        };
        let mut cursor_position: PhysicalPosition<f64> = (0., 0.).into();
        event_loop.run(move |evt, _, control_flow| {
            // just to allow '?' usage
            || -> winrt::Result<()> {
                *control_flow = ControlFlow::Wait;
                match evt {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        window_id,
                    } if window_id == self.window.id() => {
                        *control_flow = ControlFlow::Exit;
                        Ok(())
                    }
                    Event::WindowEvent {
                        event: WindowEvent::Resized(size),
                        ..
                    } => {
                        let window_size = Vector2 {
                            x: size.width as f32,
                            y: size.height as f32,
                        };
                        self.root.set_size(&window_size)?;
                        panel.on_resize()?;
                        event_handler(evt, &mut *panel, &proxy)
                    }
                    Event::WindowEvent {
                        event: WindowEvent::CursorMoved { position, .. },
                        ..
                    } => {
                        cursor_position = position;
                        // TODO: on mouse move handle here
                        Ok(())
                    }
                    Event::WindowEvent {
                        event: WindowEvent::MouseInput { state, button, .. },
                        ..
                    } => {
                        // TODO: check for scaled modes
                        let position = Vector2 {
                            x: cursor_position.x as f32, // - window_position.x as f32,
                            y: cursor_position.y as f32, // - window_position.y as f32,
                        };
                        panel.on_mouse_input(position, button, state)
                    }
                    Event::MainEventsCleared => {
                        panel.on_idle(&proxy)?;
                        event_handler(evt, &mut *panel, &proxy)
                    }
                    evt => event_handler(evt, &mut *panel, &proxy),
                }
            }()
            .unwrap()
        });
    }
}
