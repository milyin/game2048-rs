use bindings::windows::{
    foundation::{IPropertyValue, PropertyValue},
    storage::{AppDataPaths, ApplicationData, ApplicationDataCreateDisposition},
};
use game_field_panel::{GameFieldPanel, GameFieldPanelEvent};
use model::field::Field;
use panelgui::{
    background_panel::BackgroundPanel,
    button_panel::{ButtonPanel, ButtonPanelEvent},
    control::{Control, ControlManager},
    interop::{create_dispatcher_queue_controller_for_current_thread, ro_initialize, RoInitType},
    main_window::PanelHandle,
    main_window::{MainWindow, PanelEventProxy, PanelManager},
    ribbon_panel::Ribbon,
    ribbon_panel::RibbonOrientation,
    text_panel::TextPanel,
};

mod game_field_panel;

use winit::event::Event;
use winrt::TryInto;
use winrt::{HString, Object};

const CONTAINER_NAME: &str = "Game";
const CONTAINER_SETTING_FIELD: &str = "Field";

fn run() -> winrt::Result<()> {
    ro_initialize(RoInitType::MultiThreaded)?;
    let _controller = create_dispatcher_queue_controller_for_current_thread()?;

    //
    // Read storage
    //
    //let app_data = ApplicationData::current()?;
    //let roaming_settings = app_data.roaming_settings()?;
    //let container = roaming_settings
    //     .create_container(CONTAINER_NAME, ApplicationDataCreateDisposition::Always)?;
    // if let Ok(value) = container.values()?.lookup(CONTAINER_SETTING_FIELD) {
    //     let q: IPropertyValue = value.try_into()?;
    //     let s = q.get_string()?;
    //     dbg!(&s);
    //     //        let pv: PropertyValue = Box::new(value).try_into()?;
    //     //if let Ok(pv) = value.
    // }
    // let value = PropertyValue::create_string("FOO")?;
    // container.values()?.insert(CONTAINER_SETTING_FIELD, value)?;
    /*    let value =
        if let Ok(value) = container.values()?.lookup(CONTAINER_SETTING_FIELD) {
        } else {
            let field = Field::new(4, 4);
            let s = String::new();
            let h = HString::new();
            let o: Object = h.into();
            container.values()?.insert(CONTAINER_SETTING_FIELD, s);
        }

        if let Ok(field) = roaming_settings.containers()?.lookup(CONTAINER_NAME) {
        } else {
        }
    */
    let app_data_paths = AppDataPaths::get_default()?;
    let app_data_folder = app_data_paths.local_app_data()?;
    

    //
    // Construct GUI
    //
    let mut window = MainWindow::new()?;
    window.window().set_title("2048");
    let mut panel_manager = window.create_panel_manager()?;

    // Constuct panels
    let game_field_panel = GameFieldPanel::new(&mut panel_manager)?;
    let score_panel = TextPanel::new(&mut panel_manager)?;
    let mut undo_button_panel = ButtonPanel::new(&mut panel_manager)?;
    let mut undo_button_text_panel = TextPanel::new(&mut panel_manager)?;
    let mut reset_button_panel = ButtonPanel::new(&mut panel_manager)?;
    let mut reset_button_text_panel = TextPanel::new(&mut panel_manager)?;
    //let empty_panel = EmptyPanel::new(&mut panel_manager)?;
    let mut vribbon_panel = Ribbon::new(&mut panel_manager, RibbonOrientation::Vertical)?;
    let mut hribbon_panel = Ribbon::new(&mut panel_manager, RibbonOrientation::Horizontal)?;
    let mut background_panel = BackgroundPanel::new(&mut panel_manager)?;

    //
    // Initialize panels
    //
    undo_button_text_panel.set_text("⮌")?;
    reset_button_text_panel.set_text("⭯")?;

    // Take handles
    let game_field_handle = game_field_panel.handle();
    let score_handle = score_panel.handle();
    let undo_button_handle = undo_button_panel.handle();
    let reset_button_handle = reset_button_panel.handle();
    // Join panels into tree
    undo_button_panel.add_panel(undo_button_text_panel)?;
    reset_button_panel.add_panel(reset_button_text_panel)?;
    hribbon_panel.add_panel(undo_button_panel, 1.)?;
    hribbon_panel.add_panel(score_panel, 1.)?;
    hribbon_panel.add_panel(reset_button_panel, 1.)?;
    vribbon_panel.add_panel(hribbon_panel, 1.)?;
    vribbon_panel.add_panel(game_field_panel, 4.)?;
    background_panel.add_panel(vribbon_panel)?;

    panel_manager.set_root_panel(background_panel)?;

    let mut control_manager = ControlManager::new();
    control_manager.add_control(undo_button_handle.clone());
    control_manager.add_control(reset_button_handle.clone());

    let update_buttons =
        move |panel_manager: &mut PanelManager, proxy: &PanelEventProxy| -> winrt::Result<()> {
            let game_field = panel_manager.panel(game_field_handle)?;
            let can_undo = game_field.can_undo();
            let score = game_field.get_score();
            panel_manager
                .panel(undo_button_handle)?
                .enable(proxy, can_undo)?;
            panel_manager
                .panel(score_handle)?
                .set_text(score.to_string())?;
            Ok(())
        };

    update_buttons(&mut panel_manager, window.proxy()?)?;

    window.run(move |mut event, proxy| {
        let _ = panel_manager.process_event(&event, proxy)?
            || control_manager.process_event(&mut event, &mut panel_manager, proxy)?;
        if let Event::UserEvent(ref mut e) = event {
            if undo_button_handle.extract_event(e) == Some(ButtonPanelEvent::Pressed) {
                panel_manager.panel(game_field_handle)?.undo(proxy)?;
            } else if reset_button_handle.extract_event(e) == Some(ButtonPanelEvent::Pressed) {
                panel_manager.panel(game_field_handle)?.reset(proxy)?;
            } else if game_field_handle.extract_event(e) == Some(GameFieldPanelEvent::Changed) {
                update_buttons(&mut panel_manager, proxy)?;
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
