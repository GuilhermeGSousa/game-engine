#include "internal.h"

#include <Jolt/Physics/Body/BodyCreationSettings.h>
#include <Jolt/Physics/Body/BodyFilter.h>
#include <Jolt/Physics/Body/BodyLock.h>
#include <Jolt/Physics/Collision/CollideShape.h>
#include <Jolt/Physics/Collision/Shape/BoxShape.h>
#include <Jolt/Physics/Collision/Shape/CapsuleShape.h>
#include <Jolt/Physics/Collision/Shape/RotatedTranslatedShape.h>
#include <Jolt/Physics/Collision/Shape/SphereShape.h>

#include <cfloat>
#include <cmath>

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

	void jolt_body_creation_settings_set_shape_offset(JoltBodyCreationSettings *settings,
													  const float offset[3])
	{
		settings->shape_offset = JPH::Vec3(offset[0], offset[1], offset[2]);
	}

	JoltBodyId jolt_body_create(JoltWorld *world, const JoltBodyCreationSettings *settings)
	{
		// Copy so the offset wrap never mutates the caller's reusable settings.
		JPH::BodyCreationSettings creation = settings->settings;
		if (!settings->shape_offset.IsNearZero())
			creation.SetShape(new JPH::RotatedTranslatedShape(settings->shape_offset,
															  JPH::Quat::sIdentity(),
															  creation.GetShape()));

		JPH::EActivation activation = creation.mMotionType == JPH::EMotionType::Static
										  ? JPH::EActivation::DontActivate
										  : JPH::EActivation::Activate;
		JPH::BodyID id = world->system.GetBodyInterface().CreateAndAddBody(creation, activation);
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

	// Transcribed from JPH::Character::PostSimulation / CheckCollision, which
	// bundle this logic into a class that insists on owning its body.
	void jolt_body_probe_ground(const JoltWorld *world,
								JoltBodyId body,
								float max_separation,
								float max_slope_angle,
								JoltGroundProbeResult *out_result)
	{
		out_result->state = JOLT_GROUND_STATE_IN_AIR;

		const JPH::PhysicsSystem &system = world->system;
		JPH::BodyID id(body);

		JPH::RVec3 position;
		JPH::Quat rotation;
		JPH::Vec3 velocity;
		JPH::RefConst<JPH::Shape> shape;
		JPH::ObjectLayer layer;
		{
			JPH::BodyLockRead lock(system.GetBodyLockInterface(), id);
			if (!lock.Succeeded())
				return;
			const JPH::Body &b = lock.GetBody();
			position = b.GetPosition();
			rotation = b.GetRotation();
			velocity = b.GetLinearVelocity();
			shape = b.GetShape();
			layer = b.GetObjectLayer();
		}

		// Keeps the hit whose normal points most upward.
		class Collector final : public JPH::CollideShapeCollector
		{
		public:
			explicit Collector(JPH::RVec3 base_offset) : base_offset(base_offset) {}

			void AddHit(const JPH::CollideShapeResult &result) override
			{
				JPH::Vec3 normal = -result.mPenetrationAxis.Normalized();
				float dot = normal.Dot(JPH::Vec3::sAxisY());
				if (dot > best_dot)
				{
					ground_body = result.mBodyID2;
					ground_position = base_offset + result.mContactPointOn2;
					ground_normal = normal;
					best_dot = dot;
				}
			}

			JPH::BodyID ground_body;
			JPH::RVec3 ground_position = JPH::RVec3::sZero();
			JPH::Vec3 ground_normal = JPH::Vec3::sZero();

		private:
			JPH::RVec3 base_offset;
			float best_dot = -FLT_MAX;
		};

		JPH::CollideShapeSettings settings;
		settings.mMaxSeparationDistance = max_separation;
		settings.mActiveEdgeMode = JPH::EActiveEdgeMode::CollideOnlyWithActive;
		settings.mActiveEdgeMovementDirection = velocity;
		settings.mBackFaceMode = JPH::EBackFaceMode::IgnoreBackFaces;

		JPH::RMat44 center_of_mass = JPH::RMat44::sRotationTranslation(rotation, position)
										 .PreTranslated(shape->GetCenterOfMass());
		JPH::DefaultBroadPhaseLayerFilter broadphase_filter =
			system.GetDefaultBroadPhaseLayerFilter(layer);
		JPH::DefaultObjectLayerFilter object_filter = system.GetDefaultLayerFilter(layer);
		JPH::IgnoreSingleBodyFilter body_filter(id);

		Collector collector(position);
		system.GetNarrowPhaseQuery().CollideShape(shape,
												  JPH::Vec3::sReplicate(1.0f),
												  center_of_mass,
												  settings,
												  position,
												  collector,
												  broadphase_filter,
												  object_filter,
												  body_filter);

		if (collector.ground_body.IsInvalid())
			return;

		JPH::Vec3 ground_velocity = JPH::Vec3::sZero();
		{
			JPH::BodyLockRead lock(system.GetBodyLockInterface(), collector.ground_body);
			if (lock.Succeeded())
				ground_velocity = lock.GetBody().GetPointVelocity(collector.ground_position);
		}

		out_result->state = collector.ground_normal.Dot(JPH::Vec3::sAxisY()) < std::cos(max_slope_angle)
								? JOLT_GROUND_STATE_ON_STEEP_GROUND
								: JOLT_GROUND_STATE_ON_GROUND;
		out_result->body = collector.ground_body.GetIndexAndSequenceNumber();
		out_result->position[0] = static_cast<float>(collector.ground_position.GetX());
		out_result->position[1] = static_cast<float>(collector.ground_position.GetY());
		out_result->position[2] = static_cast<float>(collector.ground_position.GetZ());
		out_result->normal[0] = collector.ground_normal.GetX();
		out_result->normal[1] = collector.ground_normal.GetY();
		out_result->normal[2] = collector.ground_normal.GetZ();
		out_result->velocity[0] = ground_velocity.GetX();
		out_result->velocity[1] = ground_velocity.GetY();
		out_result->velocity[2] = ground_velocity.GetZ();
	}

	void jolt_body_set_linear_velocity(JoltWorld *world,
									   JoltBodyId body,
									   const float velocity[3])
	{
		JPH::BodyInterface &bi = world->system.GetBodyInterface();
		JPH::BodyID id(body);
		JPH::Vec3 v(velocity[0], velocity[1], velocity[2]);
		bi.SetLinearVelocity(id, v);
		// SetLinearVelocity leaves a sleeping body asleep; a character idle
		// long enough to sleep must still respond to input.
		if (!v.IsNearZero())
			bi.ActivateBody(id);
	}

	void jolt_body_get_linear_velocity(const JoltWorld *world,
									   JoltBodyId body,
									   float out_velocity[3])
	{
		JPH::Vec3 v = const_cast<JoltWorld *>(world)->system.GetBodyInterface().GetLinearVelocity(
			JPH::BodyID(body));
		out_velocity[0] = v.GetX();
		out_velocity[1] = v.GetY();
		out_velocity[2] = v.GetZ();
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
