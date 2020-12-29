use std::any::Any;

use bindings::windows::{
    foundation::numerics::Vector2, ui::composition::ContainerVisual, ui::Colors,
};
use game_field_panel::{GameFieldHandle, GameFieldPanel, GameFieldPanelEvent};
use panelgui::{
    background_panel::BackgroundParamsBuilder,
    main_window::{globals, EmptyPanel},
    ribbon_panel::RibbonPanelHandle,
};
use panelgui::{
    button_panel::{ButtonPanelEvent, ButtonPanelHandle, ButtonParamsBuilder},
    control::{Control, ControlManager},
    main_window::winrt_error,
    main_window::Handle,
    main_window::PanelHandle,
    main_window::{MainWindow, Panel, PanelEvent, PanelEventProxy},
    message_box_panel::MessageBoxButton,
    message_box_panel::MessageBoxPanelHandle,
    message_box_panel::MessageBoxParamsBuilder,
    ribbon_panel::RibbonCellParamsBuilder,
    ribbon_panel::RibbonOrientation,
    ribbon_panel::RibbonPanel,
    ribbon_panel::RibbonParamsBuilder,
    text_panel::{TextPanelHandle, TextParamsBuilder},
};

mod game_field_panel;

struct MainPanel {
    id: usize,
    visual: ContainerVisual,
    root_panel: RibbonPanel,
    control_manager: ControlManager,
    game_field_handle: GameFieldHandle,
    undo_button_handle: ButtonPanelHandle,
    reset_button_handle: ButtonPanelHandle,
    horizontal_padding_handle: RibbonPanelHandle,
    vertical_padding_handle: RibbonPanelHandle,
    game_panel_handle: RibbonPanelHandle,
    score_handle: TextPanelHandle,
    message_box_reset_handle: Option<MessageBoxPanelHandle>,
}

impl MainPanel {
    pub fn new() -> winrt::Result<Self> {
        let id = globals().get_next_id();

        let background_panel = BackgroundParamsBuilder::default()
            .color(Colors::white()?)
            .create()?;
        let game_field_panel = GameFieldPanel::new()?;
        let score_panel = TextParamsBuilder::default().create()?;
        let undo_button_panel = ButtonParamsBuilder::default().text("⮌")?.create()?;
        let reset_button_panel = ButtonParamsBuilder::default().text("⭯")?.create()?;

        let game_field_handle = game_field_panel.handle();
        let score_handle = score_panel.handle();
        let undo_button_handle = undo_button_panel.handle();
        let reset_button_handle = reset_button_panel.handle();

        let header_panel = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Horizontal)
            .add_panel(undo_button_panel)?
            .add_panel_with_ratio(score_panel, 2.)?
            .add_panel(reset_button_panel)?
            .create()?;

        let game_ribbon = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Vertical)
            .add_panel(header_panel)?
            .add_panel_with_ratio(game_field_panel, 4.)?
            .create()?;

        let game_panel = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Stack)
            .add_panel(game_ribbon)?
            .create()?;

        let game_panel_handle = game_panel.handle();

        let vertical_padding_panel = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Vertical)
            .add_panel(game_panel)?
            .add_panel(EmptyPanel::new()?)?
            .create()?;

        let vertical_padding_handle = vertical_padding_panel.handle();

        let horizontal_padding_panel = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Horizontal)
            .add_panel(EmptyPanel::new()?)?
            .add_cell(
                RibbonCellParamsBuilder::default()
                    .panel(vertical_padding_panel)
                    .create()?,
            )
            .add_panel(EmptyPanel::new()?)?
            .create()?;

        let horizontal_padding_handle = horizontal_padding_panel.handle();

        let root_panel = RibbonParamsBuilder::default()
            .orientation(RibbonOrientation::Stack)
            .add_panel(background_panel)?
            .add_panel(horizontal_padding_panel)?
            .create()?;

        let visual = globals().compositor().create_container_visual()?;
        visual
            .children()?
            .insert_at_top(root_panel.visual().clone())?;

        let mut control_manager = ControlManager::new();
        control_manager.add_control(undo_button_handle.clone());
        control_manager.add_control(reset_button_handle.clone());

        Ok(Self {
            id,
            visual,
            root_panel,
            control_manager,
            game_field_handle,
            undo_button_handle,
            reset_button_handle,
            horizontal_padding_handle,
            vertical_padding_handle,
            game_panel_handle,
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
        let message_box = MessageBoxParamsBuilder::default()
            .message("Start new game?")
            .button_flags(MessageBoxButton::Yes | MessageBoxButton::No)
            .create()?;
        self.message_box_reset_handle = Some(message_box.handle());
        let cell = RibbonCellParamsBuilder::default()
            .panel(message_box)
            .content_ratio(Vector2 { x: 0.9, y: 0.4 })
            .create()?;
        self.game_panel_handle
            .at(&mut self.root_panel)?
            .push_cell(cell, proxy)?;
        Ok(())
    }

    fn close_message_box_reset(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        if let Some(handle) = self.message_box_reset_handle.take() {
            let cell = self
                .game_panel_handle
                .at(&mut self.root_panel)?
                .pop_cell(proxy)?;
            assert!(cell.panel().id() == handle.id());
            Ok(())
        } else {
            Err(winrt_error("Message box was not open")())
        }
    }

    fn do_undo(&mut self, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.game_field_handle
            .at(&mut self.root_panel)?
            .undo(proxy)?;
        Ok(())
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
        self.update_buttons(proxy)?;
        self.root_panel.on_init(proxy)
    }

    fn find_panel(&mut self, id: usize) -> Option<&mut dyn Any> {
        if id == self.id {
            Some(self.as_any_mut())
        } else {
            self.root_panel.find_panel(id)
        }
    }

    fn on_resize(&mut self, size: &Vector2, proxy: &PanelEventProxy) -> winrt::Result<()> {
        self.visual().set_size(size)?;
        self.root_panel.on_resize(size, proxy)?;

        let mut width_limit = self
            .horizontal_padding_handle
            .at(&mut self.root_panel)?
            .get_cell_limit(1)?;
        let mut height_limit = self
            .vertical_padding_handle
            .at(&mut self.root_panel)?
            .get_cell_limit(0)?;

        // size.x / size.y > 4/5
        if 5. * size.x > 4. * size.y {
            // x is too large limit width
            height_limit.set_size(size.y);
            width_limit.set_size(size.y * 4. / 5.);
        } else {
            // y is too large, limit height
            height_limit.set_size(size.x * 5. / 4.);
            width_limit.set_size(size.x);
        }
        self.horizontal_padding_handle
            .at(&mut self.root_panel)?
            .set_cell_limit(1, width_limit, proxy)?;
        self.vertical_padding_handle
            .at(&mut self.root_panel)?
            .set_cell_limit(0, height_limit, proxy)?;
        Ok(())
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
                self.close_message_box_reset(proxy)?;
                if cmd == MessageBoxButton::Yes {
                    self.game_field_handle
                        .at(&mut self.root_panel)?
                        .reset(proxy)?;
                }
            }
        } else if let Some(cmd) = self.game_field_handle.extract_event(panel_event) {
            match cmd {
                GameFieldPanelEvent::Changed => self.update_buttons(proxy)?,
                GameFieldPanelEvent::UndoRequested => self.do_undo(proxy)?,
                GameFieldPanelEvent::ResetRequested => self.open_message_box_reset(proxy)?,
            }
        } else {
            self.control_manager
                .process_panel_event(panel_event, &mut self.root_panel, proxy)?;
        }
        Ok(())
    }
}

fn run() -> winrt::Result<()> {
    let mut window = MainWindow::new()?;
    window.window().set_title("2048");
    let main_panel = MainPanel::new()?;
    window.run(main_panel)
}
fn main() {
    let result = run();
    // We do this for nicer HRESULT printing when errors occur.
    if let Err(error) = result {
        dbg!(&error);
        error.code().unwrap();
    }
}
