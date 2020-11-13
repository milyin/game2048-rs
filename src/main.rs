mod game_field;
//mod game_score;
mod button_panel;
mod control;
mod game_window;
mod interop;
mod numerics;
mod ribbon_panel;
mod text_panel;
mod window_target;

use button_panel::{ButtonPanel, ButtonPanelEvent};
use control::ControlManager;
use game_field::GameField;
use interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType};

use ribbon_panel::{Ribbon, RibbonOrientation};
use text_panel::TextPanel;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};

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
    // Constuct panels
    let game_field_panel = GameField::new(&mut window)?;
    let score_panel = TextPanel::new(&mut window)?;
    let mut undo_button_panel = ButtonPanel::new(&mut window)?;
    let mut undo_button_text_panel = TextPanel::new(&mut window)?;
    let empty_panel = EmptyPanel::new(&mut window)?;
    let mut vribbon_panel = Ribbon::new(&mut window, RibbonOrientation::Vertical)?;
    let mut hribbon_panel = Ribbon::new(&mut window, RibbonOrientation::Horizontal)?;

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

    let mut panel_manager = PanelManager::new(window.visual(), vribbon_panel)?;

    let mut control_manager = ControlManager::new();
    control_manager.add_control(undo_button_handle.clone());
    control_manager.enable(panel_manager.root_panel(), &undo_button_handle, false)?;

    window.run(move |event, proxy| {
        panel_manager.process_event(&event, proxy)?;
        control_manager.process_event(&event, proxy)?;
        match event {
            Event::UserEvent(e) => {
                if undo_button_handle.match_event(e) == Some(ButtonPanelEvent::Pressed) {
                    game_field_handle.at(panel_manager.root_panel())?.undo()?;
                }
            }
            _ => (),
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
