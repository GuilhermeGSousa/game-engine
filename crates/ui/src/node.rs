use ecs::{command::CommandQueue, component::Component, entity::hierarchy::Children, query::Query};
use taffy::{Dimension, Style, TaffyTree};

use crate::transform::{UITransform, UIValue};

// User defined layout data
#[derive(Component)]
pub struct UINode {
    pub width: UIValue,
    pub height: UIValue,
}

// Post taffy computed layout data
#[derive(Component)]
pub(crate) struct UIComputedNode;



pub(crate) fn compute_ui_nodes(
    ui_nodes: Query<(&UINode, &UITransform, &Children)>,
    cmd: CommandQueue)
{
    let mut taffy: TaffyTree<()> = TaffyTree::new();

    for(node, transform, children) in ui_nodes.iter()
    {
        Dimension{};
        let style = Style{
            ..Default::default()
        };
        if let Ok(node_layout) = taffy.new_leaf(style).and_then(|node_id|taffy.layout(node_id))
        {
            let node_location = node_layout.location;
            let node_size = node_layout.size;
        }
    }
}