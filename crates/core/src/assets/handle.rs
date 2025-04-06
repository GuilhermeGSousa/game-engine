use super::Asset;

pub struct AssetHandle<A: Asset> {
    // Temp
    marker: std::marker::PhantomData<A>,
}
