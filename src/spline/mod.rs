mod components;
mod projection;
mod types;

pub use components::*;
pub use projection::{
    get_effective_control_points, get_effective_curve_points, project_spline_point,
    ProjectedSplineCache, SplineProjectionConfig,
};
pub use types::*;

use bevy::prelude::*;

/// Plugin that registers spline types for reflection/serialization.
/// This plugin does NOT include editor functionality - use `SplineEditorPlugin` for that.
pub struct SplinePlugin;

impl Plugin for SplinePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SplineType>()
            .register_type::<Spline>()
            .register_type::<SelectedSpline>()
            .register_type::<ControlPointMarker>()
            .register_type::<SelectedControlPoint>();
    }
}
