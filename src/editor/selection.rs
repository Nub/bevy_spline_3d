use bevy::{prelude::*, window::PrimaryWindow};

use crate::spline::{ControlPointMarker, SelectedControlPoint, SelectedSpline, Spline};

use super::EditorSettings;

/// Resource tracking the current selection state.
#[derive(Resource, Default, Debug, Clone)]
pub struct SelectionState {
    /// Currently hovered control point, if any.
    pub hovered_point: Option<(Entity, usize)>,
    /// Whether we're currently dragging a point.
    pub dragging: bool,
    /// The point(s) being dragged: (spline_entity, point_index).
    pub dragged_points: Vec<(Entity, usize)>,
    /// The plane normal for drag operations (perpendicular to camera).
    pub drag_plane_normal: Vec3,
    /// The initial drag plane point (for consistent plane during drag).
    pub drag_plane_point: Vec3,
}

/// System to handle mouse picking of control points.
pub fn pick_control_points(
    settings: Res<EditorSettings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    splines: Query<(Entity, &Spline)>,
    mut selection_state: ResMut<SelectionState>,
) {
    if !settings.enabled {
        return;
    }

    // Don't update hover state while dragging
    if selection_state.dragging {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        selection_state.hovered_point = None;
        return;
    };

    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    let mut closest: Option<(Entity, usize, f32)> = None;

    for (entity, spline) in &splines {
        for (i, &point) in spline.control_points.iter().enumerate() {
            // Simple sphere-ray intersection
            let pick_radius = settings.point_radius * 2.0;
            if let Some(dist) = ray_sphere_intersect(ray.origin, ray.direction, point, pick_radius) {
                if closest.is_none() || dist < closest.unwrap().2 {
                    closest = Some((entity, i, dist));
                }
            }
        }
    }

    selection_state.hovered_point = closest.map(|(e, i, _)| (e, i));
}

fn ray_sphere_intersect(origin: Vec3, direction: Dir3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let a = direction.dot(*direction);
    let b = 2.0 * oc.dot(*direction);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        None
    } else {
        let t = (-b - discriminant.sqrt()) / (2.0 * a);
        if t > 0.0 {
            Some(t)
        } else {
            None
        }
    }
}

/// System to handle selection on mouse click.
pub fn handle_selection_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<EditorSettings>,
    selection_state: Res<SelectionState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    _splines: Query<(Entity, &Spline)>,
    selected_splines: Query<Entity, With<SelectedSpline>>,
    markers: Query<(Entity, &ControlPointMarker)>,
    selected_points: Query<Entity, With<SelectedControlPoint>>,
    _cameras: Query<&GlobalTransform, With<Camera>>,
) {
    if !settings.enabled {
        return;
    }

    // Don't process clicks while dragging
    if selection_state.dragging {
        return;
    }

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if let Some((spline_entity, point_index)) = selection_state.hovered_point {
        // Select this spline
        if !shift_held {
            // Clear other selections
            for entity in &selected_splines {
                commands.entity(entity).remove::<SelectedSpline>();
            }
            for entity in &selected_points {
                commands.entity(entity).remove::<SelectedControlPoint>();
            }
        }

        // Add selection to spline
        commands.entity(spline_entity).insert(SelectedSpline);

        // Find and select the control point marker
        for (marker_entity, marker) in &markers {
            if marker.spline_entity == spline_entity && marker.index == point_index {
                commands.entity(marker_entity).insert(SelectedControlPoint);
            }
        }
    } else if !shift_held {
        // Clicked on nothing, clear selection
        for entity in &selected_splines {
            commands.entity(entity).remove::<SelectedSpline>();
        }
        for entity in &selected_points {
            commands.entity(entity).remove::<SelectedControlPoint>();
        }
    }
}

/// System to handle dragging control points.
pub fn handle_point_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<EditorSettings>,
    mut selection_state: ResMut<SelectionState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut splines: Query<&mut Spline>,
) {
    if !settings.enabled {
        return;
    }

    // Start drag - capture the hovered point directly
    if mouse.just_pressed(MouseButton::Left) {
        if let Some((spline_entity, point_index)) = selection_state.hovered_point {
            selection_state.dragging = true;
            selection_state.dragged_points = vec![(spline_entity, point_index)];

            if let Ok((_, camera_transform)) = cameras.single() {
                selection_state.drag_plane_normal = camera_transform.forward().as_vec3();

                // Store initial plane point for consistent dragging
                if let Ok(spline) = splines.get(spline_entity) {
                    if point_index < spline.control_points.len() {
                        selection_state.drag_plane_point = spline.control_points[point_index];
                    }
                }
            }
        }
    }

    // End drag
    if mouse.just_released(MouseButton::Left) {
        selection_state.dragging = false;
        selection_state.dragged_points.clear();
    }

    // Continue drag - use stored dragged_points, not the component query
    if selection_state.dragging && !selection_state.dragged_points.is_empty() {
        let Ok(window) = windows.single() else {
            return;
        };
        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };
        let Ok((camera, camera_transform)) = cameras.single() else {
            return;
        };
        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
            return;
        };

        // Use the fixed drag plane point for consistent behavior
        let plane_point = selection_state.drag_plane_point;
        let plane_normal = selection_state.drag_plane_normal;

        for &(spline_entity, point_index) in &selection_state.dragged_points.clone() {
            if let Ok(mut spline) = splines.get_mut(spline_entity) {
                if point_index < spline.control_points.len() {
                    // Intersect ray with the fixed drag plane
                    if let Some(new_pos) = ray_plane_intersect(
                        ray.origin,
                        *ray.direction,
                        plane_point,
                        plane_normal,
                    ) {
                        spline.control_points[point_index] = new_pos;
                    }
                }
            }
        }
    }
}

fn ray_plane_intersect(
    ray_origin: Vec3,
    ray_direction: Vec3,
    plane_point: Vec3,
    plane_normal: Vec3,
) -> Option<Vec3> {
    let denom = plane_normal.dot(ray_direction);
    if denom.abs() < 1e-6 {
        return None;
    }

    let t = (plane_point - ray_origin).dot(plane_normal) / denom;
    if t >= 0.0 {
        Some(ray_origin + ray_direction * t)
    } else {
        None
    }
}
