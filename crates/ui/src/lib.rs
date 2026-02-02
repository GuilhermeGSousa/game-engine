pub mod material;
pub mod node;
pub mod plugin;
pub mod render;
pub mod text;
pub mod transform;

mod layout;
mod resources;
mod vertex;

#[cfg(test)]
mod tests {
    use taffy::{
        AvailableSpace, Dimension, LengthPercentage, Size, Style, TaffyTree,
        prelude::{FromLength, FromPercent},
    };

    #[test]
    fn test_taffy() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();

        let spacer = taffy
            .new_leaf(Style {
                flex_grow: 1.0,
                ..Default::default()
            })
            .unwrap();
        let bottom_box = taffy
            .new_leaf(Style {
                size: Size {
                    width: Dimension::from_percent(1.0),
                    height: Dimension::from_length(10.0),
                },
                ..Default::default()
            })
            .unwrap();
        let root = taffy
            .new_with_children(
                Style {
                    flex_direction: taffy::FlexDirection::Column,
                    size: Size {
                        width: Dimension::from_length(800.0),
                        height: Dimension::from_length(600.0),
                    },
                    ..Default::default()
                },
                &[spacer, bottom_box],
            )
            .unwrap();

        taffy.compute_layout(
            root,
            Size {
                height: AvailableSpace::Definite(800.0),
                width: AvailableSpace::Definite(600.0),
            },
        );

        let spacer_layout = taffy.layout(spacer).unwrap();
        println!(
            "Spacer location: ({}, {})",
            spacer_layout.location.x, spacer_layout.location.y
        );
        println!(
            "Spacer size: ({}, {})",
            spacer_layout.size.width, spacer_layout.size.height
        );

        let box_layout = taffy.layout(bottom_box).unwrap();
        println!(
            "Box location: ({}, {})",
            box_layout.location.x, box_layout.location.y
        );
        println!(
            "Box size: ({}, {})",
            box_layout.size.width, box_layout.size.height
        );
    }

    #[test]
    fn bottom_box() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();

        let spacer = taffy
            .new_leaf(Style {
                flex_grow: 1.0,
                ..Default::default()
            })
            .unwrap();
        let bottom_box = taffy
            .new_leaf(Style {
                size: Size {
                    width: Dimension::from_percent(100.0),
                    height: Dimension::from_percent(10.0),
                },
                ..Default::default()
            })
            .unwrap();
        let parent = taffy
            .new_with_children(
                Style {
                    flex_direction: taffy::FlexDirection::Column,
                    ..Default::default()
                },
                &[spacer, bottom_box],
            )
            .unwrap();

        taffy.compute_layout(
            parent,
            Size {
                height: AvailableSpace::Definite(800.0),
                width: AvailableSpace::Definite(600.0),
            },
        );

        let box_layout = taffy.layout(bottom_box).unwrap();
        assert_eq!(box_layout.size.height, 80.0);
        assert_eq!(box_layout.size.width, 600.0);
        assert_eq!(box_layout.location.y, 720.0);
    }
}
