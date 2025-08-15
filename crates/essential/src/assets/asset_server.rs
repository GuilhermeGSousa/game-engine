use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock, Weak},
};

use crossbeam_channel::{Receiver, Sender};
use ecs::{resource::Resource, world};

use crate::{
    assets::{handle::StrongAssetHandle, AssetPath},
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
    LoadFailed(AssetId),
}

pub struct AssetLoadContext {
    asset_server: AssetServer,
}

impl AssetLoadContext {
    pub fn asset_server(&self) -> &AssetServer {
        &self.asset_server
    }
}

impl AssetLoadContext {
    pub(crate) fn new(asset_server: AssetServer) -> Self {
        Self { asset_server }
    }
}

pub(crate) struct AssetInfo {
    handle: Weak<StrongAssetHandle>,
}

pub(crate) struct AssetServerData {
    pending_tasks: RwLock<HashMap<AssetId, Task<()>>>,
    loaded_assets: RwLock<HashSet<AssetId>>,
    asset_handles: RwLock<HashMap<AssetId, AssetInfo>>,
    asset_lifetime_send_map: RwLock<HashMap<TypeId, Sender<AssetLifetimeEvent>>>,
    asset_load_event_sender: Sender<AssetLoadEvent>,
    asset_load_event_receiver: Receiver<AssetLoadEvent>,
}

#[derive(Resource, Clone)]
pub struct AssetServer {
    data: Arc<AssetServerData>,
}

impl AssetServer {
    pub fn new() -> Self {
        let (asset_load_event_sender, asset_load_event_receiver) = crossbeam_channel::unbounded();
        let server_data = AssetServerData {
            pending_tasks: RwLock::new(HashMap::new()),
            loaded_assets: RwLock::new(HashSet::new()),
            asset_handles: RwLock::new(HashMap::new()),
            asset_lifetime_send_map: RwLock::new(HashMap::new()),
            asset_load_event_sender,
            asset_load_event_receiver,
        };

        Self {
            data: Arc::new(server_data),
        }
    }

    pub fn register_asset<A: Asset>(&mut self, asset: &AssetStore<A>) {
        let type_id = TypeId::of::<A>();
        self.data
            .asset_lifetime_send_map
            .write()
            .unwrap()
            .insert(type_id, asset.clone_drop_sender());
    }

    pub fn load<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> AssetHandle<A>
    where
        A: 'static,
    {
        self.load_internal::<A>(path, A::default_usage_settings())
    }

    pub fn load_with_usage_settings<'a, A: Asset>(
        &self,
        path: impl Into<AssetPath<'a>>,
        usage_settings: A::UsageSettings,
    ) -> AssetHandle<A>
    where
        A: 'static,
    {
        self.load_internal::<A>(path, usage_settings)
    }

    pub fn load_internal<'a, A: Asset>(
        &self,
        path: impl Into<AssetPath<'a>>,
        usage_settings: A::UsageSettings,
    ) -> AssetHandle<A> {
        let path = path.into().into_owned();
        let id = AssetId::new::<A>(&path);
        let lifetime_sender = self
            .data
            .asset_lifetime_send_map
            .read()
            .unwrap()
            .get(&TypeId::of::<A>())
            .expect("Asset lifetime sender not found, make sure to register it")
            .clone();

        let mut binding = self.data.asset_handles.write().unwrap();
        let info = binding.entry(id.clone()).or_insert_with(|| AssetInfo {
            handle: Weak::new(),
        });

        let handle = if let Some(strong_handle) = info.handle.upgrade() {
            AssetHandle::new(strong_handle)
        } else {
            let handle = Arc::new(StrongAssetHandle {
                id,
                lifetime_sender,
            });

            info.handle = Arc::downgrade(&handle);

            AssetHandle::new(handle)
        };

        if !self.data.pending_tasks.read().unwrap().contains_key(&id)
            && !self.data.loaded_assets.read().unwrap().contains(&id)
        {
            self.request_load::<A>(path.clone(), usage_settings);
        }

        handle
    }

    pub fn process_handle_drop(&mut self, id: &AssetId) {
        self.data.loaded_assets.write().unwrap().remove(id);
    }

    fn request_load<A: Asset>(&self, path: AssetPath<'static>, usage_settings: A::UsageSettings) {
        let id = AssetId::new::<A>(&path);
        let asset_loader = A::loader();

        let sender = self.data.asset_load_event_sender.clone();

        let server = self.clone();
        let task = LoadTaskPool::get_or_init(TaskPool::new).spawn(async move {
            let asset = asset_loader
                .load(path, &mut AssetLoadContext::new(server), usage_settings)
                .await;
            match asset {
                Ok(asset) => {
                    sender
                        .send(AssetLoadEvent::Loaded(LoadedAsset::new(id, asset)))
                        .unwrap();
                    ()
                }
                Err(_) => {
                    sender.send(AssetLoadEvent::LoadFailed(id)).unwrap();
                    ()
                }
            }
        });

        self.data.pending_tasks.write().unwrap().insert(id, task);
    }
}

pub fn handle_asset_load_events(world: &mut world::World) {
    let server = world.remove_resource::<AssetServer>().unwrap();

    server
        .data
        .asset_load_event_receiver
        .try_iter()
        .for_each(|event| match event {
            AssetLoadEvent::Loaded(loaded_asset) => {
                server
                    .data
                    .pending_tasks
                    .write()
                    .unwrap()
                    .remove(&loaded_asset.id);
                server
                    .data
                    .loaded_assets
                    .write()
                    .unwrap()
                    .insert(loaded_asset.id);
                loaded_asset.value.insert(loaded_asset.id, world);
            }
            AssetLoadEvent::LoadFailed(id) => {
                server.data.pending_tasks.write().unwrap().remove(&id);
                server.data.loaded_assets.write().unwrap().remove(&id);
            }
        });
    world.insert_resource(server);
}
