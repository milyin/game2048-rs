#[macro_use]
extern crate derive_builder;

mod background_panel;
mod button_panel;
mod control;
mod globals;
mod interop;
mod message_box_panel;
mod numerics;
mod panel;
mod ribbon_panel;
mod text_panel;
mod window_target;

pub use globals::{
    canvas_device, composition_graphics_device, compositor, get_next_id, init_window, run,
    send_panel_event, spawner, winrt_error,
};

pub use panel::{EmptyPanel, Handle, Panel, PanelEvent, PanelHandle};

pub use control::{Control, ControlManager};

pub use background_panel::BackgroundParamsBuilder;

pub use button_panel::{ButtonPanelEvent, ButtonPanelHandle, ButtonParamsBuilder};

pub use message_box_panel::{MessageBoxButton, MessageBoxPanelHandle, MessageBoxParamsBuilder};

pub use ribbon_panel::{
    RibbonCellParamsBuilder, RibbonOrientation, RibbonPanel, RibbonPanelHandle, RibbonParamsBuilder,
};
pub use text_panel::{TextPanelHandle, TextParamsBuilder};
