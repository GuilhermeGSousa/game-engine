pub mod asset_loader;
pub mod asset_server;
pub mod asset_store;
pub mod handle;
pub mod utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(u32);

pub trait Asset {
    fn loader() -> Box<dyn asset_loader::AssetLoader<Asset = Self>>;
    fn id(path: String) -> AssetId;
}
