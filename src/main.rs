mod game_field;
//mod game_score;
mod button_panel;
mod game_window;
mod interop;
mod numerics;
mod ribbon_panel;
mod text_panel;
mod window_target;

use button_panel::{ButtonPanel, ButtonPanelEvent};
use game_field::GameField;
use interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType};

use ribbon_panel::{Ribbon, RibbonOrientation};
use text_panel::TextPanel;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};

use game_window::{request_panel, EmptyPanel, GameWindow, Panel, PanelEvent, PanelEventProxy};

use model::field::Side::Right;
use model::field::Side::Up;
use model::field::Side::{self, Left};
use model::field::{Field, Side::Down};

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
    let mut game_field_panel = GameField::new(&mut window)?;
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
    undo_button_panel.add_panel(undo_button_text_panel)?;
    hribbon_panel.add_panel(undo_button_panel, 1.)?;
    hribbon_panel.add_panel(score_panel, 1.)?;
    hribbon_panel.add_panel(empty_panel, 1.)?;
    vribbon_panel.add_panel(hribbon_panel, 1.)?;
    vribbon_panel.add_panel(game_field_panel, 4.)?;
    window.set_panel(vribbon_panel)?;

    window.run(move |event, root_panel, proxy| match event {
        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    device_id: _,
                    input,
                    is_synthetic: _,
                },
            ..
        } => {
            if input.state == ElementState::Pressed {
                score += 1;
                score_handle.at(root_panel).set_text(score.to_string())?;
                if let Some(side) = match input.virtual_keycode {
                    Some(VirtualKeyCode::Left) => Some(Left),
                    Some(VirtualKeyCode::Right) => Some(Right),
                    Some(VirtualKeyCode::Up) => Some(Up),
                    Some(VirtualKeyCode::Down) => Some(Down),
                    _ => None,
                } {
                    game_field_handle.at(root_panel).swipe(side)?;
                } else if input.virtual_keycode == Some(VirtualKeyCode::Back) {
                    game_field_handle.at(root_panel).undo()?;
                }
                Ok(())
            } else {
                Ok(())
            }
        }
        Event::UserEvent(e) => {
            if undo_button_handle.event(e) == Some(ButtonPanelEvent::Pressed) {
                game_field_handle.at(root_panel).undo()?;
            }
            Ok(())
        }
        _ => Ok(()),
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
