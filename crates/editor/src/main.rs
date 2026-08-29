use std::f32::consts::PI;

use app::{
    App,
    plugins::{AssetManagerPlugin, TimePlugin, TransformPlugin},
    schedule_groups::{Startup, Update},
};
use color::Color;
use ecs::{
    command::CommandQueue,
    component::Component,
    query::{Query, filter::With},
    resource::Res,
};
use essential::{time::Time, transform::Transform};
use glam::{Quat, Vec3};
use render::{
    components::{
        camera::{Camera, RenderTarget},
        light::{Light, LightType},
    },
    plugin::RenderPlugin,
};
use ui::plugin::UIPlugin;
use window::{
    input::{Input, InputState},
    plugin::WindowPlugin,
};
use winit::keyboard::{KeyCode, PhysicalKey};

mod layout;
mod plugin;

use layout::EditorRttHandle;
use plugin::EditorPlugin;

fn main() {
    env_logger::init();

    let mut app = App::new();
    app.register_plugin(AssetManagerPlugin)
        .register_plugin(TimePlugin)
        .register_plugin(WindowPlugin)
        .register_plugin(RenderPlugin)
        .register_plugin(TransformPlugin)
        .register_plugin(UIPlugin)
        .register_plugin(EditorPlugin)
        .add_system(Startup, spawn_scene)
        .add_system(Update, navigate_camera);

    app.run();
}

// ── Scene ─────────────────────────────────────────────────────────────────────

/// Marker so the camera-navigation system can find the editor camera entity.
#[derive(Component)]
struct EditorCamera;

fn spawn_scene(mut cmd: CommandQueue, rtt: Res<EditorRttHandle>) {
    // Editor camera — renders into the RTT texture shown by UIViewport.
    cmd.spawn((
        EditorCamera,
        Camera {
            render_target: RenderTarget::Texture(rtt.0.clone()),
            ..Default::default()
        },
        Transform::from_translation_rotation(
            Vec3::new(0.0, 2.0, -6.0),
            Quat::from_euler(glam::EulerRot::XYZ, 0.0, PI, 0.0),
        ),
    ));

    // Simple directional-ish spot light.
    let mut light_transform =
        Transform::from_translation_rotation(Vec3::new(4.0, 8.0, 4.0), Quat::IDENTITY);
    light_transform.rotation = Quat::from_euler(glam::EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0);

    cmd.spawn((
        Light {
            color: Color::rgba(1.0, 0.95, 0.85, 1.0),
            intensity: 15.0,
            light_type: LightType::Spot {
                cone_angle: 60.0_f32.to_radians(),
            },
            shadowmaps_enabled: false,
        },
        light_transform,
    ));
}

// ── Camera navigation ─────────────────────────────────────────────────────────

fn navigate_camera(
    cameras: Query<&mut Transform, With<EditorCamera>>,
    input: Res<Input>,
    time: Res<Time>,
) {
    let mut transform = match cameras.iter().next() {
        Some(t) => t,
        None => return,
    };

    let speed = 5.0 * time.delta().as_secs_f32();

    let forward = transform.forward();
    let right = transform.right();

    if input.get_key_state(PhysicalKey::Code(KeyCode::KeyW)) != InputState::Released {
        transform.translation += forward * speed;
    }
    if input.get_key_state(PhysicalKey::Code(KeyCode::KeyS)) != InputState::Released {
        transform.translation -= forward * speed;
    }
    if input.get_key_state(PhysicalKey::Code(KeyCode::KeyD)) != InputState::Released {
        transform.translation += right * speed;
    }
    if input.get_key_state(PhysicalKey::Code(KeyCode::KeyA)) != InputState::Released {
        transform.translation -= right * speed;
    }
    if input.get_key_state(PhysicalKey::Code(KeyCode::KeyE)) != InputState::Released {
        transform.translation += Vec3::Y * speed;
    }
    if input.get_key_state(PhysicalKey::Code(KeyCode::KeyQ)) != InputState::Released {
        transform.translation -= Vec3::Y * speed;
    }

    let sensitivity = -0.003;
    let delta = input.mouse_delta();
    if delta.x != 0.0 {
        transform.rotation *= Quat::from_axis_angle(Vec3::Y, sensitivity * delta.x);
    }
    if delta.y != 0.0 {
        let local_right = transform.right();
        transform.rotation *= Quat::from_axis_angle(local_right, sensitivity * delta.y);
    }
}
