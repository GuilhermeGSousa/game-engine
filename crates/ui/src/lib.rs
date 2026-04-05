pub mod interaction;
pub mod material;
pub mod node;
pub mod plugin;
pub mod render;
pub mod text;
pub mod transform;

mod resources;
mod vertex;

// Re-export commonly needed taffy alignment types so users don't need
// to add taffy as a direct dependency.
pub use taffy::{AlignItems, JustifyContent};

#[cfg(test)]
mod tests {
    use taffy::{AvailableSpace, Dimension, LengthPercentage, Rect, Size, Style, TaffyTree};

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
                    width: Dimension::percent(1.0),
                    height: Dimension::percent(0.1),
                },
                ..Default::default()
            })
            .unwrap();
        let root = taffy
            .new_with_children(
                Style {
                    flex_direction: taffy::FlexDirection::Column,
                    size: Size {
                        width: Dimension::percent(1.0),
                        height: Dimension::percent(1.0),
                    },
                    ..Default::default()
                },
                &[spacer, bottom_box],
            )
            .unwrap();

        taffy
            .compute_layout(
                root,
                Size {
                    height: AvailableSpace::Definite(800.0),
                    width: AvailableSpace::Definite(600.0),
                },
            )
            .expect("Error computing layout");

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

    /// Padding on a flex container should shrink the available space for its
    /// children.  A child with `flex_grow: 1` inside a 100×100 container with
    /// 10 px padding on every side should be sized 80×80 and placed at (10, 10).
    #[test]
    fn test_padding_shrinks_children() {
        let mut taffy: TaffyTree<()> = TaffyTree::new();

        let padding = LengthPercentage::length(10.0);
        let child = taffy
            .new_leaf(Style {
                flex_grow: 1.0,
                ..Default::default()
            })
            .unwrap();
        let root = taffy
            .new_with_children(
                Style {
                    size: Size {
                        width: Dimension::length(100.0),
                        height: Dimension::length(100.0),
                    },
                    padding: Rect {
                        top: padding,
                        right: padding,
                        bottom: padding,
                        left: padding,
                    },
                    ..Default::default()
                },
                &[child],
            )
            .unwrap();

        taffy
            .compute_layout(
                root,
                Size {
                    width: AvailableSpace::Definite(600.0),
                    height: AvailableSpace::Definite(800.0),
                },
            )
            .expect("Error computing layout");

        let child_layout = taffy.layout(child).unwrap();
        assert_eq!(child_layout.location.x, 10.0);
        assert_eq!(child_layout.location.y, 10.0);
        assert_eq!(child_layout.size.width, 80.0);
        assert_eq!(child_layout.size.height, 80.0);
    }

    /// Interaction hit-testing logic: a cursor inside the node bounds should
    /// be detected as hovering, one outside should not.
    #[test]
    fn test_interaction_hit_test() {
        // Simulate a node at (50, 50) with size 100×40.
        let x = 50.0_f32;
        let y = 50.0_f32;
        let w = 100.0_f32;
        let h = 40.0_f32;

        let is_hit = |cx: f32, cy: f32| -> bool {
            cx >= x && cx <= x + w && cy >= y && cy <= y + h
        };

        assert!(is_hit(50.0, 50.0), "top-left corner should be inside");
        assert!(is_hit(150.0, 90.0), "bottom-right corner should be inside");
        assert!(is_hit(100.0, 70.0), "center should be inside");
        assert!(!is_hit(49.9, 70.0), "just left of node should be outside");
        assert!(!is_hit(100.0, 91.0), "just below node should be outside");
    }
}
