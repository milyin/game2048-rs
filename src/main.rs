use std::any::Any;

use bindings::windows::ui::composition::ContainerVisual;
use game_field_panel::{GameFieldHandle, GameFieldPanel, GameFieldPanelEvent};
use panelgui::{
    background_panel::BackgroundPanel,
    button_panel::{ButtonPanel, ButtonPanelEvent, ButtonPanelHandle},
    control::{Control, ControlManager},
    interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType},
    main_window::winrt_error,
    main_window::Handle,
    main_window::PanelHandle,
    main_window::{MainWindow, Panel, PanelEvent, PanelEventProxy, PanelGlobals},
    message_box_panel::MessageBoxButton,
    message_box_panel::MessageBoxPanel,
    message_box_panel::MessageBoxPanelHandle,
    ribbon_panel::RibbonOrientation,
    ribbon_panel::RibbonPanel,
    text_panel::{TextPanel, TextPanelHandle},
};

mod game_field_panel;

struct MainPanel {
    id: usize,
    globals: PanelGlobals,
    visual: ContainerVisual,
    root_panel: RibbonPanel,
    control_manager: ControlManager,
    game_field_handle: GameFieldHandle,
    undo_button_handle: ButtonPanelHandle,
    reset_button_handle: ButtonPanelHandle,
    score_handle: TextPanelHandle,
    message_box_reset_handle: Option<MessageBoxPanelHandle>,
}

impl MainPanel {
    pub fn new(globals: &PanelGlobals) -> winrt::Result<Self> {
        let globals = globals.clone();
        let id = globals.get_next_id();
        let visual = globals.compositor().create_container_visual()?;

        let mut root_panel = RibbonPanel::new(&globals, RibbonOrientation::Stack)?;
        let background_panel = BackgroundPanel::new(&globals)?;
        let game_field_panel = GameFieldPanel::new(&globals)?;
        let score_panel = TextPanel::new(&globals)?;
        let mut undo_button_panel = ButtonPanel::new(&globals)?;
        let mut undo_button_text_panel = TextPanel::new(&globals)?;
        let mut reset_button_panel = ButtonPanel::new(&globals)?;
        let mut reset_button_text_panel = TextPanel::new(&globals)?;
        let mut vribbon_panel = RibbonPanel::new(&globals, RibbonOrientation::Vertical)?;
        let mut hribbon_panel = RibbonPanel::new(&globals, RibbonOrientation::Horizontal)?;

        // Take handles
        let game_field_handle = game_field_panel.handle();
        let score_handle = score_panel.handle();
        let undo_button_handle = undo_button_panel.handle();
        let reset_button_handle = reset_button_panel.handle();

        undo_button_text_panel.set_text("⮌")?;
        reset_button_text_panel.set_text("⭯")?;

        undo_button_panel.add_panel(undo_button_text_panel)?;
        reset_button_panel.add_panel(reset_button_text_panel)?;
        hribbon_panel.push_panel(undo_button_panel, 1.)?;
        hribbon_panel.push_panel(score_panel, 1.)?;
        hribbon_panel.push_panel(reset_button_panel, 1.)?;
        vribbon_panel.push_panel(hribbon_panel, 1.)?;
        vribbon_panel.push_panel(game_field_panel, 4.)?;
        root_panel.push_panel(background_panel, 1.0)?;
        root_panel.push_panel(vribbon_panel, 1.0)?;
        visual
            .children()?
            .insert_at_top(root_panel.visual().clone())?;

        let mut control_manager = ControlManager::new();
        control_manager.add_control(undo_button_handle.clone());
        control_manager.add_control(reset_button_handle.clone());

        //root_panel.push_panel(message_box, 1.0)?;

        Ok(Self {
            id,
            globals,
            visual,
            root_panel,
            control_manager,
            game_field_handle,
            undo_button_handle,
            reset_button_handle,
            score_handle,
            message_box_reset_handle: None,
        })
    }

    fn update_buttons(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        let game_field = self.game_field_handle.at(&mut self.root_panel)?;
        let can_undo = game_field.can_undo();
        let score = game_field.get_score();
        self.undo_button_handle
            .at(&mut self.root_panel)?
            .enable(proxy, can_undo)?;
        self.score_handle
            .at(&mut self.root_panel)?
            .set_text(score.to_string())?;
        Ok(())
    }

    fn open_message_box_reset(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        let message_box = MessageBoxPanel::new(
            &self.globals,
            "Do you want to start new game?",
            MessageBoxButton::Yes | MessageBoxButton::No,
        )?;
        self.message_box_reset_handle = Some(message_box.handle());
        self.root_panel.push_panel(message_box, 1.0)?;
        self.root_panel.adjust_cells(proxy)?;
        Ok(())
    }

    fn close_message_box_reset(&mut self) -> winrt::Result<()> {
        if let Some(handle) = self.message_box_reset_handle.take() {
            let panel = self.root_panel.pop_panel()?;
            assert!(panel.id() == handle.id());
            Ok(())
        } else {
            Err(winrt_error("Message box was not open"))
        }
    }
}

impl Panel for MainPanel {
    fn id(&self) -> usize {
        self.id
    }

    fn visual(&self) -> ContainerVisual {
        self.visual.clone()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn on_init(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.on_resize(&self.visual().parent()?.size()?, proxy)?;
        self.update_buttons(proxy)
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id {
            Some(self.as_any_mut())
        } else {
            self.root_panel.find_panel(id)
        }
    }

    fn on_resize(
        &mut self,
        size: &bindings::windows::foundation::numerics::Vector2,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        self.visual().set_size(size)?;
        self.root_panel.on_resize(size, proxy)
    }

    fn on_idle(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.root_panel.on_idle(proxy)
    }

    fn on_mouse_move(
        &mut self,
        position: &bindings::windows::foundation::numerics::Vector2,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        self.root_panel.on_mouse_move(position, proxy)
    }

    fn on_mouse_input(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        self.root_panel.on_mouse_input(button, state, proxy)
    }

    fn on_keyboard_input(
        &mut self,
        input: winit::event::KeyboardInput,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<bool> {
        Ok(self.root_panel.on_keyboard_input(input, proxy)?
            || self
                .control_manager
                .process_keyboard_input(input, &mut self.root_panel, proxy)?)
    }

    fn on_panel_event(
        &mut self,
        panel_event: &mut PanelEvent,
        proxy: &PanelEventProxy,
    ) -> winrt::Result<()> {
        self.root_panel.on_panel_event(panel_event, proxy)?;
        if self.undo_button_handle.extract_event(panel_event) == Some(ButtonPanelEvent::Pressed) {
            self.game_field_handle
                .at(&mut self.root_panel)?
                .undo(proxy)?;
        } else if self.reset_button_handle.extract_event(panel_event)
            == Some(ButtonPanelEvent::Pressed)
        {
            self.open_message_box_reset(proxy)?;
        } else if let Some(h) = self.message_box_reset_handle.as_ref() {
            if let Some(cmd) = h.extract_event(panel_event) {
                self.close_message_box_reset()?;
                if cmd == MessageBoxButton::Yes {
                    self.game_field_handle
                        .at(&mut self.root_panel)?
                        .reset(proxy)?;
                }
            }
        } else if self.game_field_handle.extract_event(panel_event)
            == Some(GameFieldPanelEvent::Changed)
        {
            self.update_buttons(proxy)?;
        } else {
            self.control_manager
                .process_panel_event(panel_event, &mut self.root_panel, proxy)?;
        }
        Ok(())
    }
}

fn run() -> winrt::Result<()> {
    ro_initialize(RoInitType::MultiThreaded)?;
    let _controller = create_dispatcher_queue_controller_for_current_thread()?;
    let mut window = MainWindow::new()?;
    window.window().set_title("2048");
    let main_panel = MainPanel::new(window.get_globals())?;
    window.run(main_panel)
}
fn main() {
    let result = run();
    // We do this for nicer HRESULT printing when errors occur.
    if let Err(error) = result {
        error.code().unwrap();
    }
}
