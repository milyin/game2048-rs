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

pub trait Panel {
    fn visual(&self) -> ContainerVisual;
    fn on_resize(&mut self) -> winrt::Result<()> {
        self.visual().set_size(self.visual().parent()?.size()?)
    }
    fn on_idle(&mut self, _proxy: &EventLoopProxy<Box<dyn Any>>) -> winrt::Result<()> {
        Ok(())
    }
    fn on_user_event(
        &mut self,
        evt: Box<dyn Any>,
        _proxy: &EventLoopProxy<Box<dyn Any>>,
    ) -> winrt::Result<Option<Box<dyn Any>>> {
        Ok(Some(evt))
    }
}

fn to_winrt_error<T: std::fmt::Display>(e: T) -> winrt::Error {
    winrt::Error::new(winrt::ErrorCode(0), format!("{}", e).as_str())
}

pub trait SendUserEvent {
    fn send_user_event<T: Any>(&self, event: T) -> winrt::Result<()>;
}

impl SendUserEvent for EventLoopProxy<Box<dyn Any>> {
    fn send_user_event<T: Any>(&self, event: T) -> winrt::Result<()> {
        self.send_event(Box::new(event)).map_err(to_winrt_error)
    }
}

pub struct EmptyPanel {
    visual: ContainerVisual,
}

impl EmptyPanel {
    pub fn new(game_window: &GameWindow) -> winrt::Result<Self> {
        let visual = game_window.compositor().create_container_visual()?;
        Ok(Self { visual })
    }
}

impl Panel for EmptyPanel {
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
    event_loop: Option<EventLoop<Box<dyn Any>>>,
    window: Window,
    panel: Option<Box<dyn Panel>>,
}

impl GameWindow {
    pub fn new() -> winrt::Result<Self> {
        let event_loop = EventLoop::<Box<dyn Any>>::with_user_event();
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

    pub fn set_panel<P: Panel + 'static>(&mut self, panel: P) -> winrt::Result<()> {
        self.visual().children()?.insert_at_top(panel.visual())?;
        self.panel = Some(Box::new(panel));
        Ok(())
    }

    pub fn run<F>(mut self, mut event_handler: F)
    where
        F: 'static
            + FnMut(Event<'_, Box<dyn Any>>, &EventLoopProxy<Box<dyn Any>>) -> winrt::Result<()>,
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
                        if let Some(user_event) = panel.on_user_event(user_event, &proxy)? {
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
