use std::ops::Deref;

use derive_more::{Deref, DerefMut};
use ecs::resource::Resource;

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct TextRenderer(pub(crate) glyphon::TextRenderer);

#[derive(Resource, Deref)]
pub(crate) struct TextCache(pub(crate) glyphon::Cache);

#[derive(Resource, Deref)]
pub(crate) struct TextSwashCache(pub(crate) glyphon::SwashCache);

#[derive(Resource, Deref)]
pub(crate) struct TextViewport(pub(crate) glyphon::Viewport);

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct TextFontSystem(pub(crate) glyphon::FontSystem);

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct TextAtlas(pub(crate) glyphon::TextAtlas);
