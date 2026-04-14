use std::f32::consts::PI;

use animation::plugin::AnimationPlugin;
use app::{
    plugins::{AssetManagerPlugin, TimePlugin, TransformPlugin},
    App,
};
use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{query_filter::With, Query},
    resource::{Res, ResMut},
};
use essential::{
    assets::asset_server::AssetServer,
    time::Time,
    transform::{GlobalTranform, Transform},
};
use glam::{Quat, Vec3, Vec4};
use gltf_loader::plugin::GLTFPlugin;
use obj_loader::{
    obj_loader::{OBJAsset, OBJSpawnerComponent},
    plugin::OBJPlugin,
};
use physics::{physics_state::PhysicsState, plugin::PhysicsPlugin, rigid_body::RigidBody};
use render::{
    assets::texture::TextureUsageSettings,
    components::{
        camera::Camera,
        light::{LighType, Light, SpotLight},
        material_component::MaterialComponent,
        mesh_component::MeshComponent,
    },
    material_plugin::MaterialPlugin,
    plugin::RenderPlugin,
};
use skybox::{material::SkyboxMaterial, plugin::SkyboxPlugin, SkyboxCube};
use ui::plugin::UIPlugin;
use wgpu_types::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};
use window::{
    input::{Input, InputState},
    plugin::WindowPlugin,
};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::custom_material::{TintUniform, UnlitMaterial};
use crate::movement_animation::{
    setup_animations, setup_state_machine, spawn_on_button_press, update_movement_fsm,
};

mod custom_material;
mod movement_animation;

const MESH_ASSET: &str = "res/sphere.obj";
const GROUND_ASSET: &str = "res/ground.obj";
const SKYBOX_TEXTURE: &str = "res/Ryfjallet-cubemap.png";

#[derive(Component)]
struct Player;

pub fn run_game() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let mut app = App::empty();
    app.register_plugin(AssetManagerPlugin)
        .register_plugin(TimePlugin)
        .register_plugin(WindowPlugin)
        .register_plugin(RenderPlugin)
        .register_plugin(TransformPlugin)
        .register_plugin(PhysicsPlugin)
        .register_plugin(AnimationPlugin)
        .register_plugin(GLTFPlugin)
        .register_plugin(OBJPlugin)
        .register_plugin(SkyboxPlugin)
        .register_plugin(MaterialPlugin::<UnlitMaterial>::new())
        .register_plugin(UIPlugin)
        .add_system(app::update_group::UpdateGroup::Update, move_around)
        .add_system(
            app::update_group::UpdateGroup::Update,
            spawn_on_button_press,
        )
        .add_system(app::update_group::UpdateGroup::Update, setup_state_machine)
        .add_system(app::update_group::UpdateGroup::Update, setup_animations)
        .add_system(app::update_group::UpdateGroup::Update, update_movement_fsm)
        .add_system(app::update_group::UpdateGroup::Update, spawn_with_collider)
        .add_system(app::update_group::UpdateGroup::Update, spawn_unlit_obj)
        .add_system(app::update_group::UpdateGroup::Startup, spawn_floor)
        .add_system(app::update_group::UpdateGroup::Startup, spawn_player);

    app.run();
}

fn spawn_player(
    skybox_cube: Res<SkyboxCube>,
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
) {
    let camera = Camera::default();

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
        color: Vec4::new(1.0, 0.0, 1.0, 1.0),
        intensity: 10.0,
        light_type: LighType::Spot(SpotLight {
            cone_angle: 50.0 * PI / 180.0,
        }),
    };
    let mut light_transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY);
    light_transform.rotation = Quat::from_euler(glam::EulerRot::XYZ, PI / 2.0, 0.0, 0.0);

    cmd.spawn((
        Player,
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ))
    .add_child((
        camera,
        Transform::from_translation_rotation(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY),
    ));
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
    let unlit_mat = asset_server.add(UnlitMaterial {
        tint: TintUniform::new(0.2, 0.8, 1.0, 1.0),
    });
    cmd.spawn((
        UnlitOBJSpawner {
            mesh: ground_mesh,
            material: unlit_mat,
        },
        ground_collider,
        ground_transform,
    ));
}

#[derive(ecs::component::Component)]
struct UnlitOBJSpawner {
    mesh: essential::assets::handle::AssetHandle<OBJAsset>,
    material: essential::assets::handle::AssetHandle<UnlitMaterial>,
}

fn spawn_unlit_obj(
    mut cmd: CommandQueue,
    spawners: ecs::query::Query<(Entity, &UnlitOBJSpawner)>,
    obj_assets: Res<essential::assets::asset_store::AssetStore<OBJAsset>>,
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
                        MaterialComponent::<UnlitMaterial> {
                            handle: spawner.material.clone(),
                        },
                    ))
                    .entity();
                cmd.add_child(entity, child);
            }
            cmd.remove::<UnlitOBJSpawner>(entity);
        }
    }
}

fn move_around(
    players: Query<&mut Transform, With<Player>>,
    cameras: Query<&mut Transform, With<Camera>>,
    input: Res<Input>,
    time: Res<Time>,
) {
    let mut player_transform = players.iter().next().unwrap();
    let mut camera_transform = cameras.iter().next().unwrap();

    let displacement = 10.0 * time.delta().as_secs_f32();

    let key_d = input.get_key_state(PhysicalKey::Code(KeyCode::KeyD));
    let key_a = input.get_key_state(PhysicalKey::Code(KeyCode::KeyA));
    let key_w = input.get_key_state(PhysicalKey::Code(KeyCode::KeyW));
    let key_s = input.get_key_state(PhysicalKey::Code(KeyCode::KeyS));

    let right = player_transform.right();
    let left = player_transform.left();
    let forward = player_transform.forward();
    let back = player_transform.backward();

    if key_d == InputState::Pressed || key_d == InputState::Down {
        player_transform.translation += right * displacement;
    }
    if key_a == InputState::Pressed || key_a == InputState::Down {
        player_transform.translation += left * displacement;
    }
    if key_w == InputState::Pressed || key_w == InputState::Down {
        player_transform.translation += forward * displacement;
    }
    if key_s == InputState::Pressed || key_s == InputState::Down {
        player_transform.translation += back * displacement;
    }

    let sensitivity = -0.5;
    let mouse_delta = input.mouse_delta();
    let yaw_delta = sensitivity * mouse_delta.x * time.delta().as_secs_f32();
    let pitch_delta = sensitivity * mouse_delta.y * time.delta().as_secs_f32();
    player_transform.rotation *= Quat::from_axis_angle(Vec3::Y, yaw_delta);
    camera_transform.rotation *= Quat::from_axis_angle(Vec3::X, pitch_delta);
}

fn spawn_with_collider(
    cameras: Query<(&Camera, &GlobalTranform)>,
    mut cmd: CommandQueue,
    input: Res<Input>,
    asset_server: Res<AssetServer>,
    mut physics_state: ResMut<PhysicsState>,
) {
    let (_, pos) = cameras.iter().next().expect("No camera found");
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
