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
    canvas_device, composition_graphics_device, compositor, expect_panel_event, get_next_id,
    init_window, root_panel_with, run, send_panel_event, spawn, spawner, winrt_error,
};

pub use panel::{EmptyPanel, Handle, Panel, PanelEvent, PanelHandle};

pub use control::{Control, ControlManager};

pub use background_panel::BackgroundParamsBuilder;

pub use button_panel::{ButtonPanelEvent, ButtonPanelHandle, ButtonParamsBuilder};

pub use message_box_panel::{
    show_message_box, MessageBoxButton, MessageBoxPanelHandle, MessageBoxParamsBuilder,
};

pub use ribbon_panel::{
    RibbonCellParamsBuilder, RibbonOrientation, RibbonPanel, RibbonPanelHandle, RibbonParamsBuilder,
};
pub use text_panel::{TextPanelHandle, TextParamsBuilder};
