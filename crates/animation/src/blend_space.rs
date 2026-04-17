use std::any::Any;

use essential::utils::AsAny;
use glam::Vec2;

use crate::{
    evaluation::AnimationGraphContext,
    node::{AnimationNode, AnimationNodeInstance},
    target::AnimationTarget,
};


#[derive(AsAny, Default)]
pub struct BlendNode
{
}

impl AnimationNode for BlendNode {
    fn create_instance(
        &self,
        _creation_context: &crate::evaluation::AnimationGraphContext,
    ) -> Box<dyn crate::node::AnimationNodeInstance> {
        Box::new(BlendNodeInstance {})
    }
}

#[derive(AsAny, Default)]
pub struct BlendNodeInstance
{
}

impl AnimationNodeInstance for BlendNodeInstance {
    fn reset(&mut self) {
        
    }

    fn evaluate(
        &self,
        node: &dyn AnimationNode,
        target: &AnimationTarget,
        evaluated_inputs: &[crate::evaluation::EvaluatedNode],
        context: &AnimationGraphContext<'_>,
    ) -> essential::transform::Transform {
        todo!()
    }

    fn update(
        &mut self,
        node: &dyn AnimationNode,
        delta_time: f32,
        context: &AnimationGraphContext<'_>,
    ) {
        
    }
}

#[derive(AsAny, Default)]
pub struct BlendSpace1DNode
{
    
}

impl AnimationNode for BlendSpace1DNode {
    fn create_instance(
        &self,
        _creation_context: &AnimationGraphContext,
    ) -> Box<dyn AnimationNodeInstance> {
        Box::new(BlendSpace1DNodeInstance::default())
    }
}

#[derive(AsAny, Default)]
pub struct BlendSpace1DNodeInstance
{
    pub(crate) value: f32,
}

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

#[derive(AsAny, Default)]
pub struct BlendSpace2DNodeInstance
{
    pub(crate) value: Vec2
}

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