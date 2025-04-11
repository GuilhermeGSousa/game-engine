use std::{collections::HashMap, path};

use ecs::resource::Resource;

use crate::tasks::{task_pool::TaskPool, Task};

use super::{handle::AssetHandle, Asset, AssetId};

#[derive(Resource)]
pub struct AssetServer {
    pending_tasks: HashMap<AssetId, Task<()>>,
}

impl AssetServer {
    pub fn new() -> Self {
        AssetServer {
            pending_tasks: HashMap::new(),
        }
    }

    pub fn load<A: Asset>(&mut self, path: &str) -> Result<AssetHandle<A>, String>
    where
        A: 'static,
    {
        let asset_loader = A::loader();
        let id = A::id(path.to_string());
        let task = TaskPool::spawn(async move {
            let asset = asset_loader.load(id.clone()).await;
            match asset {
                Ok(_) => {
                    // TODO
                    ()
                }
                Err(_) => {
                    // TODO
                    ()
                }
            }
        });

        self.pending_tasks.insert(id, task);

        Err("Not implemented".to_string())
    }
}
