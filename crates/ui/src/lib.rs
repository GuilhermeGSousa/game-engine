pub mod node;
pub mod plugin;
pub mod render;
pub mod text;
pub mod transform;

mod layouts;
mod resources;
mod vertex;

#[cfg(test)]
mod tests {
    use taffy::{Style, TaffyTree};

    #[test]
    fn test_taffy() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();

        let leaf = taffy
            .new_leaf(Style {
                ..Default::default()
            })
            .unwrap();

        let parent = taffy
            .new_with_children(
                Style {
                    ..Default::default()
                },
                &[leaf],
            )
            .unwrap();

        taffy.compute_layout(
            parent,
            Size {
                height: AvailableSpace::Definite(100.0),
                width: AvailableSpace::Definite(100.0),
            },
        );

        let parent_layout = taffy.layout(parent).unwrap();

    }
}
