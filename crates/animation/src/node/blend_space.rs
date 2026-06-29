use std::any::Any;

use essential::{
    geometry::delauney::{Triangle, TriangulatedPoint2D, Triangulation2D},
    utils::AsAny,
};
use glam::Vec2;
use log::warn;

use crate::node::{AnimationNode, AnimationNodeInstance};

#[derive(AsAny)]
pub struct BlendSpace2DNode {
    triangulation: Triangulation2D,
}

impl BlendSpace2DNode {
    pub fn new(points: Vec<Vec2>) -> Self {
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
