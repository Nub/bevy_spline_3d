mod gizmos;
mod input;
mod selection;

pub use selection::SelectionState;

use bevy::{gizmos::config::GizmoConfigStore, prelude::*};

/// Settings for the spline editor.
#[derive(Resource, Debug, Clone)]
pub struct EditorSettings {
    /// Whether the editor is enabled (responds to input).
    pub enabled: bool,
    /// Whether to show gizmos (spline curves and control points).
    pub show_gizmos: bool,
    /// Whether to show Bézier handle lines.
    pub show_bezier_handles: bool,
    /// Number of line segments per spline segment for rendering.
    pub curve_resolution: usize,
    /// Radius of control point spheres.
    pub point_radius: f32,
    /// Line width for spline curves and handles.
    pub line_width: f32,
    /// Color of unselected spline curves.
    pub spline_color: Color,
    /// Color of selected spline curves.
    pub selected_spline_color: Color,
    /// Color of control points on unselected splines.
    pub point_color: Color,
    /// Color of control points on selected splines.
    pub active_point_color: Color,
    /// Color of selected control points.
    pub selected_point_color: Color,
    /// Color of Bézier handle lines.
    pub handle_line_color: Color,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_gizmos: true,
            show_bezier_handles: true,
            curve_resolution: 32,
            point_radius: 0.1,
            line_width: 3.0,
            spline_color: Color::srgb(0.5, 0.5, 0.5),
            selected_spline_color: Color::srgb(1.0, 0.8, 0.2),
            point_color: Color::srgb(0.3, 0.3, 0.8),
            active_point_color: Color::srgb(0.5, 0.5, 1.0),
            selected_point_color: Color::srgb(1.0, 0.4, 0.4),
            handle_line_color: Color::srgba(0.6, 0.6, 0.6, 0.5),
        }
    }
}

impl EditorSettings {
    /// Toggle the editor on/off.
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Toggle gizmo visibility.
    pub fn toggle_gizmos(&mut self) {
        self.show_gizmos = !self.show_gizmos;
    }
}

/// System to sync editor settings to gizmo config.
fn sync_gizmo_config(
    settings: Res<EditorSettings>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line_width = settings.line_width;
}

/// Plugin that adds interactive spline editing functionality.
///
/// This plugin requires `SplinePlugin` to be added first.
///
/// # Features
/// - Visual gizmos for spline curves and control points
/// - Mouse picking and dragging of control points
/// - Hotkeys for adding/removing points, changing spline type, etc.
///
/// # Hotkeys
/// - `A`: Add control point after selection
/// - `X`: Delete selected control point(s)
/// - `Tab`: Cycle spline type
/// - `C`: Toggle closed/open spline
/// - `Escape`: Deselect all
///
/// # Disabling
/// Use the `EditorSettings` resource to enable/disable the editor:
/// ```ignore
/// fn toggle_editor(mut settings: ResMut<EditorSettings>) {
///     settings.toggle();
/// }
/// ```
pub struct SplineEditorPlugin;

impl Plugin for SplineEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorSettings>()
            .init_resource::<SelectionState>()
            .add_systems(
                Update,
                (
                    // Config sync
                    sync_gizmo_config,
                    // Gizmo rendering
                    gizmos::render_spline_curves,
                    gizmos::render_control_points,
                    gizmos::sync_control_point_entities,
                    gizmos::cleanup_orphaned_markers,
                    // Selection
                    selection::pick_control_points,
                    selection::handle_selection_click,
                    selection::handle_point_drag,
                    // Input
                    input::handle_hotkeys,
                ),
            );
    }
}
