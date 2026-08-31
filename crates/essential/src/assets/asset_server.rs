use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock, Weak},
};

use crossbeam_channel::{Receiver, Sender};
use ecs::{resource::Resource, world};
use tasks::load_pool::LoadTaskPool;

use crate::{
    assets::{handle::StrongAssetHandle, AssetPath, LoadableAsset},
    tasks::{task_pool::TaskPool, Task},
};

use super::{
    asset_container::AssetContainer,
    asset_store::AssetStore,
    handle::{AssetHandle, AssetLifetimeEvent},
    Asset, AssetId,
};

struct LoadedAsset {
    pub(crate) id: AssetId,
    pub(crate) value: Box<dyn AssetContainer>,
}

impl LoadedAsset {
    pub fn new<A: Asset + 'static>(id: AssetId, value: A) -> Self {
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
    asset_id: AssetId,
}

impl AssetLoadContext {
    pub fn asset_server(&self) -> &AssetServer {
        &self.asset_server
    }

    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }
}

impl AssetLoadContext {
    pub(crate) fn new(asset_server: AssetServer, asset_id: AssetId) -> Self {
        Self {
            asset_server,
            asset_id,
        }
    }
}

pub(crate) struct AssetInfo {
    // std::sync::Weak, used only for handle-reuse/dedup in AssetHandleProvider
    // (doesn't keep the asset alive) — unrelated to the AssetHandle::Weak
    // enum variant (an unresolved AssetId reference).
    handle: Weak<StrongAssetHandle>,
}

pub(crate) struct AssetServerData {
    pending_tasks: RwLock<HashMap<AssetId, Task<()>>>,
    loaded_assets: RwLock<HashSet<AssetId>>,
    path_to_id: RwLock<HashMap<AssetPath<'static>, AssetId>>,
    handle_provider: AssetHandleProvider,
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
            path_to_id: RwLock::new(HashMap::new()),
            handle_provider: AssetHandleProvider::new(),
            asset_load_event_sender,
            asset_load_event_receiver,
        };

        Self {
            data: Arc::new(server_data),
        }
    }

    pub fn register_asset<A: Asset>(&mut self, asset: &AssetStore<A>) {
        self.data
            .handle_provider
            .register_asset::<A>(asset.clone_drop_sender());
    }

    pub fn load<'a, A>(&self, path: impl Into<AssetPath<'a>>) -> AssetHandle<A>
    where
        A: LoadableAsset + 'static,
    {
        self.load_internal::<A>(path, A::default_usage_settings())
    }

    pub fn add<A: Asset>(&self, asset: A) -> AssetHandle<A> {
        let id = AssetId::new();

        let sender = self.data.asset_load_event_sender.clone();
        let _ = sender.send(AssetLoadEvent::Loaded(LoadedAsset::new(id, asset)));
        self.data.handle_provider.request_handle(id, None)
    }

    pub fn load_with_usage_settings<'a, A>(
        &self,
        path: impl Into<AssetPath<'a>>,
        usage_settings: A::UsageSettings,
    ) -> AssetHandle<A>
    where
        A: LoadableAsset + 'static,
    {
        self.load_internal::<A>(path, usage_settings)
    }

    /// Loads (or returns a handle to an already-loading/loaded asset for) the
    /// given `AssetId` directly, with no `AssetPath` involved. Used by
    /// callers (e.g. importers building references, or cooked-format
    /// loaders) that only have an `AssetId` and no human-readable path.
    ///
    /// This is the same per-ID dedup and request-load logic `load_internal`
    /// has always used, extracted so `load_internal` and `load_by_id` share
    /// it: if an asset for `id` isn't already loaded or loading, a load task
    /// is spawned via `request_load`; either way, a handle to `id` is
    /// returned (deduped/reused via `AssetHandleProvider.asset_handles`).
    pub fn load_by_id<A: LoadableAsset + 'static>(&self, id: AssetId) -> AssetHandle<A> {
        if !self.data.pending_tasks.read().unwrap().contains_key(&id)
            && !self.data.loaded_assets.read().unwrap().contains(&id)
        {
            self.request_load::<A>(None, id, A::default_usage_settings());
        }

        self.data.handle_provider.request_handle(id, None)
    }

    fn load_internal<'a, A: LoadableAsset>(
        &self,
        path: impl Into<AssetPath<'a>>,
        usage_settings: A::UsageSettings,
    ) -> AssetHandle<A> {
        let path = path.into().into_owned();

        let id = match self.data.path_to_id.write().unwrap().entry(path.clone()) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => *occupied_entry.get(),
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                *vacant_entry.insert(AssetId::from_path(&path.address()))
            }
        };

        if !self.data.pending_tasks.read().unwrap().contains_key(&id)
            && !self.data.loaded_assets.read().unwrap().contains(&id)
        {
            self.request_load::<A>(Some(path.clone()), id, usage_settings);
        }

        self.data.handle_provider.request_handle(id, Some(path))
    }

    pub fn process_handle_drop(&mut self, id: &AssetId, path: Option<AssetPath<'static>>) {
        self.data.loaded_assets.write().unwrap().remove(id);

        if let Some(path) = path {
            self.data.path_to_id.write().unwrap().remove(&path);
        }
    }

    /// Spawns the async load task for `id`. `path` is the human-readable
    /// asset path used both to actually locate/parse the file today and for
    /// error logging; `load_by_id` callers have no path (they only have an
    /// `AssetId`), so they pass `None` and get an empty placeholder path
    /// here. Cooked-format loaders (added in later tasks) are expected to
    /// resolve their file location from `AssetLoadContext::asset_id()`
    /// instead of relying on this path, which is what makes `load_by_id`
    /// (path-less) usable for them.
    fn request_load<A: LoadableAsset>(
        &self,
        path: Option<AssetPath<'static>>,
        id: AssetId,
        usage_settings: A::UsageSettings,
    ) {
        let asset_loader = A::loader();

        let sender = self.data.asset_load_event_sender.clone();

        let server = self.clone();
        let path = path.unwrap_or_else(|| AssetPath::new(""));
        // No profiling scope around the async body: a scope guard must not be
        // held across .await (tasks can migrate between worker threads).
        // Load costs show up on the named "asset-load-N" threads instead.
        let task =
            LoadTaskPool::get_or_init(|| TaskPool::with_name("asset-load")).spawn(async move {
                let log_path = path.clone();
                let asset = asset_loader
                    .load(path, &mut AssetLoadContext::new(server, id), usage_settings)
                    .await;
                match asset {
                    Ok(asset) => {
                        sender
                            .send(AssetLoadEvent::Loaded(LoadedAsset::new(id, asset)))
                            .unwrap();
                    }
                    Err(error) => {
                        log::error!(
                            "Failed to load asset '{}' (type {}): {:#}",
                            log_path.to_path().display(),
                            std::any::type_name::<A>(),
                            error
                        );
                        sender.send(AssetLoadEvent::LoadFailed(id)).unwrap();
                    }
                }
            });

        self.data.pending_tasks.write().unwrap().insert(id, task);
    }
}

impl Default for AssetServer {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: This shouldn't need to be public
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

struct AssetHandleProvider {
    asset_handles: RwLock<HashMap<AssetId, AssetInfo>>,
    asset_lifetime_send_map: RwLock<HashMap<TypeId, Sender<AssetLifetimeEvent>>>,
}

impl AssetHandleProvider {
    pub fn new() -> Self {
        Self {
            asset_handles: RwLock::new(HashMap::new()),
            asset_lifetime_send_map: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_asset<A: Asset>(&self, lifetime_sender: Sender<AssetLifetimeEvent>) {
        let type_id = TypeId::of::<A>();
        self.asset_lifetime_send_map
            .write()
            .unwrap()
            .insert(type_id, lifetime_sender);
    }

    pub fn request_handle<A: Asset>(
        &self,
        id: AssetId,
        path: Option<AssetPath<'static>>,
    ) -> AssetHandle<A> {
        let lifetime_sender = self
            .asset_lifetime_send_map
            .read()
            .unwrap()
            .get(&TypeId::of::<A>())
            .expect("Asset lifetime sender not found, make sure to register it")
            .clone();

        let mut binding = self.asset_handles.write().unwrap();

        let info = binding.entry(id).or_insert_with(|| AssetInfo {
            handle: Weak::new(),
        });

        if let Some(strong_handle) = info.handle.upgrade() {
            AssetHandle::strong(strong_handle)
        } else {
            let handle = Arc::new(StrongAssetHandle {
                id,
                lifetime_sender,
                path,
            });

            info.handle = Arc::downgrade(&handle);

            AssetHandle::strong(handle)
        }
    }
}
