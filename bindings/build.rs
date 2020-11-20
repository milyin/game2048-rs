winrt::build!(
    dependencies
        os
        nuget: Win2D.uwp
    types
        windows::foundation::PropertyValue
        windows::foundation::numerics::{Vector2, Vector3}
        windows::foundation::TimeSpan
        windows::graphics::SizeInt32
        windows::system::DispatcherQueueController
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
        }
        windows::ui::composition::desktop::DesktopWindowTarget
        windows::ui::Color
        windows::ui::Colors
        windows::ui::ColorHelper
        windows::storage::*
        microsoft::graphics::canvas::{CanvasDevice}
        microsoft::graphics::canvas::text::*
        microsoft::graphics::canvas::ui::composition::*
);

fn main() {
    build();
}
