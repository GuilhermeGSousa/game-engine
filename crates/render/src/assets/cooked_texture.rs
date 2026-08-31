use asset_cook::CookedAsset;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CookedTexture {
    pub width: u32,
    pub height: u32,
    pub srgb: bool,
    pub pixels: Vec<u8>,
}

impl CookedAsset for CookedTexture {
    const TYPE_NAME: &'static str = "Texture";
}
