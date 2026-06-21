use std::f32::consts::PI;

use color::LinearRgba;
use game_engine::{
    app::App,
    ecs::{
        command::CommandQueue,
        component::Component,
        entity::Entity,
        query::Query,
        resource::{Res, ResMut},
        system::schedule::{Schedules, UpdateGroup},
    },
    essential::{
        assets::asset_server::AssetServer,
        transform::{GlobalTransform, Transform},
    },
    mesh::MeshComponent,
    obj_loader::obj_loader::{OBJAsset, OBJSpawnerComponent},
    physics::{physics_state::PhysicsState, rigid_body::RigidBody},
    render::{
        assets::{material::StandardMaterial, texture::TextureUsageSettings},
        components::{
            camera::Camera,
            light::{Light, LightType, SpotLight},
            material::MaterialComponent,
        },
        material_plugin::MaterialPlugin,
    },
    skybox::{material::SkyboxMaterial, SkyboxCube},
    window::input::{Input, InputState},
    DefaultPlugins,
};
use gameplay::{movement::first_person_player_fly, player::spawn_first_person_player};
use glam::{Quat, Vec3};
use wgpu_types::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::custom_material::UnlitMaterial;
use crate::movement_animation::{
    setup_animations, setup_state_machine, spawn_on_button_press, update_movement_fsm,
};

mod custom_material;
mod movement_animation;

const MESH_ASSET: &str = "res/sphere.obj";
const GROUND_ASSET: &str = "res/ground.obj";
const SKYBOX_TEXTURE: &str = "res/Ryfjallet-cubemap.png";

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    std::env::set_current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("Failed to set working directory");

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let mut app = App::new();
    app.register_plugin(DefaultPlugins::default())
        .register_plugin(MaterialPlugin::<UnlitMaterial>::new())
        .add_system(UpdateGroup::Update, first_person_player_fly)
        .add_system(UpdateGroup::Update, spawn_on_button_press)
        .add_system(UpdateGroup::Update, setup_state_machine)
        .add_system(UpdateGroup::Update, setup_animations)
        .add_system(UpdateGroup::Update, update_movement_fsm)
        .add_system(UpdateGroup::Update, spawn_with_collider)
        .add_system(UpdateGroup::Update, spawn_unlit_obj)
        .add_system(UpdateGroup::Startup, spawn_floor)
        .add_system(UpdateGroup::Startup, spawn_player);
    let schedules = app.get_resource::<Schedules>();
    println!("Schedules: {:?}", schedules.unwrap());
    app.run();
}

fn spawn_player(
    skybox_cube: Res<SkyboxCube>,
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
) {
    let skybox_material = SkyboxMaterial {
        texture: Some(asset_server.load_with_usage_settings(
            SKYBOX_TEXTURE,
            TextureUsageSettings {
                texture_descriptor: TextureDescriptor {
                    label: Some("cubemap_texture"),
                    size: Extent3d {
                        width: 256,
                        height: 256,
                        depth_or_array_layers: 6,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                texture_view_descriptor: TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::Cube),
                    ..Default::default()
                },
            },
        )),
    };

    let skybox_cube = MeshComponent {
        handle: skybox_cube.clone(),
    };
    cmd.spawn((
        MaterialComponent {
            handle: asset_server.add(skybox_material),
        },
        skybox_cube,
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ));

    let light = Light {
        color: LinearRgba::new(1.0, 0.0, 1.0, 1.0),
        intensity: 10.0,
        light_type: LightType::Spot(SpotLight {
            cone_angle: 50.0_f32.to_radians(),
        }),
    };
    let mut light_transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
    light_transform.rotation = Quat::from_euler(glam::EulerRot::XYZ, PI / 2.0, 0.0, 0.0);

    spawn_first_person_player(&mut cmd, Vec3::new(0.0, 2.0, 0.0), ());

    cmd.spawn((light, light_transform));
}

fn spawn_floor(
    mut cmd: CommandQueue,
    mut physics_state: ResMut<PhysicsState>,
    asset_server: Res<AssetServer>,
) {
    let height = 1.0;
    let ground_transform =
        Transform::from_translation_rotation(Vec3::Y * (-2.0 * height), Quat::IDENTITY);
    let ground_collider = physics_state.make_cuboid(100.0, height, 100.0, &ground_transform, None);
    let ground_mesh = asset_server.load::<OBJAsset>(GROUND_ASSET);
    let ground_mat = asset_server.add(StandardMaterial::new(None, None));
    cmd.spawn((
        OBJSpawner {
            mesh: ground_mesh,
            material: ground_mat,
        },
        ground_collider,
        ground_transform,
    ));
}

#[derive(game_engine::ecs::component::Component)]
struct OBJSpawner {
    mesh: game_engine::essential::assets::handle::AssetHandle<OBJAsset>,
    material: game_engine::essential::assets::handle::AssetHandle<StandardMaterial>,
}

fn spawn_unlit_obj(
    mut cmd: CommandQueue,
    spawners: game_engine::ecs::query::Query<(Entity, &OBJSpawner)>,
    obj_assets: Res<game_engine::essential::assets::asset_store::AssetStore<OBJAsset>>,
) {
    for (entity, spawner) in spawners.iter() {
        if let Some(asset) = obj_assets.get(&spawner.mesh) {
            for obj_mesh in asset.meshes() {
                let child = *cmd
                    .spawn((
                        MeshComponent {
                            handle: obj_mesh.handle.clone(),
                        },
                        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
                        MaterialComponent::<StandardMaterial> {
                            handle: spawner.material.clone(),
                        },
                    ))
                    .entity();
                cmd.add_child(entity, child);
            }
            cmd.remove::<OBJSpawner>(entity);
        }
    }
}

fn spawn_with_collider(
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut cmd: CommandQueue,
    input: Res<Input>,
    asset_server: Res<AssetServer>,
    mut physics_state: ResMut<PhysicsState>,
) {
    let Some((_, pos)) = cameras.iter().next() else {
        return;
    };
    let key_r = input.get_key_state(PhysicalKey::Code(KeyCode::KeyR));

    if key_r == InputState::Pressed {
        let spawn_point = pos.translation() + pos.forward() * 10.0;
        let cube_transform = Transform::from_translation_rotation(spawn_point, Quat::IDENTITY);
        let rigid_body = RigidBody::new(&cube_transform, &mut physics_state);
        let collider = physics_state.make_sphere(&rigid_body, 1.0);
        cmd.spawn((
            OBJSpawnerComponent(asset_server.load::<OBJAsset>(MESH_ASSET)),
            rigid_body,
            collider,
            cube_transform.clone(),
        ));
    }
}
