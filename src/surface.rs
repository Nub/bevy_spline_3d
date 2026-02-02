//! Surface projection for conforming spline-based geometry to terrain.
//!
//! This module provides components and utilities for projecting roads and
//! distributed objects onto surfaces using raycasting via avian3d physics.

use avian3d::prelude::*;
use bevy::prelude::*;

/// Configuration for projecting geometry onto surfaces.
///
/// Add this component to a `SplineRoad` or `SplineDistribution` entity
/// to make it conform to terrain below.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct SurfaceProjection {
    /// Whether surface projection is enabled.
    pub enabled: bool,
    /// Offset above the spline point to start the raycast from.
    /// Increase this if your spline passes through terrain.
    pub ray_origin_offset: f32,
    /// Maximum distance to cast the ray downward.
    pub max_distance: f32,
    /// Offset along the surface normal to prevent z-fighting.
    /// Applied in the direction of the hit normal.
    pub normal_offset: f32,
    /// Whether to align object rotation to surface normal (distribution only).
    pub align_to_normal: bool,
    /// Optional collision layers to query against.
    /// If None, all layers are queried.
    #[reflect(ignore)]
    pub collision_layers: Option<LayerMask>,
}

impl Default for SurfaceProjection {
    fn default() -> Self {
        Self {
            enabled: true,
            ray_origin_offset: 10.0,
            max_distance: 100.0,
            normal_offset: 0.1,
            align_to_normal: false,
            collision_layers: None,
        }
    }
}

impl SurfaceProjection {
    /// Create a new surface projection with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the ray origin offset.
    pub fn with_ray_offset(mut self, offset: f32) -> Self {
        self.ray_origin_offset = offset;
        self
    }

    /// Set the maximum raycast distance.
    pub fn with_max_distance(mut self, distance: f32) -> Self {
        self.max_distance = distance;
        self
    }

    /// Set the normal offset to prevent z-fighting.
    pub fn with_normal_offset(mut self, offset: f32) -> Self {
        self.normal_offset = offset;
        self
    }

    /// Enable alignment to surface normal.
    pub fn with_normal_alignment(mut self, align: bool) -> Self {
        self.align_to_normal = align;
        self
    }

    /// Set collision layers to query.
    pub fn with_layers(mut self, layers: LayerMask) -> Self {
        self.collision_layers = Some(layers);
        self
    }
}

/// Collision layers for surface projection.
#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum ProjectionLayer {
    /// Layer for terrain/surfaces that can be projected onto.
    #[default]
    Terrain,
}

/// Result of a surface projection query.
#[derive(Debug, Clone)]
pub struct ProjectionHit {
    /// The projected position on the surface.
    pub position: Vec3,
    /// The surface normal at the hit point.
    pub normal: Vec3,
}

/// Project a point onto the surface below it.
///
/// Returns `None` if no surface is found within the max distance.
pub fn project_point(
    spatial_query: &SpatialQuery,
    point: Vec3,
    config: &SurfaceProjection,
) -> Option<ProjectionHit> {
    if !config.enabled {
        return None;
    }

    let ray_origin = point + Vec3::Y * config.ray_origin_offset;
    let ray_direction = Dir3::NEG_Y;

    let filter = if let Some(layers) = config.collision_layers {
        SpatialQueryFilter::default().with_mask(layers)
    } else {
        SpatialQueryFilter::default()
    };

    let hit = spatial_query.cast_ray(
        ray_origin,
        ray_direction,
        config.max_distance,
        true,
        &filter,
    )?;

    let hit_position = ray_origin + *ray_direction * hit.distance;
    // Offset along the surface normal to prevent z-fighting
    let adjusted_position = hit_position + hit.normal * config.normal_offset;

    Some(ProjectionHit {
        position: adjusted_position,
        normal: hit.normal,
    })
}

/// Project a point onto the surface, returning the original if no hit.
pub fn project_point_or_original(
    spatial_query: &SpatialQuery,
    point: Vec3,
    config: &SurfaceProjection,
) -> Vec3 {
    project_point(spatial_query, point, config)
        .map(|hit| hit.position)
        .unwrap_or(point)
}

/// Plugin for surface projection functionality.
///
/// This plugin registers the `SurfaceProjection` component and integrates
/// with the road and distribution systems when the `surface_projection`
/// feature is enabled.
pub struct SurfaceProjectionPlugin;

impl Plugin for SurfaceProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SurfaceProjection>();
    }
}
