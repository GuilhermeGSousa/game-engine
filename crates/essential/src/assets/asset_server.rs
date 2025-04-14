use std::{any::TypeId, collections::HashMap};

use crossbeam_channel::{Receiver, Sender};
use ecs::{resource::Resource, world};

use crate::{
    assets::AssetPath,
    tasks::{task_pool::TaskPool, Task},
};

use super::{
    asset_container::AssetContainer,
    asset_store::AssetStore,
    handle::{AssetHandle, AssetLifetimeEvent},
    load_pool::LoadTaskPool,
    Asset, AssetId,
};

struct LoadedAsset {
    pub(crate) id: AssetId,
    pub(crate) value: Box<dyn AssetContainer>,
}

impl LoadedAsset {
    pub fn new<A: Asset>(id: AssetId, value: A) -> Self
    where
        A: 'static,
    {
        LoadedAsset {
            id,
            value: Box::new(value),
        }
    }
}

enum AssetLoadEvent {
    Loaded(LoadedAsset),
    Failed(AssetId),
}

#[derive(Resource)]
pub struct AssetServer {
    pending_tasks: HashMap<AssetId, Task<()>>,
    asset_load_event_sender: Sender<AssetLoadEvent>,
    asset_load_event_receiver: Receiver<AssetLoadEvent>,

    // Data
    asset_lifetime_send_map: HashMap<TypeId, Sender<AssetLifetimeEvent>>,
}

impl AssetServer {
    pub fn new() -> Self {
        let (asset_load_event_sender, asset_load_event_receiver) = crossbeam_channel::unbounded();
        AssetServer {
            pending_tasks: HashMap::new(),
            asset_load_event_sender,
            asset_load_event_receiver,
            asset_lifetime_send_map: HashMap::new(),
        }
    }

    pub fn register_asset<A: Asset>(&mut self, asset: &AssetStore<A>) {
        let type_id = TypeId::of::<A>();
        self.asset_lifetime_send_map
            .insert(type_id, asset.clone_drop_sender());
    }

    pub fn load<A: Asset>(&mut self, path: impl Into<AssetPath>) -> AssetHandle<A>
    where
        A: 'static,
    {
        let path = path.into();
        let id = AssetId::new::<A>(&path);
        let lifetime_sender = self
            .asset_lifetime_send_map
            .get(&TypeId::of::<A>())
            .expect("Asset lifetime sender not found, make sure to register it")
            .clone();

        let handle = AssetHandle::new(id, lifetime_sender);

        if !self.pending_tasks.contains_key(&id) {
            self.load_asset_internal::<A>(path, id);
        }

        handle
    }

    fn load_asset_internal<A: Asset>(&mut self, path: AssetPath, id: AssetId) {
        let asset_loader = A::loader();

        let sender = self.asset_load_event_sender.clone();

        let task = LoadTaskPool::get_or_init(TaskPool::new).spawn(async move {
            let asset = asset_loader.load(path).await;
            match asset {
                Ok(asset) => {
                    sender
                        .send(AssetLoadEvent::Loaded(LoadedAsset::new(id, asset)))
                        .unwrap();
                    ()
                }
                Err(_) => {
                    sender.send(AssetLoadEvent::Failed(id)).unwrap();
                    ()
                }
            }
        });

        self.pending_tasks.insert(id, task);
    }
}

pub fn handle_asset_load_events(world: &mut world::World) {
    let mut server = world.remove_resource::<AssetServer>().unwrap();

    server
        .asset_load_event_receiver
        .try_iter()
        .for_each(|event| match event {
            AssetLoadEvent::Loaded(loaded_asset) => {
                server.pending_tasks.remove(&loaded_asset.id).unwrap();
                loaded_asset.value.insert(loaded_asset.id, world);
            }
            AssetLoadEvent::Failed(id) => {
                server.pending_tasks.remove(&id).unwrap();
            }
        });
    world.insert_resource(server);
}
