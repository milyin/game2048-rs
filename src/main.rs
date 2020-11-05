mod game_field;
//mod game_score;
mod game_window;
mod interop;
mod numerics;
mod ribbon_panel;
mod text_panel;
mod window_target;

use game_field::GameField;
use interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType};

use ribbon_panel::{Ribbon, RibbonOrientation};
use text_panel::TextPanel;
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::EventLoopProxy,
};

use game_window::{EmptyPanel, GameWindow, Panel, PanelEvent, SendUserEvent};

use model::field::Side::Right;
use model::field::Side::Up;
use model::field::Side::{self, Left};
use model::field::{Field, Side::Down};

fn swipe(
    panel_id: usize,
    field: &mut Field,
    side: Side,
    proxy: &EventLoopProxy<PanelEvent>,
) -> winrt::Result<()> {
    if field.can_swipe(side) {
        field.swipe(side);
        field.append_tile();
        field.append_tile();
        proxy.send_command_to_panel(panel_id, field.clone())
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
    let game_field_id = game_field.id();
    let mut score = 0 as usize;

    let mut score_panel = TextPanel::new(&mut window)?;
    score_panel.set_text(score.to_string());
    let mut vribbon = Ribbon::new(&mut window, RibbonOrientation::Vertical)?;
    vribbon.add_panel(score_panel, 1.)?;
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
                    Some(VirtualKeyCode::Left) => swipe(game_field_id, &mut field, Left, proxy),
                    Some(VirtualKeyCode::Right) => swipe(game_field_id, &mut field, Right, proxy),
                    Some(VirtualKeyCode::Up) => swipe(game_field_id, &mut field, Up, proxy),
                    Some(VirtualKeyCode::Down) => swipe(game_field_id, &mut field, Down, proxy),
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
