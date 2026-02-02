use bevy::prelude::*;

use crate::spline::{ControlPointMarker, SelectedControlPoint, SelectedSpline, Spline, SplineType};

use super::EditorSettings;

/// Cached sampled points for a spline curve.
///
/// This component stores pre-computed sample points to avoid
/// resampling the spline every frame during gizmo rendering.
#[derive(Component, Default, Clone)]
pub struct CachedSplineCurve {
    /// The sampled points along the curve.
    pub points: Vec<Vec3>,
    /// The resolution used when sampling (to detect settings changes).
    pub resolution: usize,
}

/// System to update cached spline curves when splines change.
pub fn update_spline_cache(
    mut commands: Commands,
    settings: Res<EditorSettings>,
    changed_splines: Query<(Entity, &Spline), Changed<Spline>>,
    mut cached: Query<(Entity, &Spline, &mut CachedSplineCurve)>,
    uncached: Query<(Entity, &Spline), Without<CachedSplineCurve>>,
) {
    // Add cache to splines that don't have one
    for (entity, spline) in &uncached {
        let points = if spline.is_valid() {
            spline.sample(settings.curve_resolution)
        } else {
            Vec::new()
        };
        commands.entity(entity).insert(CachedSplineCurve {
            points,
            resolution: settings.curve_resolution,
        });
    }

    // Update cache for changed splines
    for (entity, spline) in &changed_splines {
        if let Ok((_, _, mut cache)) = cached.get_mut(entity) {
            cache.points = if spline.is_valid() {
                spline.sample(settings.curve_resolution)
            } else {
                Vec::new()
            };
            cache.resolution = settings.curve_resolution;
        }
    }

    // Update cache if resolution changed
    if settings.is_changed() {
        for (_, spline, mut cache) in &mut cached {
            if cache.resolution != settings.curve_resolution {
                cache.points = if spline.is_valid() {
                    spline.sample(settings.curve_resolution)
                } else {
                    Vec::new()
                };
                cache.resolution = settings.curve_resolution;
            }
        }
    }
}

/// System to render spline curves using Bevy gizmos.
///
/// Uses cached sample points from [`CachedSplineCurve`] to avoid
/// resampling splines every frame.
pub fn render_spline_curves(
    settings: Res<EditorSettings>,
    splines: Query<(&Spline, Option<&SelectedSpline>, Option<&CachedSplineCurve>)>,
    mut gizmos: Gizmos,
) {
    if !settings.show_gizmos {
        return;
    }

    for (spline, selected, cache) in &splines {
        if !spline.is_valid() {
            continue;
        }

        let color = if selected.is_some() {
            settings.selected_spline_color
        } else {
            settings.spline_color
        };

        // Use cached points if available, otherwise sample (fallback for first frame)
        let points: Vec<Vec3>;
        let points_ref = if let Some(cache) = cache {
            &cache.points
        } else {
            points = spline.sample(settings.curve_resolution);
            &points
        };

        for window in points_ref.windows(2) {
            gizmos.line(window[0], window[1], color);
        }

        // For closed splines, connect last to first
        if spline.closed && points_ref.len() >= 2 {
            gizmos.line(points_ref[points_ref.len() - 1], points_ref[0], color);
        }

        // Render Bézier handle lines
        if spline.spline_type == SplineType::CubicBezier && settings.show_bezier_handles {
            render_bezier_handles(&spline.control_points, &settings, &mut gizmos);
        }
    }
}

fn render_bezier_handles(points: &[Vec3], settings: &EditorSettings, gizmos: &mut Gizmos) {
    if points.len() < 4 {
        return;
    }

    let num_segments = (points.len() - 1) / 3;
    for seg in 0..num_segments {
        let i = seg * 3;
        if i + 3 < points.len() {
            // Line from anchor to handle
            gizmos.line(points[i], points[i + 1], settings.handle_line_color);
            gizmos.line(points[i + 3], points[i + 2], settings.handle_line_color);
        }
    }
}

/// System to render control point spheres.
pub fn render_control_points(
    settings: Res<EditorSettings>,
    splines: Query<(Entity, &Spline, Option<&SelectedSpline>)>,
    selected_points: Query<&ControlPointMarker, With<SelectedControlPoint>>,
    mut gizmos: Gizmos,
) {
    if !settings.show_gizmos {
        return;
    }

    // Collect selected point indices per spline
    let mut selected_indices: std::collections::HashMap<Entity, std::collections::HashSet<usize>> =
        std::collections::HashMap::new();
    for marker in &selected_points {
        selected_indices
            .entry(marker.spline_entity)
            .or_default()
            .insert(marker.index);
    }

    for (entity, spline, spline_selected) in &splines {
        let entity_selected = selected_indices.get(&entity);

        for (i, &point) in spline.control_points.iter().enumerate() {
            let is_selected = entity_selected.is_some_and(|indices| indices.contains(&i));

            let color = if is_selected {
                settings.selected_point_color
            } else if spline_selected.is_some() {
                settings.active_point_color
            } else {
                settings.point_color
            };

            let radius = if is_selected {
                settings.point_radius * 1.3
            } else {
                settings.point_radius
            };

            gizmos.sphere(Isometry3d::from_translation(point), radius, color);
        }
    }
}

/// Sync control point marker entities with spline control points.
pub fn sync_control_point_entities(
    mut commands: Commands,
    splines: Query<(Entity, &Spline), Changed<Spline>>,
    existing_markers: Query<(Entity, &ControlPointMarker)>,
) {
    for (spline_entity, spline) in &splines {
        // Remove old markers for this spline
        for (marker_entity, marker) in &existing_markers {
            if marker.spline_entity == spline_entity {
                commands.entity(marker_entity).despawn();
            }
        }

        // Create new markers
        for (index, _) in spline.control_points.iter().enumerate() {
            commands.spawn(ControlPointMarker {
                spline_entity,
                index,
            });
        }
    }
}

/// Clean up control point markers when spline entities are removed.
pub fn cleanup_orphaned_markers(
    mut commands: Commands,
    mut removed: RemovedComponents<Spline>,
    markers: Query<(Entity, &ControlPointMarker)>,
) {
    for removed_spline in removed.read() {
        for (marker_entity, marker) in &markers {
            if marker.spline_entity == removed_spline {
                commands.entity(marker_entity).despawn();
            }
        }
    }
}
