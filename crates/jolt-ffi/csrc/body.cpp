#include "internal.h"

#include <Jolt/Physics/Body/BodyCreationSettings.h>
#include <Jolt/Physics/Body/BodyLock.h>
#include <Jolt/Physics/Collision/Shape/BoxShape.h>
#include <Jolt/Physics/Collision/Shape/SphereShape.h>

#include "body.h"

extern "C"
{

	JoltBodyId jolt_body_create_dynamic_sphere(JoltWorld *world,
											   const float position[3],
											   float radius,
											   float density)
	{
		JPH::SphereShape *shape = new JPH::SphereShape(radius);
		shape->SetDensity(density);
		JPH::BodyCreationSettings settings(shape,
										   JPH::RVec3(position[0], position[1], position[2]),
										   JPH::Quat::sIdentity(),
										   JPH::EMotionType::Dynamic,
										   Layers::MOVING);
		JPH::BodyID id = world->system.GetBodyInterface().CreateAndAddBody(
			settings, JPH::EActivation::Activate);
		return id.GetIndexAndSequenceNumber();
	}

	JoltBodyId jolt_body_create_dynamic_box(JoltWorld *world,
											const float position[3],
											const float half_extents[3],
											float density)
	{
		JPH::BoxShape *shape =
			new JPH::BoxShape(JPH::Vec3(half_extents[0], half_extents[1], half_extents[2]));
		shape->SetDensity(density);
		JPH::BodyCreationSettings settings(shape,
										   JPH::RVec3(position[0], position[1], position[2]),
										   JPH::Quat::sIdentity(),
										   JPH::EMotionType::Dynamic,
										   Layers::MOVING);
		JPH::BodyID id = world->system.GetBodyInterface().CreateAndAddBody(
			settings, JPH::EActivation::Activate);
		return id.GetIndexAndSequenceNumber();
	}

	JoltBodyId jolt_body_create_static_box(JoltWorld *world,
										   const float position[3],
										   const float half_extents[3])
	{
		JPH::BodyCreationSettings settings(
			new JPH::BoxShape(JPH::Vec3(half_extents[0], half_extents[1], half_extents[2])),
			JPH::RVec3(position[0], position[1], position[2]),
			JPH::Quat::sIdentity(),
			JPH::EMotionType::Static,
			Layers::NON_MOVING);
		JPH::BodyID id = world->system.GetBodyInterface().CreateAndAddBody(
			settings, JPH::EActivation::DontActivate);
		return id.GetIndexAndSequenceNumber();
	}

	JoltBodyId jolt_body_create_static_sphere(JoltWorld *world,
											  const float position[3],
											  float radius)
	{
		JPH::BodyCreationSettings settings(new JPH::SphereShape(radius),
										   JPH::RVec3(position[0], position[1], position[2]),
										   JPH::Quat::sIdentity(),
										   JPH::EMotionType::Static,
										   Layers::NON_MOVING);
		JPH::BodyID id = world->system.GetBodyInterface().CreateAndAddBody(
			settings, JPH::EActivation::DontActivate);
		return id.GetIndexAndSequenceNumber();
	}

	void jolt_body_destroy(JoltWorld *world, JoltBodyId body)
	{
		JPH::BodyInterface &bi = world->system.GetBodyInterface();
		JPH::BodyID id(body);

		// Jolt does not wake bodies that were touching a removed body, which
		// would leave anything sleeping on top of it floating in mid-air. Grab
		// the body's bounds (before removal invalidates broad-phase state), then
		// activate everything overlapping them once the body is out.
		JPH::AABox bounds;
		{
			JPH::BodyLockRead lock(world->system.GetBodyLockInterface(), id);
			if (lock.Succeeded())
				bounds = lock.GetBody().GetWorldSpaceBounds();
		}

		bi.RemoveBody(id);
		if (bounds.IsValid())
		{
			bounds.ExpandBy(JPH::Vec3::sReplicate(0.05f)); // contact tolerance
			bi.ActivateBodiesInAABox(bounds,
									 JPH::BroadPhaseLayerFilter(),
									 JPH::ObjectLayerFilter());
		}
		bi.DestroyBody(id);
	}

	void jolt_body_get_transform(const JoltWorld *world,
								 JoltBodyId body,
								 float out_position[3],
								 float out_rotation[4])
	{
		const JPH::BodyInterface &bi = const_cast<JoltWorld *>(world)->system.GetBodyInterface();

		JPH::RVec3 position = bi.GetPosition(JPH::BodyID(body));
		out_position[0] = static_cast<float>(position.GetX());
		out_position[1] = static_cast<float>(position.GetY());
		out_position[2] = static_cast<float>(position.GetZ());

		JPH::Quat rotation = bi.GetRotation(JPH::BodyID(body));
		out_rotation[0] = rotation.GetX();
		out_rotation[1] = rotation.GetY();
		out_rotation[2] = rotation.GetZ();
		out_rotation[3] = rotation.GetW();
	}

} // extern "C"
