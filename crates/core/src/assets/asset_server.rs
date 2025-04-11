use std::{collections::HashMap, path};

use crossbeam_channel::{Receiver, Sender};
use ecs::{resource::Resource, world};

use crate::tasks::{task_pool::TaskPool, Task};

use super::{handle::AssetHandle, Asset, AssetId};

enum AssetLoadEvent {
    Loaded(AssetId),
    Failed(AssetId),
}

#[derive(Resource)]
pub struct AssetServer {
    pending_tasks: HashMap<AssetId, Task<()>>,
    asset_load_event_sender: Sender<AssetLoadEvent>,
    pub(crate) asset_load_event_receiver: Receiver<AssetLoadEvent>,
}

impl AssetServer {
    pub fn new() -> Self {
        let (asset_load_event_sender, asset_load_event_receiver) = crossbeam_channel::unbounded();
        AssetServer {
            pending_tasks: HashMap::new(),
            asset_load_event_sender,
            asset_load_event_receiver,
        }
    }

    pub fn load<A: Asset>(&mut self, path: &str) -> Result<AssetHandle<A>, String>
    where
        A: 'static,
    {
        let asset_loader = A::loader();
        let id = A::id(path.to_string());

        let sender = self.asset_load_event_sender.clone();
        let task = TaskPool::spawn(async move {
            let asset = asset_loader.load(id.clone()).await;
            match asset {
                Ok(_) => {
                    // TODO
                    sender.send(AssetLoadEvent::Loaded(id)).unwrap();
                    ()
                }
                Err(_) => {
                    // TODO
                    sender.send(AssetLoadEvent::Failed(id)).unwrap();
                    ()
                }
            }
        });

        self.pending_tasks.insert(id, task);

        Err("Not implemented".to_string())
    }
}

pub fn handle_asset_load_events(world: &mut world::World) {
    let server = world.get_resource_mut::<AssetServer>().unwrap();

    server
        .asset_load_event_receiver
        .try_iter()
        .for_each(|event| {
            match event {
                AssetLoadEvent::Loaded(id) => {
                    // Handle loaded asset
                    println!("Asset loaded: {:?}", id);
                }
                AssetLoadEvent::Failed(id) => {
                    // Handle failed asset load
                    println!("Asset load failed: {:?}", id);
                }
            }
        });
}
