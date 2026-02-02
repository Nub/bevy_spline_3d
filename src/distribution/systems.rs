use bevy::prelude::*;

use crate::spline::Spline;
use crate::surface::SurfaceProjection;

use super::{
    DistributedInstance, DistributionOrientation, DistributionSource, DistributionSpacing,
    DistributionState, SplineDistribution,
};
use super::projection::NeedsInstanceProjection;

/// Number of samples used to compute arc length lookup table.
const ARC_LENGTH_SAMPLES: usize = 256;

/// Hide entities marked as distribution sources.
pub fn hide_source_entities(
    mut sources: Query<&mut Visibility, Added<DistributionSource>>,
) {
    for mut visibility in &mut sources {
        *visibility = Visibility::Hidden;
    }
}

/// Update distributed instances when distribution or spline changes.
#[allow(clippy::too_many_arguments)]
pub fn update_distributions(
    mut commands: Commands,
    distributions: Query<(Entity, &SplineDistribution, Option<&DistributionState>)>,
    splines: Query<&Spline>,
    sources: Query<(
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&Children>,
    )>,
    mut instances: Query<(&mut Transform, &DistributedInstance)>,
    changed_splines: Query<Entity, Changed<Spline>>,
    changed_distributions: Query<Entity, Changed<SplineDistribution>>,
    projection_query: Query<(), With<SurfaceProjection>>,
) {
    // Collect changed spline entities for quick lookup
    let changed_spline_set: std::collections::HashSet<Entity> =
        changed_splines.iter().collect();
    let changed_dist_set: std::collections::HashSet<Entity> =
        changed_distributions.iter().collect();

    for (dist_entity, distribution, state) in &distributions {
        if !distribution.enabled {
            continue;
        }

        let Ok(spline) = splines.get(distribution.spline) else {
            continue;
        };

        if !spline.is_valid() {
            continue;
        }

        // Check if we need to rebuild instances
        let needs_rebuild = state.is_none()
            || changed_dist_set.contains(&dist_entity)
            || state.as_ref().is_some_and(|s| {
                s.cached_count != distribution.count || s.cached_source != distribution.source
            });

        // Check if we need to update transforms
        let needs_transform_update =
            needs_rebuild || changed_spline_set.contains(&distribution.spline);

        // Compute t values based on spacing mode
        let t_values = match distribution.spacing {
            DistributionSpacing::Uniform => compute_uniform_t_values(spline, distribution.count),
            DistributionSpacing::Parametric => compute_parametric_t_values(distribution.count),
        };

        if needs_rebuild {
            // Despawn old instances
            if let Some(state) = state {
                for &instance_entity in &state.instances {
                    if let Ok(mut entity_commands) = commands.get_entity(instance_entity) {
                        entity_commands.despawn();
                    }
                }
            }

            // Spawn new instances
            let mut new_instances = Vec::with_capacity(distribution.count);

            // Get source components to clone
            let source_data = sources.get(distribution.source).ok();

            for (i, &t) in t_values.iter().enumerate() {
                let transform = calculate_transform(spline, t, distribution);

                let mut entity_commands = commands.spawn((
                    transform,
                    DistributedInstance {
                        distribution: dist_entity,
                        index: i,
                    },
                    Visibility::default(),
                ));

                // Clone visual components from source
                if let Some((mesh, material, _children)) = source_data {
                    if let Some(mesh) = mesh {
                        entity_commands.insert(mesh.clone());
                    }
                    if let Some(material) = material {
                        entity_commands.insert(material.clone());
                    }
                }

                // Mark for surface projection if enabled
                if projection_query.get(dist_entity).is_ok() {
                    entity_commands.insert(NeedsInstanceProjection);
                }

                new_instances.push(entity_commands.id());
            }

            // Update state
            commands.entity(dist_entity).insert(DistributionState {
                instances: new_instances,
                cached_count: distribution.count,
                cached_source: distribution.source,
            });
        } else if needs_transform_update {
            // Just update transforms on existing instances
            if let Some(state) = state {
                for (i, &instance_entity) in state.instances.iter().enumerate() {
                    if let Ok((mut transform, _)) = instances.get_mut(instance_entity) {
                        let t = t_values.get(i).copied().unwrap_or(0.5);
                        *transform = calculate_transform(spline, t, distribution);

                        // Mark for surface projection if enabled
                        if projection_query.get(dist_entity).is_ok() {
                            commands.entity(instance_entity).insert(NeedsInstanceProjection);
                        }
                    }
                }
            }
        }
    }
}

/// Compute arc-length lookup table for a spline.
/// Returns a vector of (t, cumulative_length) pairs.
fn compute_arc_length_table(spline: &Spline) -> Vec<(f32, f32)> {
    let mut table = Vec::with_capacity(ARC_LENGTH_SAMPLES + 1);
    let mut cumulative_length = 0.0;
    let mut prev_point = spline.evaluate(0.0).unwrap_or(Vec3::ZERO);

    table.push((0.0, 0.0));

    for i in 1..=ARC_LENGTH_SAMPLES {
        let t = i as f32 / ARC_LENGTH_SAMPLES as f32;
        let point = spline.evaluate(t).unwrap_or(prev_point);
        cumulative_length += (point - prev_point).length();
        table.push((t, cumulative_length));
        prev_point = point;
    }

    table
}

/// Find the t parameter for a given arc length using the lookup table.
fn arc_length_to_t(table: &[(f32, f32)], target_length: f32) -> f32 {
    if table.is_empty() {
        return 0.0;
    }

    let total_length = table.last().map(|(_, l)| *l).unwrap_or(0.0);
    if total_length <= 0.0 {
        return 0.0;
    }

    // Clamp target length
    let target = target_length.clamp(0.0, total_length);

    // Binary search for the segment containing target_length
    let idx = table
        .binary_search_by(|(_, l)| l.partial_cmp(&target).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or_else(|i| i.saturating_sub(1));

    if idx >= table.len() - 1 {
        return 1.0;
    }

    let (t0, l0) = table[idx];
    let (t1, l1) = table[idx + 1];

    if (l1 - l0).abs() < 1e-6 {
        return t0;
    }

    // Linear interpolation within segment
    let alpha = (target - l0) / (l1 - l0);
    t0 + alpha * (t1 - t0)
}

/// Compute t values for uniform distribution.
fn compute_uniform_t_values(spline: &Spline, count: usize) -> Vec<f32> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![0.5];
    }

    let table = compute_arc_length_table(spline);
    let total_length = table.last().map(|(_, l)| *l).unwrap_or(0.0);

    (0..count)
        .map(|i| {
            let target_length = (i as f32 / (count - 1) as f32) * total_length;
            arc_length_to_t(&table, target_length)
        })
        .collect()
}

/// Compute t values for parametric distribution.
fn compute_parametric_t_values(count: usize) -> Vec<f32> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![0.5];
    }

    (0..count)
        .map(|i| i as f32 / (count - 1) as f32)
        .collect()
}

/// Calculate transform for a distributed instance at parameter t.
fn calculate_transform(spline: &Spline, t: f32, distribution: &SplineDistribution) -> Transform {
    let position = spline.evaluate(t).unwrap_or(Vec3::ZERO);

    let rotation = match distribution.orientation {
        DistributionOrientation::PositionOnly => Quat::IDENTITY,
        DistributionOrientation::AlignToTangent { up } => {
            if let Some(tangent) = spline.evaluate_tangent(t) {
                let tangent = tangent.normalize_or_zero();
                if tangent.length_squared() > 0.001 {
                    // Create rotation that points -Z along tangent (forward in Bevy)
                    let forward = -tangent;
                    let right = up.cross(forward).normalize_or_zero();
                    if right.length_squared() > 0.001 {
                        let corrected_up = forward.cross(right).normalize();
                        Quat::from_mat3(&Mat3::from_cols(right, corrected_up, forward))
                    } else {
                        Quat::IDENTITY
                    }
                } else {
                    Quat::IDENTITY
                }
            } else {
                Quat::IDENTITY
            }
        }
    };

    // Apply offset in local space
    let offset = rotation * distribution.offset;

    Transform {
        translation: position + offset,
        rotation,
        scale: Vec3::ONE,
    }
}

/// Cleanup instances when distribution is removed.
pub fn cleanup_distributions(
    mut commands: Commands,
    mut removed: RemovedComponents<SplineDistribution>,
    states: Query<&DistributionState>,
    instances: Query<(Entity, &DistributedInstance)>,
) {
    for removed_dist in removed.read() {
        // Try to get state for cleanup
        if let Ok(state) = states.get(removed_dist) {
            for &instance_entity in &state.instances {
                if let Ok(mut entity_commands) = commands.get_entity(instance_entity) {
                    entity_commands.despawn();
                }
            }
        }

        // Also cleanup any instances that reference this distribution
        // (in case state wasn't available)
        for (entity, instance) in &instances {
            if instance.distribution == removed_dist {
                commands.entity(entity).despawn();
            }
        }
    }
}
