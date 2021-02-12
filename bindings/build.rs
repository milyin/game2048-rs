fn main() {
    windows::build!(
        windows::foundation::numerics::{Vector2, Vector3},
        windows::foundation::TimeSpan,
        windows::graphics::SizeInt32,
        windows::system::DispatcherQueueController,
        windows::win32::system_services::CreateDispatcherQueueController,
        windows::win32::system_services::DispatcherQueueOptions,
        windows::ui::composition::{
            AnimationIterationBehavior,
            CompositionBatchTypes,
            CompositionBorderMode,
            CompositionColorBrush,
            CompositionGeometry,
            CompositionShape,
            CompositionSpriteShape,
            Compositor,
            ContainerVisual,
            SpriteVisual,
        },
        windows::ui::composition::desktop::DesktopWindowTarget,
        windows::ui::{Colors, ColorHelper},
        windows::win32::winrt::{ICompositorDesktopInterop, RoInitialize},
        microsoft::graphics::canvas::{CanvasDevice},
        microsoft::graphics::canvas::text::*,
        microsoft::graphics::canvas::ui::composition::*,
    );
}
