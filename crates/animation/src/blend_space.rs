use std::any::Any;

use essential::utils::AsAny;

use crate::node::{AnimationNode, AnimationNodeInstance};


#[derive(AsAny)]
pub struct BlendSpace1DNode
{
}

impl AnimationNode for BlendSpace1DNode {
    fn create_instance(
        &self,
        _creation_context: &crate::evaluation::AnimationGraphContext,
    ) -> Box<dyn crate::node::AnimationNodeInstance> {
        todo!()
    }
}

#[derive(AsAny)]
pub struct BlendSpace1DNodeInstance
{}

impl AnimationNodeInstance for BlendSpace1DNodeInstance {
    fn reset(&mut self) {
        todo!()
    }

    fn evaluate(
        &self,
        node: &dyn AnimationNode,
        target: &crate::target::AnimationTarget,
        evaluated_inputs: &[crate::evaluation::EvaluatedNode],
        context: &crate::evaluation::AnimationGraphContext<'_>,
    ) -> essential::transform::Transform {
        todo!()
    }

    fn update(
        &mut self,
        node: &dyn AnimationNode,
        delta_time: f32,
        context: &crate::evaluation::AnimationGraphContext<'_>,
    ) {
        todo!()
    }
}

#[derive(AsAny)]
pub struct BlendSpace2DNode
{
}

impl AnimationNode for BlendSpace2DNode {
    fn create_instance(
        &self,
        _creation_context: &crate::evaluation::AnimationGraphContext,
    ) -> Box<dyn crate::node::AnimationNodeInstance> {
        todo!()
    }
}

#[derive(AsAny)]
pub struct BlendSpace2DNodeInstance
{}

impl AnimationNodeInstance for BlendSpace2DNodeInstance {
    fn reset(&mut self) {
        todo!()
    }

    fn evaluate(
        &self,
        node: &dyn AnimationNode,
        target: &crate::target::AnimationTarget,
        evaluated_inputs: &[crate::evaluation::EvaluatedNode],
        context: &crate::evaluation::AnimationGraphContext<'_>,
    ) -> essential::transform::Transform {
        todo!()
    }

    fn update(
        &mut self,
        node: &dyn AnimationNode,
        delta_time: f32,
        context: &crate::evaluation::AnimationGraphContext<'_>,
    ) {
        todo!()
    }
}