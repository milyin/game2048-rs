use crate::window_target::CompositionDesktopWindowTargetSource;
use bindings::microsoft::graphics::canvas::ui::composition::CanvasComposition;
use bindings::microsoft::graphics::canvas::CanvasDevice;
use bindings::windows::foundation::numerics::Vector2;
use bindings::windows::ui::composition::desktop::DesktopWindowTarget;
use bindings::windows::ui::composition::CompositionGraphicsDevice;
use bindings::windows::ui::composition::Compositor;
use bindings::windows::ui::composition::ContainerVisual;
use winit::window::Window;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub trait Panel {
    fn visual(&self) -> ContainerVisual;
    fn adjust_size(&mut self) -> winrt::Result<()> {
        self.visual().set_size(self.visual().parent()?.size()?)
    }
    fn handle_event(&mut self, _evt: &Event<'_, ()>) -> winrt::Result<()> {
        Ok(())
    }
}

pub struct GameWindow {
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
    root: ContainerVisual,
    _target: DesktopWindowTarget,
    event_loop: Option<EventLoop<()>>,
    window: Window,
    panel: Option<Box<dyn Panel>>,
}

impl GameWindow {
    pub fn new() -> winrt::Result<Self> {
        let event_loop = EventLoop::new();
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
        F: 'static + FnMut(Event<'_, ()>) -> winrt::Result<()>,
    {
        let event_loop = self.event_loop.take().unwrap();
        let mut panel = self.panel.take().unwrap();
        event_loop.run(move |evt, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match evt {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == self.window.id() => *control_flow = ControlFlow::Exit,
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let window_size = Vector2 {
                        x: size.width as f32,
                        y: size.height as f32,
                    };
                    self.root.set_size(&window_size).unwrap();
                    panel.adjust_size().unwrap();
                    panel.handle_event(&evt).unwrap();
                    event_handler(evt).unwrap()
                }
                evt => {
                    panel.handle_event(&evt).unwrap();
                    event_handler(evt).unwrap()
                }
            }
        });
    }
}
