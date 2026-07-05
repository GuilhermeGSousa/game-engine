// Implementation of the C API declared in shim.h, modelled on Jolt's
// HelloWorld sample. Compiled together with the vendored Jolt sources in a
// single cc::Build invocation, so every translation unit sees the same JPH_*
// configuration defines (see build.rs) and Jolt's RegisterTypes version check
// passes by construction.

#include <Jolt/Jolt.h>

#include <Jolt/Core/Factory.h>
#include <Jolt/Core/JobSystemThreadPool.h>
#include <Jolt/Core/TempAllocator.h>
#include <Jolt/Physics/Body/BodyCreationSettings.h>
#include <Jolt/Physics/Body/BodyLock.h>
#include <Jolt/Physics/Collision/CastResult.h>
#include <Jolt/Physics/Collision/RayCast.h>
#include <Jolt/Physics/Collision/Shape/BoxShape.h>
#include <Jolt/Physics/Collision/Shape/SphereShape.h>
#include <Jolt/Physics/PhysicsSettings.h>
#include <Jolt/Physics/PhysicsSystem.h>
#include <Jolt/RegisterTypes.h>

#include <cstdarg>
#include <cstdio>
#include <mutex>

#include "shim.h"

JPH_SUPPRESS_WARNINGS

namespace {

void TraceImpl(const char *inFMT, ...)
{
	va_list list;
	va_start(list, inFMT);
	char buffer[1024];
	vsnprintf(buffer, sizeof(buffer), inFMT, list);
	va_end(list);

	fprintf(stderr, "[jolt] %s\n", buffer);
}

// Object layers: statics only collide with dynamics; dynamics collide with
// everything.
namespace Layers {
	constexpr JPH::ObjectLayer NON_MOVING = 0;
	constexpr JPH::ObjectLayer MOVING = 1;
	constexpr JPH::uint NUM_LAYERS = 2;
} // namespace Layers

namespace BroadPhaseLayers {
	constexpr JPH::BroadPhaseLayer NON_MOVING(0);
	constexpr JPH::BroadPhaseLayer MOVING(1);
	constexpr JPH::uint NUM_LAYERS = 2;
} // namespace BroadPhaseLayers

class ObjectLayerPairFilterImpl final : public JPH::ObjectLayerPairFilter
{
public:
	bool ShouldCollide(JPH::ObjectLayer inObject1, JPH::ObjectLayer inObject2) const override
	{
		switch (inObject1)
		{
		case Layers::NON_MOVING:
			return inObject2 == Layers::MOVING;
		case Layers::MOVING:
			return true;
		default:
			JPH_ASSERT(false);
			return false;
		}
	}
};

class BPLayerInterfaceImpl final : public JPH::BroadPhaseLayerInterface
{
public:
	BPLayerInterfaceImpl()
	{
		mObjectToBroadPhase[Layers::NON_MOVING] = BroadPhaseLayers::NON_MOVING;
		mObjectToBroadPhase[Layers::MOVING] = BroadPhaseLayers::MOVING;
	}

	JPH::uint GetNumBroadPhaseLayers() const override
	{
		return BroadPhaseLayers::NUM_LAYERS;
	}

	JPH::BroadPhaseLayer GetBroadPhaseLayer(JPH::ObjectLayer inLayer) const override
	{
		JPH_ASSERT(inLayer < Layers::NUM_LAYERS);
		return mObjectToBroadPhase[inLayer];
	}

#if defined(JPH_EXTERNAL_PROFILE) || defined(JPH_PROFILE_ENABLED)
	const char *GetBroadPhaseLayerName(JPH::BroadPhaseLayer inLayer) const override
	{
		switch ((JPH::BroadPhaseLayer::Type)inLayer)
		{
		case (JPH::BroadPhaseLayer::Type)BroadPhaseLayers::NON_MOVING:
			return "NON_MOVING";
		case (JPH::BroadPhaseLayer::Type)BroadPhaseLayers::MOVING:
			return "MOVING";
		default:
			JPH_ASSERT(false);
			return "INVALID";
		}
	}
#endif

private:
	JPH::BroadPhaseLayer mObjectToBroadPhase[Layers::NUM_LAYERS];
};

class ObjectVsBroadPhaseLayerFilterImpl final : public JPH::ObjectVsBroadPhaseLayerFilter
{
public:
	bool ShouldCollide(JPH::ObjectLayer inLayer1, JPH::BroadPhaseLayer inLayer2) const override
	{
		switch (inLayer1)
		{
		case Layers::NON_MOVING:
			return inLayer2 == BroadPhaseLayers::MOVING;
		case Layers::MOVING:
			return true;
		default:
			JPH_ASSERT(false);
			return false;
		}
	}
};

} // namespace

// The layer interfaces are declared before the system: PhysicsSystem::Init
// stores references to them, so they must be destroyed after it (members are
// destroyed in reverse declaration order).
struct JoltWorld {
	BPLayerInterfaceImpl broad_phase_layer_interface;
	ObjectVsBroadPhaseLayerFilterImpl object_vs_broad_phase_layer_filter;
	ObjectLayerPairFilterImpl object_layer_pair_filter;
	JPH::PhysicsSystem system;
};

struct JoltStepper {
	JPH::TempAllocatorImpl temp_allocator;
	JPH::JobSystemThreadPool job_system;

	explicit JoltStepper(uint32_t temp_allocator_bytes)
		: temp_allocator(temp_allocator_bytes),
		  // -1 worker threads = hardware concurrency - 1.
		  job_system(JPH::cMaxPhysicsJobs, JPH::cMaxPhysicsBarriers, -1)
	{
	}
};

extern "C" {

void jolt_global_init(void)
{
	static std::once_flag once;
	std::call_once(once, [] {
		JPH::RegisterDefaultAllocator();
		JPH::Trace = TraceImpl;
		JPH::Factory::sInstance = new JPH::Factory();
		JPH::RegisterTypes();
	});
}

JoltWorld *jolt_world_create(uint32_t max_bodies,
                             uint32_t num_body_mutexes,
                             uint32_t max_body_pairs,
                             uint32_t max_contact_constraints)
{
	jolt_global_init();

	JoltWorld *world = new JoltWorld();
	world->system.Init(max_bodies,
	                   num_body_mutexes,
	                   max_body_pairs,
	                   max_contact_constraints,
	                   world->broad_phase_layer_interface,
	                   world->object_vs_broad_phase_layer_filter,
	                   world->object_layer_pair_filter);
	return world;
}

void jolt_world_destroy(JoltWorld *world)
{
	delete world;
}

JoltStepper *jolt_stepper_create(uint32_t temp_allocator_bytes)
{
	jolt_global_init();
	return new JoltStepper(temp_allocator_bytes);
}

void jolt_stepper_destroy(JoltStepper *stepper)
{
	delete stepper;
}

uint32_t jolt_world_step(JoltWorld *world,
                         JoltStepper *stepper,
                         float delta_time,
                         int collision_steps)
{
	JPH::EPhysicsUpdateError error = world->system.Update(delta_time,
	                                                      collision_steps,
	                                                      &stepper->temp_allocator,
	                                                      &stepper->job_system);
	return static_cast<uint32_t>(error);
}

JoltBodyId jolt_body_create_dynamic(JoltWorld *world,
                                    const float position[3],
                                    float placeholder_radius)
{
	JPH::BodyCreationSettings settings(new JPH::SphereShape(placeholder_radius),
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

void jolt_body_set_sphere_shape(JoltWorld *world, JoltBodyId body, float radius)
{
	world->system.GetBodyInterface().SetShape(JPH::BodyID(body),
	                                          new JPH::SphereShape(radius),
	                                          /*inUpdateMassProperties=*/true,
	                                          JPH::EActivation::Activate);
}

void jolt_body_set_box_shape(JoltWorld *world,
                             JoltBodyId body,
                             const float half_extents[3])
{
	world->system.GetBodyInterface().SetShape(
		JPH::BodyID(body),
		new JPH::BoxShape(JPH::Vec3(half_extents[0], half_extents[1], half_extents[2])),
		/*inUpdateMassProperties=*/true,
		JPH::EActivation::Activate);
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

bool jolt_world_cast_ray(const JoltWorld *world,
                         const float origin[3],
                         const float direction[3],
                         JoltRayHit *out_hit)
{
	JPH::RRayCast ray(JPH::RVec3(origin[0], origin[1], origin[2]),
	                  JPH::Vec3(direction[0], direction[1], direction[2]));

	JPH::RayCastResult hit;
	if (!world->system.GetNarrowPhaseQuery().CastRay(ray, hit))
		return false;

	out_hit->body = hit.mBodyID.GetIndexAndSequenceNumber();
	out_hit->fraction = hit.mFraction;

	// The surface normal requires shape access, which needs the body lock
	// (the recommended pattern from NarrowPhaseQuery::CastRay's docs).
	JPH::BodyLockRead lock(world->system.GetBodyLockInterface(), hit.mBodyID);
	if (lock.Succeeded())
	{
		JPH::Vec3 normal = lock.GetBody().GetWorldSpaceSurfaceNormal(
			hit.mSubShapeID2, ray.GetPointOnRay(hit.mFraction));
		out_hit->normal[0] = normal.GetX();
		out_hit->normal[1] = normal.GetY();
		out_hit->normal[2] = normal.GetZ();
	}
	else
	{
		out_hit->normal[0] = 0.0f;
		out_hit->normal[1] = 0.0f;
		out_hit->normal[2] = 0.0f;
	}

	return true;
}

} // extern "C"
