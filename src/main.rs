//mod background_panel;
mod button_panel;
mod control;
mod game_field_panel;
mod game_window;
mod interop;
mod numerics;
mod ribbon_panel;
mod text_panel;
mod window_target;

use button_panel::{ButtonPanel, ButtonPanelEvent};
use control::ControlManager;
use game_field_panel::{GameFieldPanel, GameFieldPanelEvent};
use interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType};

use ribbon_panel::{Ribbon, RibbonOrientation};
use text_panel::TextPanel;
use winit::event::Event;

use game_window::{EmptyPanel, GameWindow, PanelHandle, PanelManager};

fn run() -> winrt::Result<()> {
    ro_initialize(RoInitType::MultiThreaded)?;
    let _controller = create_dispatcher_queue_controller_for_current_thread()?;

    //
    // Construct model
    //
    let mut score = 0 as usize;
    //
    // Construct GUI
    //
    let mut window = GameWindow::new()?;
    window.window().set_title("2048");
    let mut panel_manager = window.create_panel_manager()?;

    // Constuct panels
    let game_field_panel = GameFieldPanel::new(&mut panel_manager)?;
    let score_panel = TextPanel::new(&mut panel_manager)?;
    let mut undo_button_panel = ButtonPanel::new(&mut panel_manager)?;
    let mut undo_button_text_panel = TextPanel::new(&mut panel_manager)?;
    let empty_panel = EmptyPanel::new(&mut panel_manager)?;
    let mut vribbon_panel = Ribbon::new(&mut panel_manager, RibbonOrientation::Vertical)?;
    let mut hribbon_panel = Ribbon::new(&mut panel_manager, RibbonOrientation::Horizontal)?;

    //
    // Initialize panels
    //
    undo_button_text_panel.set_text("⮌")?;

    // Take handles
    let game_field_handle = game_field_panel.handle();
    let score_handle = score_panel.handle();
    let undo_button_handle = undo_button_panel.handle();
    // Join panels into tree
    undo_button_panel.add_subpanel(undo_button_text_panel)?;
    hribbon_panel.add_panel(undo_button_panel, 1.)?;
    hribbon_panel.add_panel(score_panel, 1.)?;
    hribbon_panel.add_panel(empty_panel, 1.)?;
    vribbon_panel.add_panel(hribbon_panel, 1.)?;
    vribbon_panel.add_panel(game_field_panel, 4.)?;

    panel_manager.set_root_panel(vribbon_panel);

    let mut control_manager = ControlManager::new();
    control_manager.add_control(undo_button_handle.clone());

    let update_buttons = move |panel_manager: &mut PanelManager,
                               control_manager: &mut ControlManager|
          -> winrt::Result<()> {
        let game_field = panel_manager.panel(game_field_handle)?;
        let can_undo = game_field.can_undo();
        let score = game_field.get_score();
        control_manager
            .with(panel_manager)
            .enable(undo_button_handle, can_undo)?;
        panel_manager
            .panel(score_handle)?
            .set_text(score.to_string())?;
        Ok(())
    };

    update_buttons(&mut panel_manager, &mut control_manager)?;

    window.run(move |mut event, proxy| {
        panel_manager.process_event(&event, proxy)?;
        control_manager.process_event(&event, proxy)?;
        if let Event::UserEvent(ref mut e) = event {
            if undo_button_handle.extract_event(e) == Some(ButtonPanelEvent::Pressed) {
                panel_manager.panel(game_field_handle)?.undo(proxy)?;
            } else if game_field_handle.extract_event(e) == Some(GameFieldPanelEvent::Changed) {
                update_buttons(&mut panel_manager, &mut control_manager)?;
            }
        }
        Ok(())
    });
    Ok(())
}

fn main() {
    let result = run();

    // We do this for nicer HRESULT printing when errors occur.
    if let Err(error) = result {
        error.code().unwrap();
    }
}
