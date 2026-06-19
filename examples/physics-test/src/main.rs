//! A minimal *rendered* Jolt collision test.
//!
//! Opens a window with a camera and a light, lays down a static floor, and drops
//! a handful of dynamic spheres. The spheres fall under gravity and visibly
//! collide with the floor (and each other), driven by the engine's physics
//! plugin. All meshes are generated in code, so no asset files are needed.
//!
//! Run it with `cargo run -p physics-test` (requires a display/GPU).

use std::f32::consts::FRAC_PI_4;

use game_engine::{
    app::App,
    ecs::{command::CommandQueue, resource::Res, resource::ResMut, system::schedule::UpdateGroup},
    essential::{assets::asset_server::AssetServer, transform::Transform},
    jolt_physics::{physics_state::PhysicsState, rigid_body::RigidBody},
    render::{
        assets::{material::StandardMaterial, mesh::Mesh, vertex::Vertex},
        components::{
            camera::Camera,
            light::{Light, LightType},
            material_component::MaterialComponent,
            mesh_component::MeshComponent,
        },
    },
    DefaultPlugins,
};
use glam::{Quat, Vec3, Vec4};

const SPHERE_RADIUS: f32 = 1.0;

fn main() {
    env_logger::init();

    let mut app = App::new();
    app.register_plugin(DefaultPlugins::default())
        .add_system(UpdateGroup::Startup, spawn_scene);
    app.run();
}

fn spawn_scene(
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
    mut physics: ResMut<PhysicsState>,
) {
    // Camera: pulled back and up, pitched slightly down to frame the floor.
    cmd.spawn((
        Camera::perspective(FRAC_PI_4, 16.0 / 9.0),
        Transform::from_translation_rotation(
            Vec3::new(0.0, 5.0, 14.0),
            Quat::from_rotation_x(-0.25),
        ),
    ));

    // Light above the scene.
    cmd.spawn((
        Light {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            intensity: 10.0,
            light_type: LightType::Point,
        },
        Transform::from_translation_rotation(Vec3::new(0.0, 8.0, 4.0), Quat::IDENTITY),
    ));

    // Static floor: collider top surface sits at y = 0, with a matching plane mesh.
    let floor_transform =
        Transform::from_translation_rotation(Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);
    physics.make_cuboid(20.0, 0.5, 20.0, &floor_transform, None);

    let floor_mesh = asset_server.add(make_plane(40.0, 40.0));
    let floor_material = asset_server
        .add(StandardMaterial::new(None, None).with_base_color_factor(Vec4::new(0.4, 0.4, 0.45, 1.0)));
    cmd.spawn((
        MeshComponent { handle: floor_mesh },
        MaterialComponent {
            handle: floor_material,
        },
        floor_transform,
    ));

    // Dynamic spheres dropped from increasing heights, nudged off-axis so they
    // tumble and collide rather than stack perfectly.
    let sphere_mesh = asset_server.add(make_uv_sphere(SPHERE_RADIUS, 16, 32));
    let colors = [
        Vec4::new(0.9, 0.2, 0.2, 1.0),
        Vec4::new(0.2, 0.8, 0.3, 1.0),
        Vec4::new(0.2, 0.4, 0.9, 1.0),
        Vec4::new(0.9, 0.8, 0.2, 1.0),
        Vec4::new(0.8, 0.3, 0.8, 1.0),
    ];

    for (i, color) in colors.into_iter().enumerate() {
        let height = 4.0 + i as f32 * 3.0;
        let offset = (i as f32 - 2.0) * 0.4;
        let transform = Transform::from_translation_rotation(
            Vec3::new(offset, height, offset * 0.5),
            Quat::IDENTITY,
        );

        let body = RigidBody::new(&transform, &mut physics);
        physics.make_sphere(&body, SPHERE_RADIUS);

        let material = asset_server.add(StandardMaterial::new(None, None).with_base_color_factor(color));
        cmd.spawn((
            body,
            MeshComponent {
                handle: sphere_mesh.clone(),
            },
            MaterialComponent { handle: material },
            transform,
        ));
    }
}

/// Builds a flat `width` x `length` plane in the XZ plane (centered at the origin)
/// with upward-facing normals.
fn make_plane(width: f32, length: f32) -> Mesh {
    let hw = width / 2.0;
    let hl = length / 2.0;
    let normal = [0.0, 1.0, 0.0];
    let vertices = vec![
        Vertex {
            pos_coords: [-hw, 0.0, -hl],
            normal,
            ..Vertex::default()
        },
        Vertex {
            pos_coords: [hw, 0.0, -hl],
            normal,
            ..Vertex::default()
        },
        Vertex {
            pos_coords: [hw, 0.0, hl],
            normal,
            ..Vertex::default()
        },
        Vertex {
            pos_coords: [-hw, 0.0, hl],
            normal,
            ..Vertex::default()
        },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];

    let mut mesh = Mesh { vertices, indices };
    mesh.compute_tangents();
    mesh
}

/// Builds a UV sphere of the given radius with analytic normals.
fn make_uv_sphere(radius: f32, rings: u32, segments: u32) -> Mesh {
    let mut vertices = Vec::with_capacity(((rings + 1) * (segments + 1)) as usize);
    let mut indices = Vec::with_capacity((rings * segments * 6) as usize);

    for ring in 0..=rings {
        let phi = std::f32::consts::PI * ring as f32 / rings as f32;
        let (sin_phi, cos_phi) = phi.sin_cos();
        for segment in 0..=segments {
            let theta = std::f32::consts::TAU * segment as f32 / segments as f32;
            let (sin_theta, cos_theta) = theta.sin_cos();
            let normal = [sin_phi * cos_theta, cos_phi, sin_phi * sin_theta];
            vertices.push(Vertex {
                pos_coords: [normal[0] * radius, normal[1] * radius, normal[2] * radius],
                normal,
                ..Vertex::default()
            });
        }
    }

    let stride = segments + 1;
    for ring in 0..rings {
        for segment in 0..segments {
            let a = ring * stride + segment;
            let b = a + stride;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }

    let mut mesh = Mesh { vertices, indices };
    mesh.compute_tangents();
    mesh
}
