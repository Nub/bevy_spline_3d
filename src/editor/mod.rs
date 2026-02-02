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
    /// Whether to show Bézier handle lines and CatmullRom connections.
    pub show_handle_lines: bool,
    /// Visual appearance settings for gizmos.
    pub visuals: GizmoVisuals,
    /// Color settings for editor gizmos.
    pub colors: GizmoColors,
    /// Size settings for editor gizmos.
    pub sizes: GizmoSizes,
}

/// Visual appearance settings for spline gizmos.
#[derive(Debug, Clone)]
pub struct GizmoVisuals {
    /// Number of line segments per spline segment for rendering.
    pub curve_resolution: usize,
    /// Height offset for projected spline visualization above the terrain surface.
    /// This prevents the spline gizmos from clipping into the terrain.
    pub projection_visual_offset: f32,
}

/// Color settings for spline editor gizmos.
#[derive(Debug, Clone)]
pub struct GizmoColors {
    /// Color of unselected spline curves.
    pub spline: Color,
    /// Color of selected spline curves.
    pub spline_selected: Color,
    /// Color of control points on unselected splines.
    pub point: Color,
    /// Color of control points on selected splines.
    pub point_active: Color,
    /// Color of selected control points.
    pub point_selected: Color,
    /// Color of spline endpoint control points (first and last) on unselected splines.
    pub endpoint: Color,
    /// Color of spline endpoint control points on selected splines.
    pub endpoint_active: Color,
    /// Color of Bézier handle lines and CatmullRom connection lines.
    pub handle_line: Color,
}

/// Size settings for spline editor gizmos.
#[derive(Debug, Clone)]
pub struct GizmoSizes {
    /// Base radius of control point spheres.
    pub point_radius: f32,
    /// Line width for spline curves and handles.
    pub line_width: f32,
    /// Scale multiplier for control points when spline is selected.
    pub point_selected_spline_scale: f32,
    /// Scale multiplier for control points when the point itself is selected.
    pub point_selected_scale: f32,
    /// Scale multiplier for endpoint control points.
    pub endpoint_scale: f32,
    /// Scale multiplier for endpoint control points when spline is selected.
    pub endpoint_selected_spline_scale: f32,
}

impl Default for GizmoVisuals {
    fn default() -> Self {
        Self {
            curve_resolution: 32,
            projection_visual_offset: 0.3,
        }
    }
}

impl Default for GizmoColors {
    fn default() -> Self {
        Self {
            spline: Color::srgb(0.5, 0.5, 0.5),
            spline_selected: Color::srgb(1.0, 0.8, 0.2),
            point: Color::srgb(0.3, 0.3, 0.8),
            point_active: Color::srgb(0.5, 0.5, 1.0),
            point_selected: Color::srgb(1.0, 0.4, 0.4),
            endpoint: Color::srgb(0.8, 0.2, 0.8),
            endpoint_active: Color::srgb(1.0, 0.4, 1.0),
            handle_line: Color::srgba(0.6, 0.6, 0.6, 0.5),
        }
    }
}

impl Default for GizmoSizes {
    fn default() -> Self {
        Self {
            point_radius: 0.1,
            line_width: 3.0,
            point_selected_spline_scale: 1.2,
            point_selected_scale: 1.5,
            endpoint_scale: 1.2,
            endpoint_selected_spline_scale: 1.4,
        }
    }
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_gizmos: true,
            show_handle_lines: true,
            visuals: GizmoVisuals::default(),
            colors: GizmoColors::default(),
            sizes: GizmoSizes::default(),
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

    /// Toggle handle line visibility.
    pub fn toggle_handle_lines(&mut self) {
        self.show_handle_lines = !self.show_handle_lines;
    }
}

/// System to sync editor settings to gizmo config.
fn sync_gizmo_config(
    settings: Res<EditorSettings>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = settings.sizes.line_width;
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
                    // Cache update (must run before rendering)
                    gizmos::update_spline_cache,
                    // Gizmo rendering (uses cached points)
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
                )
                    .chain(),
            );

        // Add spline projection visualization
        use bevy::transform::TransformSystems;
        // Run projection after physics and transform propagation.
        // Only runs when avian3d physics is available.
        app.add_systems(
            PostUpdate,
            gizmos::project_spline_visualization
                .after(TransformSystems::Propagate)
                .run_if(gizmos::physics_available),
        );
    }
}
