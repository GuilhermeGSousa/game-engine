use color::LinearRgba;
use game_engine::{
    app::App,
    ecs::{
        command::CommandQueue,
        resource::{Res, ResMut},
        system::schedule::UpdateGroup,
    },
    essential::{assets::asset_server::AssetServer, transform::Transform},
    render::components::light::{Light, LightType},
    DefaultPlugins,
};
use gameplay::{
    camera::{spawn_third_person_camera, update_spring_arm_camera},
    character_rig::spawn_character,
    level::{animate_moving_platforms, spawn_level},
    movement::{rotate_player_to_face_movement, third_person_movement_input},
    movement_state::{advance_movement_state, write_animation_params},
    player::spawn_third_person_player,
};
use glam::{Quat, Vec3};
use physics::physics_state::PhysicsState;
use world_grid::WorldGrid;

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
    app.register_plugin(DefaultPlugins::default());

    app.add_system(UpdateGroup::Startup, spawn_world);

    // Update: camera orbit (reads mouse), then camera-relative input -> character controller,
    // then facing/state-machine/animation bridging, all in that order so each system sees the
    // freshest data from the one before it (matches the sequential-registration-order
    // convention already used by the fly-camera example).
    app.add_system(UpdateGroup::Update, update_spring_arm_camera)
        .add_system(UpdateGroup::Update, third_person_movement_input)
        .add_system(UpdateGroup::Update, rotate_player_to_face_movement)
        .add_system(UpdateGroup::Update, advance_movement_state)
        .add_system(UpdateGroup::Update, write_animation_params);

    app.add_system(UpdateGroup::FixedUpdate, animate_moving_platforms);

    app.run();
}

fn spawn_world(
    mut cmd: CommandQueue,
    asset_server: Res<AssetServer>,
    mut physics_state: ResMut<PhysicsState>,
) {
    cmd.spawn(WorldGrid::default());

    spawn_level(&mut cmd, &asset_server, &mut physics_state);

    let player = spawn_third_person_player(&mut cmd, &mut physics_state, Vec3::new(0.0, 2.0, 0.0));
    spawn_character(&mut cmd, &asset_server, player);
    spawn_third_person_camera(&mut cmd, player, 6.0);

    cmd.spawn((
        Light {
            color: LinearRgba::new(1.0, 0.98, 0.92, 1.0),
            intensity: 3.0,
            light_type: LightType::Directional,
        },
        Transform::from_translation_rotation(
            Vec3::new(0.0, 10.0, 0.0),
            Quat::from_euler(glam::EulerRot::YXZ, 0.6, -0.9, 0.0),
        ),
    ));
}
