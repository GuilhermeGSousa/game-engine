pub mod asset_loader;
pub mod asset_manager;
pub mod asset_store;
pub mod handle;
pub mod utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(u32);

pub trait Asset {}
