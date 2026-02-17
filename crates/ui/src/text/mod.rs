use ecs::component::Component;

pub(crate) mod resources;


#[derive(Component)]
pub struct TextComponent
{
    text: String
}

#[derive(Component)]
pub struct RenderTextComponent
{
    buffer: glyphon::Buffer
}