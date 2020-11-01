mod game_field;
//mod game_score;
mod game_window;
mod interop;
mod numerics;
mod ribbon;
mod window_target;

use std::any::Any;

use game_field::GameField;
use interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType};

use ribbon::{Ribbon, RibbonOrientation};
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::EventLoopProxy,
};

use game_window::{EmptyPanel, GameWindow, SendUserEvent};

use model::field::Side::Right;
use model::field::Side::Up;
use model::field::Side::{self, Left};
use model::field::{Field, Side::Down};

fn swipe(field: &mut Field, side: Side, proxy: &EventLoopProxy<Box<dyn Any>>) -> winrt::Result<()> {
    if field.can_swipe(side) {
        field.swipe(side);
        field.append_tile();
        field.append_tile();
        proxy.send_user_event(field.clone())
    } else {
        Ok(())
    }
}

fn run() -> winrt::Result<()> {
    ro_initialize(RoInitType::MultiThreaded)?;
    let _controller = create_dispatcher_queue_controller_for_current_thread()?;

    let mut field = Field::new(4, 4);
    field.append_tile();
    field.append_tile();

    let mut window = GameWindow::new()?;
    window.window().set_title("2048");
    let mut game_field = GameField::new(&mut window)?;
    game_field.animate_set_field(&field)?;

    let empty_panel = EmptyPanel::new(&window)?;
    let mut vribbon = Ribbon::new(&window, RibbonOrientation::Vertical)?;
    vribbon.add_panel(empty_panel, 1.)?;
    vribbon.add_panel(game_field, 4.)?;

    window.set_panel(vribbon)?;

    window.run(move |event, proxy| match event {
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
                    Some(VirtualKeyCode::Left) => swipe(&mut field, Left, proxy),
                    Some(VirtualKeyCode::Right) => swipe(&mut field, Right, proxy),
                    Some(VirtualKeyCode::Up) => swipe(&mut field, Up, proxy),
                    Some(VirtualKeyCode::Down) => swipe(&mut field, Down, proxy),
                    _ => Ok(()),
                }
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
