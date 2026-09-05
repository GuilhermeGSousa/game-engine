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
    content::AssetRegistry,
    handle::{AssetHandle, AssetLifetimeEvent},
    Asset, AssetId, ContentAssetRoot,
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
    content_root: ContentAssetRoot,
}

impl AssetLoadContext {
    pub fn asset_server(&self) -> &AssetServer {
        &self.asset_server
    }

    pub fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    pub fn content_root(&self) -> &ContentAssetRoot {
        &self.content_root
    }
}

impl AssetLoadContext {
    pub(crate) fn new(
        asset_server: AssetServer,
        asset_id: AssetId,
        content_root: ContentAssetRoot,
    ) -> Self {
        Self {
            asset_server,
            asset_id,
            content_root,
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
    content_root: RwLock<ContentAssetRoot>,
    // Cached after the first path-less (`load_by_id`) resolution; see
    // `AssetServer::resolve_by_id`.
    registry: RwLock<Option<Arc<AssetRegistry>>>,
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
            content_root: RwLock::new(ContentAssetRoot::default_for_platform()),
            registry: RwLock::new(None),
        };

        Self {
            data: Arc::new(server_data),
        }
    }

    /// The root every content-asset loader resolves its address against.
    /// Defaults to [`ContentAssetRoot::default_for_platform`]; override with
    /// [`AssetServer::set_content_root`] before triggering loads.
    pub fn content_root(&self) -> ContentAssetRoot {
        self.data.content_root.read().unwrap().clone()
    }

    pub fn set_content_root(&self, root: ContentAssetRoot) {
        *self.data.content_root.write().unwrap() = root;
        *self.data.registry.write().unwrap() = None;
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
    /// callers (e.g. importers building references) that only have an
    /// `AssetId` and no human-readable path; `request_load` resolves it to
    /// an address via the asset registry.
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

    /// Resolves `id` to its content-tree address via the asset registry, for
    /// a path-less (`load_by_id`) load. Loads and caches the registry from
    /// `content_root()` on first use; concurrent first-use callers may each
    /// load it once — a benign race, not a correctness issue, since the
    /// registry is read-only from the runtime's perspective.
    async fn resolve_by_id(&self, id: AssetId) -> Option<String> {
        let cached = self.data.registry.read().unwrap().clone();
        if let Some(registry) = cached {
            return registry.get(id).map(str::to_owned);
        }

        let root = self.content_root();
        let registry = match crate::assets::utils::load_registry(&root).await {
            Ok(registry) => Arc::new(registry),
            Err(error) => {
                log::error!("failed to load asset registry: {error:#}");
                return None;
            }
        };
        let address = registry.get(id).map(str::to_owned);
        *self.data.registry.write().unwrap() = Some(registry);
        address
    }

    /// Spawns the async load task for `id`. `path` is the human-readable
    /// asset path used both to locate/parse the file and for error logging;
    /// `load_by_id` callers have no path (they only have an `AssetId`), so
    /// they pass `None` and this resolves one via the asset registry before
    /// the loader ever runs — a registry miss fails the load outright rather
    /// than calling the loader with a placeholder path.
    fn request_load<A: LoadableAsset>(
        &self,
        path: Option<AssetPath<'static>>,
        id: AssetId,
        usage_settings: A::UsageSettings,
    ) {
        let asset_loader = A::loader();

        let sender = self.data.asset_load_event_sender.clone();

        let server = self.clone();
        // No profiling scope around the async body: a scope guard must not be
        // held across .await (tasks can migrate between worker threads).
        // Load costs show up on the named "asset-load-N" threads instead.
        let task =
            LoadTaskPool::get_or_init(|| TaskPool::with_name("asset-load")).spawn(async move {
                let path = match path {
                    Some(path) => path,
                    None => match server.resolve_by_id(id).await {
                        Some(address) => AssetPath::new(address),
                        None => {
                            log::error!(
                                "no content asset registered for AssetId {id:?} (type {})",
                                std::any::type_name::<A>()
                            );
                            sender.send(AssetLoadEvent::LoadFailed(id)).unwrap();
                            return;
                        }
                    },
                };
                let log_path = path.clone();
                let content_root = server.content_root();
                let asset = asset_loader
                    .load(
                        path,
                        &mut AssetLoadContext::new(server, id, content_root),
                        usage_settings,
                    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{
        asset_loader::AssetLoader,
        asset_store::AssetStore,
        content::{save_content_asset, AssetRegistry},
    };
    use ecs::world::World;

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("asset-server-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("content")).unwrap();
        dir
    }

    /// A minimal real asset: written to disk by `save_content_asset` and read
    /// back through the same `load_content_asset_bytes` path every shipped
    /// loader uses, so `load_by_id` is exercised end to end rather than
    /// against a mock.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct FixtureAsset {
        value: u32,
    }

    impl Asset for FixtureAsset {
        fn name() -> &'static str {
            "FixtureAsset"
        }
    }

    impl LoadableAsset for FixtureAsset {
        type UsageSettings = ();

        fn loader() -> Box<dyn AssetLoader<Asset = Self>> {
            Box::new(FixtureLoader)
        }

        fn default_usage_settings() -> Self::UsageSettings {}
    }

    struct FixtureLoader;

    #[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
    impl AssetLoader for FixtureLoader {
        type Asset = FixtureAsset;

        async fn load(
            &self,
            path: AssetPath<'static>,
            load_context: &mut AssetLoadContext,
            _usage_settings: (),
        ) -> anyhow::Result<Self::Asset> {
            let bytes = crate::assets::utils::load_content_asset_bytes(
                load_context.content_root(),
                &path.address(),
                FixtureAsset::name(),
            )
            .await?;
            Ok(bincode::deserialize(&bytes)?)
        }
    }

    /// Runs the load task `load_by_id` spawned for `id` to completion.
    ///
    /// Taking the real `Task` out of `pending_tasks` and awaiting it makes the
    /// wait deterministic — the task is a `Future`, so there is no sleeping or
    /// polling — while still running the genuine `LoadTaskPool` task, registry
    /// read and loader.
    fn drive_pending_load(server: &AssetServer, id: AssetId) {
        let task = server
            .data
            .pending_tasks
            .write()
            .unwrap()
            .remove(&id)
            .expect("load_by_id spawns a load task for an id that isn't loaded yet");
        pollster::block_on(task);
    }

    #[test]
    fn resolve_by_id_finds_a_registered_asset() {
        let dir = temp_root("resolve-hit");
        let id = AssetId::from_path("content/hero/scene.gasset");
        let mut registry = AssetRegistry::new();
        registry.insert(id, "content/hero/scene.gasset");
        registry.save(&dir).expect("save registry");

        let server = AssetServer::new();
        server.set_content_root(ContentAssetRoot::Directory(dir.clone()));
        let resolved = pollster::block_on(server.resolve_by_id(id));

        assert_eq!(resolved.as_deref(), Some("content/hero/scene.gasset"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_by_id_returns_none_for_an_unregistered_id() {
        let dir = temp_root("resolve-miss");
        let server = AssetServer::new();
        server.set_content_root(ContentAssetRoot::Directory(dir.clone()));
        let resolved = pollster::block_on(server.resolve_by_id(AssetId::new()));
        assert_eq!(resolved, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_by_id_caches_the_registry_after_first_load() {
        let dir = temp_root("resolve-cache");
        let id = AssetId::from_path("content/hero/scene.gasset");
        let mut registry = AssetRegistry::new();
        registry.insert(id, "content/hero/scene.gasset");
        registry.save(&dir).expect("save registry");

        let server = AssetServer::new();
        server.set_content_root(ContentAssetRoot::Directory(dir.clone()));
        let first = pollster::block_on(server.resolve_by_id(id));
        assert_eq!(first.as_deref(), Some("content/hero/scene.gasset"));

        // Removing the on-disk registry must not affect a cached lookup.
        std::fs::remove_file(dir.join("content/.registry.toml")).unwrap();
        let second = pollster::block_on(server.resolve_by_id(id));
        assert_eq!(
            second.as_deref(),
            Some("content/hero/scene.gasset"),
            "the registry is cached after first use, so a since-deleted file must not affect the second lookup"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_by_id_loads_a_registered_content_asset() {
        let dir = temp_root("load-by-id-hit");
        let address = "content/fixture/value.gasset";
        // Writes both the .gasset file and its registry entry, exactly as
        // `import` and an editor save do.
        save_content_asset(&FixtureAsset { value: 7 }, &dir, address).expect("save content asset");
        let id = AssetId::from_path(address);

        let mut world = World::new();
        let store = AssetStore::<FixtureAsset>::new();
        let mut server = AssetServer::new();
        server.register_asset::<FixtureAsset>(&store);
        server.set_content_root(ContentAssetRoot::Directory(dir.clone()));
        world.insert_resource(store);
        world.insert_resource(server.clone());

        let handle = server.load_by_id::<FixtureAsset>(id);
        drive_pending_load(&server, id);
        handle_asset_load_events(&mut world);

        assert!(
            server.data.loaded_assets.read().unwrap().contains(&id),
            "a registered id must resolve through the registry and finish loading"
        );
        let store = world
            .get_resource::<AssetStore<FixtureAsset>>()
            .expect("asset store");
        assert_eq!(store.get(&handle).map(|asset| asset.value), Some(7));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_by_id_fails_for_an_unregistered_id() {
        let dir = temp_root("load-by-id-miss");
        let store = AssetStore::<FixtureAsset>::new();
        let mut server = AssetServer::new();
        server.register_asset::<FixtureAsset>(&store);
        server.set_content_root(ContentAssetRoot::Directory(dir.clone()));

        let id = AssetId::new();
        let _handle = server.load_by_id::<FixtureAsset>(id);
        drive_pending_load(&server, id);

        match server.data.asset_load_event_receiver.try_recv() {
            Ok(AssetLoadEvent::LoadFailed(failed)) => assert_eq!(failed, id),
            Ok(AssetLoadEvent::Loaded(_)) => panic!("an unregistered id must not produce an asset"),
            Err(error) => panic!("expected a LoadFailed event, got {error}"),
        }
        assert!(!server.data.loaded_assets.read().unwrap().contains(&id));

        std::fs::remove_dir_all(&dir).ok();
    }
}
