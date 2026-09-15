//! Loading scene assets and spawning them into the world.
//!
//! Read-only: a scene is opened, spawned, and thereafter the world is the only
//! record of it. Nothing here retains a parsed copy of the file for panels to
//! read — they inspect the entities instead.
use app::{schedule_groups::Update, App, Plugin};
use ecs::{command::CommandQueue, Component, Entity, Query, ResMut, Resource};
use essential::assets::{content::read_content_asset, AssetId};
use essential::transform::Transform;
use scene::scene::Scene;
use scene::spawner::spawn_scene;
use std::{collections::VecDeque, path::PathBuf, thread::JoinHandle};

use crate::selection::Selection;

pub enum SceneCommand {
    Open {
        root: PathBuf,
        address: String,
        id: AssetId,
    },
    /// Despawns every open scene, e.g. when the project changes.
    Clear,
}

#[derive(Resource, Default)]
pub struct SceneCommands(pub VecDeque<SceneCommand>);

/// Marks the entity a spawned scene hangs from.
///
/// The hierarchy panel roots its tree here rather than at every parentless
/// entity, because the editor's own UI, camera, light and grid share this
/// world and would otherwise flood the tree.
#[derive(Component)]
pub struct SceneRoot {
    pub asset_id: AssetId,
    pub address: String,
}

#[derive(Resource, Default)]
pub struct SceneState {
    pub status: String,
    /// Bumped whenever scenes are spawned or cleared.
    pub revision: u64,
    job: Option<JoinHandle<Result<LoadedScene, String>>>,
}

impl SceneState {
    pub fn loading(&self) -> bool {
        self.job.is_some()
    }
}

struct LoadedScene {
    id: AssetId,
    address: String,
    scene: Scene,
}

/// Reads and decodes a `.gasset` scene. Runs on a worker thread.
fn load_scene(root: PathBuf, address: String, id: AssetId) -> anyhow::Result<LoadedScene> {
    let bytes = std::fs::read(root.join(&address))?;
    let (header, payload) = read_content_asset(&bytes)?;
    anyhow::ensure!(
        header.asset_id == id,
        "Asset identity changed; refresh the project catalogue"
    );
    anyhow::ensure!(header.kind == "Scene", "Selected asset is not a Scene");
    Ok(LoadedScene {
        id,
        address,
        scene: bincode::deserialize(payload)?,
    })
}

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SceneState::default());
        app.insert_resource(SceneCommands::default());
        app.add_system(Update, update_scenes);
    }
}

fn update_scenes(
    mut commands: ResMut<SceneCommands>,
    mut state: ResMut<SceneState>,
    mut selection: ResMut<Selection>,
    roots: Query<(Entity, &SceneRoot)>,
    mut cmd: CommandQueue,
) {
    // Consume requests first: replacing a job discards its result even if it
    // just completed.
    while let Some(command) = commands.0.pop_front() {
        state.job = None;
        match command {
            SceneCommand::Clear => {
                for (entity, _) in roots.iter() {
                    cmd.despawn_recursive(entity);
                }
                selection.clear();
                state.revision += 1;
                state.status.clear();
            }
            SceneCommand::Open { root, address, id } => {
                state.status = format!("Opening {address}…");
                state.job = Some(std::thread::spawn(move || {
                    load_scene(root, address, id).map_err(|error| format!("{error:#}"))
                }));
            }
        }
    }

    if !state.job.as_ref().is_some_and(|job| job.is_finished()) {
        return;
    }
    match state
        .job
        .take()
        .expect("job was just observed as finished")
        .join()
        .unwrap_or_else(|_| Err("Scene worker failed".into()))
    {
        Ok(loaded) => {
            state.status = format!("{} · {} nodes", loaded.address, loaded.scene.nodes.len());
            let root_entity = cmd
                .spawn((
                    Transform::IDENTITY,
                    SceneRoot {
                        asset_id: loaded.id,
                        address: loaded.address.clone(),
                    },
                ))
                .entity();
            // The scene's own cameras stay unspawned: the editor supplies the
            // viewport camera, and a second one would fight it for the target.
            spawn_scene(&mut cmd, &loaded.scene, root_entity);
            selection.select_entity(root_entity);
            state.revision += 1;
        }
        Err(error) => state.status = format!("Could not open scene: {error}"),
    }
}
