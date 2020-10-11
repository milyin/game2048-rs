mod game_field;
//mod game_score;
mod game_window;
mod interop;
mod numerics;
mod window_target;

use game_field::GameField;
use interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType};

use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};

use game_window::GameWindow;

use model::field::Side::Down;
use model::field::Side::Left;
use model::field::Side::Right;
use model::field::Side::Up;

use std::sync::mpsc;

fn run() -> winrt::Result<()> {
    ro_initialize(RoInitType::MultiThreaded)?;
    let _controller = create_dispatcher_queue_controller_for_current_thread()?;

    let (tx, rx) = mpsc::channel();

    let mut window = GameWindow::new()?;
    window.window().set_title("2048");
    let game_field = GameField::new(&mut window, rx)?;
    window.set_panel(game_field)?;

    window.run(move |event| match event {
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
                match input.virtual_keycode {
                    Some(VirtualKeyCode::Left) => tx.send(Left).unwrap(),
                    Some(VirtualKeyCode::Right) => tx.send(Right).unwrap(),
                    Some(VirtualKeyCode::Up) => tx.send(Up).unwrap(),
                    Some(VirtualKeyCode::Down) => tx.send(Down).unwrap(),
                    _ => (),
                    //Some(VirtualKeyCode::Left) => game_field.swipe(Left),
                    //Some(VirtualKeyCode::Right) => game_field.swipe(Right),
                    //Some(VirtualKeyCode::Up) => game_field.swipe(Up),
                    //Some(VirtualKeyCode::Down) => game_field.swipe(Down),
                    //Some(VirtualKeyCode::Back) => game_field.undo(),
                    //_ => Ok(()),
                }
                Ok(())
            } else {
                Ok(())
            }
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
