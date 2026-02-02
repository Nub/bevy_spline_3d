//! Road intersection generation.
//!
//! This module provides components and systems for creating intersections
//! where multiple roads meet.

use bevy::{
    prelude::*,
    mesh::PrimitiveTopology,
};

use crate::spline::Spline;
use super::SplineRoad;

/// Which end of a road connects to an intersection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum RoadEnd {
    /// The start of the road (t = 0).
    #[default]
    Start,
    /// The end of the road (t = 1).
    End,
}

impl RoadEnd {
    /// Get the t parameter for this end.
    pub fn t(&self) -> f32 {
        match self {
            RoadEnd::Start => 0.0,
            RoadEnd::End => 1.0,
        }
    }

    /// Get the direction multiplier (1 for outward from intersection, -1 for inward).
    pub fn direction(&self) -> f32 {
        match self {
            RoadEnd::Start => -1.0, // Tangent points away, we want toward intersection
            RoadEnd::End => 1.0,    // Tangent points toward intersection
        }
    }
}

/// A connection between a road and an intersection.
#[derive(Debug, Clone, Reflect)]
pub struct RoadConnection {
    /// The SplineRoad entity.
    pub road: Entity,
    /// Which end of the road connects here.
    pub end: RoadEnd,
}

impl RoadConnection {
    /// Create a new road connection.
    pub fn new(road: Entity, end: RoadEnd) -> Self {
        Self { road, end }
    }

    /// Connect to the start of a road.
    pub fn start(road: Entity) -> Self {
        Self::new(road, RoadEnd::Start)
    }

    /// Connect to the end of a road.
    pub fn end(road: Entity) -> Self {
        Self::new(road, RoadEnd::End)
    }
}

/// Component that defines a road intersection.
///
/// An intersection is a point where multiple roads meet. The system will
/// generate geometry to fill the gap between the connected roads.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct RoadIntersection {
    /// The roads connected to this intersection.
    pub connections: Vec<RoadConnection>,
    /// Whether to automatically update when connected roads change.
    pub auto_update: bool,
    /// Radius of the intersection (how far the blend extends).
    /// If None, calculated automatically from road widths.
    pub radius: Option<f32>,
}

impl Default for RoadIntersection {
    fn default() -> Self {
        Self {
            connections: Vec::new(),
            auto_update: true,
            radius: None,
        }
    }
}

impl RoadIntersection {
    /// Create a new intersection with the given road connections.
    pub fn new(connections: Vec<RoadConnection>) -> Self {
        Self {
            connections,
            ..default()
        }
    }

    /// Add a road connection.
    pub fn with_connection(mut self, road: Entity, end: RoadEnd) -> Self {
        self.connections.push(RoadConnection::new(road, end));
        self
    }

    /// Set a fixed radius for the intersection.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
}

/// Marker component for generated intersection mesh entities.
#[derive(Component, Debug, Clone, Copy)]
pub struct GeneratedIntersectionMesh {
    /// The RoadIntersection entity this mesh belongs to.
    pub intersection: Entity,
}

/// Information about a road endpoint for intersection generation.
#[derive(Debug, Clone)]
pub struct RoadEndpoint {
    /// World position at the road end.
    pub position: Vec3,
    /// Tangent direction (pointing into the intersection).
    #[allow(dead_code)]
    pub tangent: Vec3,
    /// Right vector (perpendicular to tangent, in road plane).
    pub right: Vec3,
    /// Half-width of the road.
    pub half_width: f32,
    /// The angle around the intersection center.
    pub angle: f32,
}

/// Generate intersection mesh where roads meet.
pub fn generate_intersection_mesh(
    endpoints: &[RoadEndpoint],
    center: Vec3,
) -> Option<Mesh> {
    if endpoints.len() < 2 {
        return None;
    }

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Add center vertex
    positions.push([center.x, center.y, center.z]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    // Add edge vertices for each road (left and right edge points)
    for endpoint in endpoints {
        let left = endpoint.position + endpoint.right * endpoint.half_width;
        let right = endpoint.position - endpoint.right * endpoint.half_width;

        positions.push([left.x, left.y, left.z]);
        normals.push([0.0, 1.0, 0.0]);

        positions.push([right.x, right.y, right.z]);
        normals.push([0.0, 1.0, 0.0]);

        // UV based on position relative to center
        let left_dir = (left - center).normalize_or_zero();
        let right_dir = (right - center).normalize_or_zero();
        uvs.push([0.5 + left_dir.x * 0.5, 0.5 + left_dir.z * 0.5]);
        uvs.push([0.5 + right_dir.x * 0.5, 0.5 + right_dir.z * 0.5]);
    }

    // Create triangles connecting center to edge vertices
    // Vertices are: 0=center, then pairs of (left, right) for each road
    // Use CCW winding for upward-facing normals
    let num_roads = endpoints.len();
    for i in 0..num_roads {
        let right_idx = 2 + i * 2;
        let next_left_idx = 1 + ((i + 1) % num_roads) * 2;

        // Triangle from center to next left edge to current right edge
        // This fills the gap between roads (CCW winding)
        indices.push(0);
        indices.push(next_left_idx as u32);
        indices.push(right_idx as u32);
    }

    // Also create triangles for the road entry areas
    for i in 0..num_roads {
        let left_idx = 1 + i * 2;
        let right_idx = 2 + i * 2;

        // Triangle from center to right edge to left edge (road entry, CCW winding)
        indices.push(0);
        indices.push(right_idx as u32);
        indices.push(left_idx as u32);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    Some(mesh)
}

/// System to update intersection meshes when roads change.
pub fn update_intersection_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    intersections: Query<(
        Entity,
        &RoadIntersection,
        Option<&MeshMaterial3d<StandardMaterial>>,
    )>,
    roads: Query<&SplineRoad>,
    splines: Query<&Spline>,
    changed_splines: Query<Entity, Changed<Spline>>,
    changed_roads: Query<Entity, Changed<SplineRoad>>,
    changed_intersections: Query<Entity, Changed<RoadIntersection>>,
    existing_meshes: Query<(Entity, &GeneratedIntersectionMesh)>,
    children_query: Query<&Children>,
) {
    let changed_spline_set: std::collections::HashSet<Entity> =
        changed_splines.iter().collect();
    let changed_road_set: std::collections::HashSet<Entity> =
        changed_roads.iter().collect();
    let changed_intersection_set: std::collections::HashSet<Entity> =
        changed_intersections.iter().collect();

    for (intersection_entity, intersection, material) in &intersections {
        // Check if we need to update
        let needs_update = changed_intersection_set.contains(&intersection_entity)
            || (intersection.auto_update && intersection.connections.iter().any(|conn| {
                changed_road_set.contains(&conn.road)
                    || roads.get(conn.road).ok().map_or(false, |road| {
                        changed_spline_set.contains(&road.spline)
                    })
            }));

        if !needs_update {
            continue;
        }

        // Gather endpoint information for each connected road
        let mut endpoints: Vec<RoadEndpoint> = Vec::new();
        let mut center = Vec3::ZERO;
        let mut max_half_width: f32 = 2.0; // Default width

        for conn in &intersection.connections {
            let Ok(road) = roads.get(conn.road) else {
                continue;
            };

            let Ok(spline) = splines.get(road.spline) else {
                continue;
            };

            if !spline.is_valid() {
                continue;
            }

            let t = conn.end.t();
            let Some(position) = spline.evaluate(t) else {
                continue;
            };

            let tangent = spline
                .evaluate_tangent(t)
                .map(|t| t.normalize_or_zero() * conn.end.direction())
                .unwrap_or(Vec3::Z);

            // Calculate right vector (perpendicular to tangent in XZ plane)
            let up = Vec3::Y;
            let right = tangent.cross(up).normalize_or_zero();

            // Get road width from segment mesh (estimate based on typical roads)
            // TODO: Extract actual width from segment mesh
            let half_width = 2.0;
            max_half_width = max_half_width.max(half_width);

            center += position;

            endpoints.push(RoadEndpoint {
                position,
                tangent,
                right,
                half_width,
                angle: 0.0, // Will be calculated below
            });
        }

        if endpoints.len() < 2 {
            continue;
        }

        // Calculate center point
        center /= endpoints.len() as f32;

        // Calculate angles for each endpoint relative to center
        for endpoint in &mut endpoints {
            let dir = (endpoint.position - center).normalize_or_zero();
            endpoint.angle = dir.z.atan2(dir.x);
        }

        // Sort endpoints by angle for proper mesh generation
        endpoints.sort_by(|a, b| a.angle.partial_cmp(&b.angle).unwrap());

        // Generate the intersection mesh
        let Some(mesh) = generate_intersection_mesh(&endpoints, center) else {
            continue;
        };

        let mesh_handle = meshes.add(mesh);

        // Find or create the mesh entity
        let mut found_mesh_entity = None;
        if let Ok(children) = children_query.get(intersection_entity) {
            for child in children.iter() {
                if existing_meshes.get(child).is_ok() {
                    found_mesh_entity = Some(child);
                    break;
                }
            }
        }

        if let Some(mesh_entity) = found_mesh_entity {
            commands.entity(mesh_entity).insert(Mesh3d(mesh_handle));
            if let Some(mat) = material {
                commands.entity(mesh_entity).insert(mat.clone());
            }
        } else {
            let mut entity_commands = commands.spawn((
                Mesh3d(mesh_handle),
                Transform::default(),
                Visibility::default(),
                GeneratedIntersectionMesh {
                    intersection: intersection_entity,
                },
            ));

            if let Some(mat) = material {
                entity_commands.insert(mat.clone());
            }

            let mesh_entity = entity_commands.id();
            commands.entity(intersection_entity).add_child(mesh_entity);
        }
    }
}

/// Cleanup intersection meshes when intersection is removed.
pub fn cleanup_intersection_meshes(
    mut commands: Commands,
    mut removed: RemovedComponents<RoadIntersection>,
    meshes: Query<(Entity, &GeneratedIntersectionMesh)>,
) {
    for removed_intersection in removed.read() {
        for (entity, mesh) in &meshes {
            if mesh.intersection == removed_intersection {
                commands.entity(entity).despawn();
            }
        }
    }
}
