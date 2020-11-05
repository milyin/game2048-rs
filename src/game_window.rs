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
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::{event_loop::EventLoopProxy, window::Window};

pub struct PanelEvent {
    pub panel_id: usize,
    pub command: Box<dyn Any>,
}

pub trait Panel {
    fn id(&self) -> usize;
    fn visual(&self) -> ContainerVisual;
    fn on_resize(&mut self) -> winrt::Result<()> {
        self.visual().set_size(self.visual().parent()?.size()?)
    }
    fn on_idle(&mut self, _proxy: &EventLoopProxy<PanelEvent>) -> winrt::Result<()> {
        Ok(())
    }
    fn on_command(&mut self, command: Box<dyn Any>) -> winrt::Result<()> {
        Ok(())
    }
    fn translate_panel_event(
        &mut self,
        evt: PanelEvent,
        _proxy: &EventLoopProxy<PanelEvent>,
    ) -> winrt::Result<Option<PanelEvent>> {
        if evt.panel_id == self.id() {
            self.on_command(evt.command);
            Ok(None)
        } else {
            Ok(Some(evt))
        }
    }
}

fn to_winrt_error<T: std::fmt::Display>(e: T) -> winrt::Error {
    winrt::Error::new(winrt::ErrorCode(0), format!("{}", e).as_str())
}

pub trait SendUserEvent {
    fn send_command_to_panel<T: Any>(&self, panel_id: usize, command: T) -> winrt::Result<()>;
}

impl SendUserEvent for EventLoopProxy<PanelEvent> {
    fn send_command_to_panel<T: Any>(&self, panel_id: usize, command: T) -> winrt::Result<()> {
        self.send_event(PanelEvent {
            panel_id,
            command: Box::new(command),
        })
        .map_err(to_winrt_error)
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

    pub fn run<F>(mut self, mut event_handler: F)
    where
        F: 'static + FnMut(Event<'_, PanelEvent>, &EventLoopProxy<PanelEvent>) -> winrt::Result<()>,
    {
        let event_loop = self.event_loop.take().unwrap();
        let mut panel = self.panel.take().unwrap();
        panel.on_resize().unwrap();
        let proxy = event_loop.create_proxy();
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
                        event_handler(evt, &proxy)
                    }
                    Event::MainEventsCleared => {
                        panel.on_idle(&proxy)?;
                        event_handler(evt, &proxy)
                    }
                    Event::UserEvent(user_event) => {
                        if let Some(user_event) = panel.translate_panel_event(user_event, &proxy)? {
                            event_handler(Event::UserEvent(user_event), &proxy)?;
                        }
                        Ok(())
                    }
                    evt => event_handler(evt, &proxy),
                }
            }()
            .unwrap()
        });
    }
}
