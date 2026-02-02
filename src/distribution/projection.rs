//! Surface projection for distributed instances.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::surface::SurfaceProjection;

use super::{DistributedInstance, SplineDistribution};

/// Marker component for instances that need projection.
/// Added when instances are created/updated, removed after projection.
#[derive(Component, Debug, Clone, Copy)]
pub struct NeedsInstanceProjection;

/// System to project distributed instances onto surfaces below.
pub fn project_distributed_instances(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    distributions: Query<&SurfaceProjection, With<SplineDistribution>>,
    mut instances: Query<(Entity, &DistributedInstance, &mut Transform), With<NeedsInstanceProjection>>,
) {
    for (instance_entity, instance, mut transform) in &mut instances {
        let Ok(config) = distributions.get(instance.distribution) else {
            commands.entity(instance_entity).remove::<NeedsInstanceProjection>();
            continue;
        };

        if !config.enabled {
            commands.entity(instance_entity).remove::<NeedsInstanceProjection>();
            continue;
        }

        let position = transform.translation;
        let ray_origin = position + Vec3::Y * config.ray_origin_offset;
        let ray_direction = Dir3::NEG_Y;

        let filter = if let Some(layers) = config.collision_layers {
            SpatialQueryFilter::default().with_mask(layers)
        } else {
            SpatialQueryFilter::default()
        };

        if let Some(hit) = spatial_query.cast_ray(
            ray_origin,
            ray_direction,
            config.max_distance,
            true,
            &filter,
        ) {
            let hit_position = ray_origin + *ray_direction * hit.distance;
            // Offset along the surface normal to prevent z-fighting
            let adjusted_position = hit_position + hit.normal * config.normal_offset;
            transform.translation = adjusted_position;

            // Optionally align rotation to surface normal
            if config.align_to_normal {
                let normal = hit.normal;
                let forward = transform.forward();
                let right = normal.cross(*forward).normalize_or_zero();
                if right.length_squared() > 0.001 {
                    let corrected_forward = right.cross(normal).normalize();
                    transform.rotation =
                        Quat::from_mat3(&Mat3::from_cols(right, normal, corrected_forward));
                }
            }

            // Projection succeeded - remove marker
            commands.entity(instance_entity).remove::<NeedsInstanceProjection>();
        }
        // If no hit, keep marker to retry next frame (physics might not be ready)
    }
}
