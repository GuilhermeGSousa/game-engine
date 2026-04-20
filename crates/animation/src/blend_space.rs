use std::any::Any;

use essential::{blend::Blendable, transform::Transform, utils::AsAny};
use glam::Vec2;

use crate::{
    evaluation::AnimationGraphContext,
    node::{AnimationNode, AnimationNodeInstance},
    target::AnimationTarget,
};

#[derive(AsAny, Default)]
pub struct BlendNode {}

impl AnimationNode for BlendNode {
    fn create_instance(
        &self,
        _creation_context: &crate::evaluation::AnimationGraphContext,
    ) -> Box<dyn crate::node::AnimationNodeInstance> {
        Box::new(BlendNodeInstance {})
    }
}

#[derive(AsAny, Default)]
pub struct BlendNodeInstance {}

impl AnimationNodeInstance for BlendNodeInstance {
    fn reset(&mut self) {}

    fn evaluate(
        &self,
        _node: &dyn AnimationNode,
        _target: &AnimationTarget,
        evaluated_inputs: &[crate::evaluation::EvaluatedNode],
        _context: &AnimationGraphContext<'_>,
    ) -> essential::transform::Transform {
        let Some(first_input) = evaluated_inputs.first() else {
            return Transform::IDENTITY;
        };

        let mut result = first_input.transform.clone();
        let mut accumulated_input = first_input.weight;

        for input in &evaluated_inputs[1..] {
            accumulated_input += input.weight;

            if accumulated_input.abs() <= f32::EPSILON {
                continue;
            }
            result = Transform::interpolate(
                result,
                input.transform.clone(),
                input.weight / accumulated_input,
            );
        }

        result
    }

    fn update(
        &mut self,
        _node: &dyn AnimationNode,
        _delta_time: f32,
        _context: &AnimationGraphContext<'_>,
    ) {
    }
}

#[derive(AsAny, Default)]
pub struct BlendSpace1DNode {}

impl AnimationNode for BlendSpace1DNode {
    fn create_instance(
        &self,
        _creation_context: &AnimationGraphContext,
    ) -> Box<dyn AnimationNodeInstance> {
        Box::new(BlendSpace1DNodeInstance::default())
    }
}

#[derive(AsAny, Default)]
pub struct BlendSpace1DNodeInstance {
    pub(crate) value: f32,
}

impl AnimationNodeInstance for BlendSpace1DNodeInstance {
    fn reset(&mut self) {}

    fn evaluate(
        &self,
        _node: &dyn AnimationNode,
        _target: &crate::target::AnimationTarget,
        evaluated_inputs: &[crate::evaluation::EvaluatedNode],
        _context: &crate::evaluation::AnimationGraphContext<'_>,
    ) -> essential::transform::Transform {
        let Some(first_input) = evaluated_inputs.first() else {
            return Transform::IDENTITY;
        };

        let Some(second_input) = evaluated_inputs.get(1) else {
            return Transform::IDENTITY;
        };

        Transform::interpolate(
            first_input.transform.clone(),
            second_input.transform.clone(),
            self.value.clamp(0.0, 1.0),
        )
    }

    fn update(
        &mut self,
        _node: &dyn AnimationNode,
        _delta_time: f32,
        _context: &AnimationGraphContext<'_>,
    ) {
    }
}

#[derive(AsAny)]
pub struct BlendSpace2DNode {}

impl AnimationNode for BlendSpace2DNode {
    fn create_instance(
        &self,
        _creation_context: &crate::evaluation::AnimationGraphContext,
    ) -> Box<dyn crate::node::AnimationNodeInstance> {
        todo!()
    }
}

#[derive(AsAny, Default)]
pub struct BlendSpace2DNodeInstance {
    pub(crate) value: Vec2,
}

impl AnimationNodeInstance for BlendSpace2DNodeInstance {
    fn reset(&mut self) {}

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
        _node: &dyn AnimationNode,
        _delta_time: f32,
        _context: &AnimationGraphContext<'_>,
    ) {
    }
}
