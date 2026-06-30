use std::any::Any;

use essential::{
    assets::handle::AssetHandle,
    geometry::delauney::{Triangle, TriangulatedPoint2D, Triangulation2D},
    utils::AsAny,
};
use glam::Vec2;
use log::warn;

use crate::{
    clip::AnimationClip,
    graph::{AnimationGraph, AnimationNodeContext, AnimationNodeIndex},
    node::{AnimationClipNode, AnimationNode, AnimationNodeInstance},
};

#[derive(AsAny)]
pub struct BlendSpace2DNode {
    triangulation: Triangulation2D,
}

impl BlendSpace2DNode {
    pub(crate) fn new(points: Vec<Vec2>) -> Self {
        Self {
            triangulation: Triangulation2D::build(points),
        }
    }

    pub fn points(&self) -> &[Vec2] {
        self.triangulation.points()
    }

    pub fn triangles(&self) -> &[Triangle] {
        self.triangulation.triangles()
    }
}

impl AnimationNode for BlendSpace2DNode {
    fn create_instance(
        &self,
        _creation_context: &crate::evaluation::AnimationGraphContext,
    ) -> Box<dyn crate::node::AnimationNodeInstance> {
        Box::new(BlendSpace2DInstanceNode::default())
    }
}

#[derive(AsAny, Default)]
pub struct BlendSpace2DInstanceNode {
    current_triangulated_point: Option<TriangulatedPoint2D>,
}

impl BlendSpace2DInstanceNode {}

impl AnimationNodeInstance for BlendSpace2DInstanceNode {
    fn reset(&mut self) {
        self.current_triangulated_point = None;
    }

    fn evaluate(
        &self,
        node: &dyn AnimationNode,
        _context: &crate::evaluation::AnimationGraphContext<'_>,
        _bone_ids: &[uuid::Uuid],
        evaluated_inputs: &[crate::pose::EvaluatedPose],
        _pool: &mut crate::pose::PosePool,
        output: &mut crate::pose::Pose,
    ) {
        if evaluated_inputs.is_empty() {
            return;
        }

        let Some(blend_space) = node.as_any().downcast_ref::<BlendSpace2DNode>() else {
            return;
        };

        if evaluated_inputs.len() != blend_space.points().len() {
            warn!(
                "Blend Space inputs and points count are different, this should not happen. Skipping this node"
            );
            return;
        }

        let Some(triangulated_point) = self.current_triangulated_point else {
            return;
        };

        let triangle = &blend_space.triangles()[triangulated_point.triangle];
        let lambda_a = triangulated_point.lambda_a;
        let lambda_b = triangulated_point.lambda_b;
        let lambda_c = triangulated_point.lambda_c;

        // Start from pose A, then bring in B and C via sequential lerp.
        // lerp(lerp(A, B, λb/(λa+λb)), C, λc) expands to λa*A + λb*B + λc*C
        // because (λa+λb) + λc = 1.
        output.copy_from(&evaluated_inputs[triangle.a].pose);

        let ab_sum = lambda_a + lambda_b;
        if ab_sum > 1e-6 {
            output.blend(&evaluated_inputs[triangle.b].pose, lambda_b / ab_sum);
        }

        output.blend(&evaluated_inputs[triangle.c].pose, lambda_c);
    }

    fn update(
        &mut self,
        _node: &dyn AnimationNode,
        _delta_time: f32,
        _context: &crate::evaluation::AnimationGraphContext<'_>,
    ) {
    }
}

pub struct BlendSpace2DBuilderContext<'a> {
    pub(crate) graph: &'a mut AnimationGraph,
    pub(crate) output_node_index: AnimationNodeIndex,
    pub(crate) points: Vec<Vec2>,
    pub(crate) nodes: Vec<Box<dyn AnimationNode>>,
}

impl<'a> BlendSpace2DBuilderContext<'a> {
    pub(crate) fn build(self) -> AnimationNodeContext<'a> {
        let blend_space = BlendSpace2DNode::new(self.points);

        let blend_space_node = self
            .graph
            .add_node(blend_space, self.output_node_index)
            .index();

        // Add blend space inputs
        for node in self.nodes.into_iter() {
            self.graph.add_boxed_node(node, blend_space_node);
        }

        AnimationNodeContext {
            graph: self.graph,
            node_index: self.output_node_index,
        }
    }

    pub fn input(&mut self, node: impl AnimationNode, point: Vec2) -> &mut Self {
        self.points.push(point);
        self.nodes.push(Box::new(node));
        self
    }

    pub fn animation_clip_input(
        &mut self,
        node: &AssetHandle<AnimationClip>,
        point: Vec2,
    ) -> &mut Self {
        self.points.push(point);
        self.nodes
            .push(Box::new(AnimationClipNode::new(node.clone())));
        self
    }
}
