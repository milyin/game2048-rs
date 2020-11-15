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
    fn get_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        self.get_panel_default(id)
    }
    fn on_resize(&mut self) -> winrt::Result<()> {
        self.on_resize_default()
    }
    fn on_idle(&mut self, _proxy: &PanelEventProxy) -> winrt::Result<()> {
        Ok(())
    }
    fn on_mouse_input(
        &mut self,
        _position: Vector2,
        _button: MouseButton,
        _state: ElementState,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }
    fn on_keyboard_input(
        &mut self,
        input: KeyboardInput,
        _proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        Ok(())
    }
    fn on_resize_default(&mut self) -> winrt::Result<()> {
        self.visual().set_size(self.visual().parent()?.size()?)
    }
    fn get_panel_default(&mut self, id: usize) -> Option<&mut dyn Any> {
        if self.id() == id {
            Some(self.as_any_mut())
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct PanelGlobals {
    compositor: Compositor,
    canvas_device: CanvasDevice,
    composition_graphics_device: CompositionGraphicsDevice,
}

pub trait Handle {
    fn id(&self) -> usize;
}

pub trait PanelHandle<PanelType: Any, PanelEventType: Any = ()>: Handle {
    fn at<'a>(&self, root_panel: &'a mut dyn Panel) -> Option<&'a mut PanelType> {
        if let Some(p) = root_panel.get_panel(self.id()) {
            if let Some(p) = p.downcast_mut::<PanelType>() {
                return Some(p);
            }
        }
        None
    }
    fn extract_event(&self, event: &mut PanelEvent) -> Option<PanelEventType> {
        if event.panel_id == self.id() {
            if let Some(data) = event.data.take() {
                data.downcast::<PanelEventType>().ok().map(|e| *e)
            } else {
                None
            }
        } else {
            None
        }
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
                data: Some(Box::new(command)),
            })
            .map_err(winrt_error)
    }
}

pub struct EmptyPanel {
    id: usize,
    visual: ContainerVisual,
}

impl EmptyPanel {
    pub fn new(panel_manager: &mut PanelManager) -> winrt::Result<Self> {
        let visual = panel_manager
            .get_globals()
            .compositor
            .create_container_visual()?;
        let id = panel_manager.get_next_id();
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
}

pub struct PanelManager {
    panel_globals: PanelGlobals,
    root_panel: Option<Box<dyn Panel>>,
    owner: ContainerVisual,
    cursor_position: PhysicalPosition<f64>,
    next_id: usize,
}

impl PanelManager {
    fn new(owner: &ContainerVisual, panel_globals: PanelGlobals) -> winrt::Result<Self> {
        let cursor_position = (0., 0.).into();
        let owner = owner.clone();
        Ok(PanelManager {
            panel_globals,
            root_panel: None,
            owner,
            cursor_position,
            next_id: 0,
        })
    }

    pub fn set_root_panel<P: Panel + 'static>(&mut self, panel: P) -> winrt::Result<()> {
        self.root_panel = Some(Box::new(panel));
        self.owner
            .children()?
            .insert_at_top(self.root_panel()?.visual().clone())?; // TODO: remove on drop
        self.root_panel()?.on_resize()?;
        Ok(())
    }

    pub fn root_panel(&mut self) -> winrt::Result<&mut (dyn Panel + 'static)> {
        if let Some(root_panel) = self.root_panel.as_deref_mut() {
            Ok(&mut *root_panel)
        } else {
            Err(winrt_error("No root panel set in panal manager"))
        }
    }

    pub fn panel<'a, T: Panel + 'static, E: Any + 'static, H: PanelHandle<T, E>>(
        &'a mut self,
        handle: H,
    ) -> winrt::Result<&'a mut T> {
        handle
            .at(self.root_panel()?)
            .ok_or(winrt_error("Can't find panel"))
    }

    pub fn get_next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn get_globals(&self) -> PanelGlobals {
        self.panel_globals.clone()
    }

    pub fn compositor(&self) -> &Compositor {
        &self.panel_globals.compositor
    }
    pub fn canvas_device(&self) -> &CanvasDevice {
        &self.panel_globals.canvas_device
    }
    pub fn composition_graphics_device(&self) -> &CompositionGraphicsDevice {
        &self.panel_globals.composition_graphics_device
    }

    pub fn process_event(
        &mut self,
        evt: &Event<PanelEvent>,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        match evt {
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                self.root_panel()?.on_resize()?;
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                self.cursor_position = *position;
                // TODO: on mouse move handle here
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        input,
                        is_synthetic: _,
                    },
                ..
            } => self.root_panel()?.on_keyboard_input(*input, proxy)?,
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                // TODO: check for scaled modes
                let position = Vector2 {
                    x: self.cursor_position.x as f32, // - window_position.x as f32,
                    y: self.cursor_position.y as f32, // - window_position.y as f32,
                };
                self.root_panel()?
                    .on_mouse_input(position, *button, *state, &proxy)?
            }
            Event::MainEventsCleared => {
                self.root_panel()?.on_idle(&proxy)?;
            }
            _ => {}
        }
        Ok(())
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

    pub fn create_panel_manager(&self) -> winrt::Result<PanelManager> {
        PanelManager::new(
            &self.root,
            PanelGlobals {
                compositor: self.compositor.clone(),
                canvas_device: self.canvas_device.clone(),
                composition_graphics_device: self.composition_graphics_device.clone(),
            },
        )
    }

    pub fn run<F>(mut self, mut event_handler: F)
    where
        F: 'static + FnMut(Event<'_, PanelEvent>, &PanelEventProxy) -> winrt::Result<()>,
    {
        let event_loop = self.event_loop.take().unwrap();
        let proxy = PanelEventProxy {
            proxy: event_loop.create_proxy(),
        };
        let mut cursor_position: PhysicalPosition<f64> = (0., 0.).into();
        event_loop.run(move |evt, _, control_flow| {
            // just to allow '?' usage
            || -> winrt::Result<()> {
                *control_flow = ControlFlow::Wait;
                match &evt {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        window_id,
                    } if *window_id == self.window.id() => {
                        *control_flow = ControlFlow::Exit;
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
                    }
                    _ => {}
                }
                event_handler(evt, &proxy)
            }()
            .unwrap()
        });
    }
}
