#include "internal.h"

#include <Jolt/Physics/Body/BodyCreationSettings.h>
#include <Jolt/Physics/Body/BodyLock.h>
#include <Jolt/Physics/Collision/Shape/BoxShape.h>
#include <Jolt/Physics/Collision/Shape/CapsuleShape.h>
#include <Jolt/Physics/Collision/Shape/SphereShape.h>

#include "body.h"

extern "C"
{

	JoltBodyCreationSettings *jolt_body_creation_settings_create(void)
	{
		JoltBodyCreationSettings *settings = new JoltBodyCreationSettings();
		settings->settings.mMotionType = JPH::EMotionType::Static;
		settings->settings.mObjectLayer = Layers::NON_MOVING;
		return settings;
	}

	void jolt_body_creation_settings_destroy(JoltBodyCreationSettings *settings)
	{
		delete settings;
	}

	void jolt_body_creation_settings_set_position(JoltBodyCreationSettings *settings,
												  const float position[3])
	{
		settings->settings.mPosition = JPH::RVec3(position[0], position[1], position[2]);
	}

	void jolt_body_creation_settings_set_rotation(JoltBodyCreationSettings *settings,
												  const float rotation[4])
	{
		settings->settings.mRotation =
			JPH::Quat(rotation[0], rotation[1], rotation[2], rotation[3]);
	}

	void jolt_body_creation_settings_set_motion_type(JoltBodyCreationSettings *settings,
													 JoltMotionType motion_type)
	{
		settings->settings.mMotionType = static_cast<JPH::EMotionType>(motion_type);
		settings->settings.mObjectLayer =
			motion_type == JOLT_MOTION_TYPE_STATIC ? Layers::NON_MOVING : Layers::MOVING;
	}

	void jolt_body_creation_settings_set_allowed_dofs(JoltBodyCreationSettings *settings,
													  JoltAllowedDofs allowed_dofs)
	{
		settings->settings.mAllowedDOFs = static_cast<JPH::EAllowedDOFs>(allowed_dofs);
	}

	void jolt_body_creation_settings_set_sphere_shape(JoltBodyCreationSettings *settings,
													  float radius,
													  float density)
	{
		JPH::SphereShape *shape = new JPH::SphereShape(radius);
		shape->SetDensity(density);
		settings->settings.SetShape(shape);
	}

	void jolt_body_creation_settings_set_box_shape(JoltBodyCreationSettings *settings,
												   const float half_extents[3],
												   float density)
	{
		JPH::BoxShape *shape =
			new JPH::BoxShape(JPH::Vec3(half_extents[0], half_extents[1], half_extents[2]));
		shape->SetDensity(density);
		settings->settings.SetShape(shape);
	}

	void jolt_body_creation_settings_set_capsule_shape(JoltBodyCreationSettings *settings,
													   float half_height,
													   float radius,
													   float density)
	{
		JPH::CapsuleShape *shape = new JPH::CapsuleShape(half_height, radius);
		shape->SetDensity(density);
		settings->settings.SetShape(shape);
	}

	JoltBodyId jolt_body_create(JoltWorld *world, const JoltBodyCreationSettings *settings)
	{
		JPH::EActivation activation = settings->settings.mMotionType == JPH::EMotionType::Static
										  ? JPH::EActivation::DontActivate
										  : JPH::EActivation::Activate;
		JPH::BodyID id =
			world->system.GetBodyInterface().CreateAndAddBody(settings->settings, activation);
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
