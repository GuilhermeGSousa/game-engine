use std::f32::consts::PI;

use animation::plugin::AnimationPlugin;
use essential::{
    assets::asset_server::AssetServer,
    time::Time,
    transform::{GlobalTranform, Transform},
};

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
use glam::{Quat, Vec3, Vec4};
use gltf::plugin::GLTFPlugin;
use obj::{
    obj_loader::{OBJAsset, OBJSpawnerComponent},
    plugin::OBJPlugin,
};
use physics::{physics_state::PhysicsState, plugin::PhysicsPlugin, rigid_body::RigidBody};
use render::{
    assets::texture::{Texture, TextureUsageSettings},
    components::{
        camera::{Camera, CameraTextureTarget, RenderTarget},
        light::{LighType, Light, SpotLight},
        material_component::MaterialComponent,
        mesh_component::MeshComponent,
        skybox::Skybox,
    },
    material_plugin::MaterialPlugin,
    plugin::RenderPlugin,
};

use taffy::FlexDirection;
use ui::{
    material::UIMaterial, node::UINode, plugin::UIPlugin, text::TextComponent, transform::UIValue,
};
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

#[allow(dead_code)]

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
    app.register_plugin(TimePlugin)
        .register_plugin(AssetManagerPlugin)
        .register_plugin(WindowPlugin)
        .register_plugin(RenderPlugin)
        .register_plugin(TransformPlugin)
        .register_plugin(PhysicsPlugin)
        .register_plugin(AnimationPlugin)
        .register_plugin(GLTFPlugin)
        .register_plugin(OBJPlugin)
        // Register the custom unlit material so the engine knows how to render it.
        .register_plugin(MaterialPlugin::<UnlitMaterial>::new())
        .register_plugin(UIPlugin)
        .add_system(app::update_group::UpdateGroup::Update, move_around)
        .add_system(
            app::update_group::UpdateGroup::Update,
            spawn_on_button_press,
        )
        .add_system(
            app::update_group::UpdateGroup::Update,
            despawn_on_button_press,
        )
        .add_system(app::update_group::UpdateGroup::Update, setup_state_machine)
        .add_system(app::update_group::UpdateGroup::Update, setup_animations)
        .add_system(app::update_group::UpdateGroup::Update, update_movement_fsm)
        .add_system(app::update_group::UpdateGroup::Update, spawn_with_collider)
        // Async OBJ spawner for the unlit-material ground plane.
        .add_system(app::update_group::UpdateGroup::Update, spawn_unlit_obj)
        .add_system(app::update_group::UpdateGroup::Startup, spawn_ui)
        .add_system(app::update_group::UpdateGroup::Startup, spawn_floor)
        .add_system(app::update_group::UpdateGroup::Startup, spawn_player)
        .add_system(
            app::update_group::UpdateGroup::Startup,
            spawn_viewport_camera,
        );

    app.run();
}

fn spawn_ui(mut cmd: CommandQueue) {
    let root_pannel = cmd.spawn((UINode {
        width: UIValue::Percent(1.0),
        height: UIValue::Percent(1.0),
        flex_direction: FlexDirection::Column,
        ..Default::default()
    },));

    let spacer_pannel = cmd.spawn((
        UINode {
            flex_grow: 1.0,
            ..Default::default()
        },
        TextComponent {
            text: "Hello I am text".to_string(),
            font_size: 20.0,
            line_height: 30.0,
        },
    ));

    let bottom_pannel = cmd.spawn((
        UINode {
            width: UIValue::Percent(1.0),
            height: UIValue::Percent(0.1),
            ..Default::default()
        },
        UIMaterial {
            color: [0.0, 1.0, 0.0, 1.0],
        },
    ));

    cmd.add_child(root_pannel, spacer_pannel);
    cmd.add_child(root_pannel, bottom_pannel);
}

fn spawn_player(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    let camera = Camera::default();
    let skybox = Skybox {
        texture: asset_server.load_with_usage_settings(
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
        ),
    };

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

    let parent = cmd.spawn((
        Player,
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ));

    let child = cmd.spawn((
        camera,
        skybox,
        Transform::from_translation_rotation(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY),
    ));

    cmd.add_child(parent, child);

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
    let ground_colider = physics_state.make_cuboid(100.0, height, 100.0, &ground_transform, None);

    // Use the ground OBJ geometry with a custom unlit material rendered by
    // the user-defined `unlit.wgsl` shader (a bright cyan tint).
    let ground_mesh = asset_server.load::<OBJAsset>(GROUND_ASSET);

    // Create the UnlitMaterial asset: bright cyan so it is clearly
    // distinguishable from the Phong-shaded spheres.
    let unlit_mat = asset_server.add(UnlitMaterial {
        tint: TintUniform::new(0.2, 0.8, 1.0, 1.0),
    });

    cmd.spawn((
        UnlitOBJSpawner {
            mesh: ground_mesh,
            material: unlit_mat,
        },
        ground_colider,
        ground_transform,
    ));
}

/// Temporary component that holds the OBJ handle and custom material handle
/// before the OBJ asset is fully loaded.  Once the OBJ finishes loading the
/// [`spawn_unlit_obj`] system expands it into child [`MeshComponent`] entities
/// and removes this component from the entity.
#[derive(ecs::component::Component)]
struct UnlitOBJSpawner {
    mesh: essential::assets::handle::AssetHandle<OBJAsset>,
    material: essential::assets::handle::AssetHandle<UnlitMaterial>,
}

/// System that waits for the OBJ to load and then spawns mesh children using
/// the custom `UnlitMaterial` instead of the MTL-based `StandardMaterial`.
fn spawn_unlit_obj(
    mut cmd: CommandQueue,
    spawners: ecs::query::Query<(Entity, &UnlitOBJSpawner)>,
    obj_assets: Res<essential::assets::asset_store::AssetStore<OBJAsset>>,
) {
    for (entity, spawner) in spawners.iter() {
        if let Some(asset) = obj_assets.get(&spawner.mesh) {
            for obj_mesh in asset.meshes() {
                let child = cmd.spawn((
                    MeshComponent {
                        handle: obj_mesh.handle.clone(),
                    },
                    Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
                    MaterialComponent::<UnlitMaterial> {
                        handle: spawner.material.clone(),
                    },
                ));
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

    if key_d == InputState::Pressed || key_d == InputState::Down {
        let right = player_transform.right();
        player_transform.translation += right * displacement;
    }

    if key_a == InputState::Pressed || key_a == InputState::Down {
        let left = player_transform.left();
        player_transform.translation += left * displacement;
    }

    if key_w == InputState::Pressed || key_w == InputState::Down {
        let forward = player_transform.forward();
        player_transform.translation += forward * displacement;
    }

    if key_s == InputState::Pressed || key_s == InputState::Down {
        let back = player_transform.backward();
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
        let mut rigid_body = RigidBody::new(&cube_transform, &mut physics_state);

        let collider = physics_state.make_sphere(&mut rigid_body, 1.0);

        cmd.spawn((
            OBJSpawnerComponent(asset_server.load::<OBJAsset>(MESH_ASSET)),
            rigid_body,
            collider,
            cube_transform.clone(),
        ));
    }
}

fn despawn_on_button_press(
    meshes: Query<(Entity, &MeshComponent, &mut Transform)>,
    mut cmd: CommandQueue,
    input: Res<Input>,
) {
    let key_d = input.get_key_state(PhysicalKey::Code(KeyCode::KeyL));

    if key_d == InputState::Pressed {
        for (entity, _, _) in meshes.iter() {
            cmd.despawn(entity);
        }
    }
}

/// Spawn a secondary camera that renders the scene into a 512×512 off-screen
/// texture instead of the main window.
///
/// The texture handle stored on [`CameraTextureTarget`] can be retrieved by
/// querying [`CameraTextureTarget`] and used in a UI viewport material to
/// display what this camera sees.
fn spawn_viewport_camera(mut cmd: CommandQueue, asset_server: Res<AssetServer>) {
    // Create a render-target texture asset.  It has no initial pixel data;
    // the engine will render into it each frame.  The handle can later be
    // passed to a material/UI system to sample from the rendered output.
    let viewport_texture = asset_server.add(Texture::new_render_target(512, 512));

    cmd.spawn((
        Camera {
            render_target: RenderTarget::texture(viewport_texture),
            ..Camera::default()
        },
        Transform::from_translation_rotation(Vec3::new(5.0, 3.0, 5.0), Quat::IDENTITY),
    ));
}

/// Example showing how to consume the texture rendered by a texture-target camera.
///
/// Query [`CameraTextureTarget`] to obtain the [`AssetHandle<Texture>`] for the
/// camera's off-screen render target.  The same handle can be passed to a
/// material or UI element to sample from the rendered output.
///
/// Register this system after the render group (e.g. `UpdateGroup::LateRender`)
/// so it sees the fully rendered texture.
///
/// ```ignore
/// app.add_system(UpdateGroup::LateRender, use_viewport_texture);
/// ```
#[allow(dead_code)]
fn use_viewport_texture(texture_targets: Query<&CameraTextureTarget>) {
    for target in texture_targets.iter() {
        let _handle = &target.texture_handle;
        // Pass `_handle` to a material's texture slot or a UI viewport panel to
        // display the rendered output.
    }
}
