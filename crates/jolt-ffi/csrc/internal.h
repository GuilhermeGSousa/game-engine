#pragma once

// C++-internal definitions shared by the shim's translation units; not part
// of the C API. Every translation unit (Jolt + shim) must be compiled with
// the same JPH_* configuration defines; build.rs is the single source of
// truth for those.

#include <Jolt/Jolt.h>

#include <Jolt/Core/JobSystemThreadPool.h>
#include <Jolt/Core/TempAllocator.h>
#include <Jolt/Physics/Body/BodyCreationSettings.h>
#include <Jolt/Physics/PhysicsSystem.h>

JPH_SUPPRESS_WARNINGS

// Object layers: statics only collide with dynamics; dynamics collide with
// everything.
namespace Layers
{
	constexpr JPH::ObjectLayer NON_MOVING = 0;
	constexpr JPH::ObjectLayer MOVING = 1;
	constexpr JPH::uint NUM_LAYERS = 2;
} // namespace Layers

namespace BroadPhaseLayers
{
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

// The layer interfaces are declared before the system: PhysicsSystem::Init
// stores references to them, so they must be destroyed after it (members are
// destroyed in reverse declaration order).
struct JoltWorld
{
	BPLayerInterfaceImpl broad_phase_layer_interface;
	ObjectVsBroadPhaseLayerFilterImpl object_vs_broad_phase_layer_filter;
	ObjectLayerPairFilterImpl object_layer_pair_filter;
	JPH::PhysicsSystem system;
};

struct JoltBodyCreationSettings
{
	JPH::BodyCreationSettings settings;
};

struct JoltStepper
{
	JPH::TempAllocatorImpl temp_allocator;
	JPH::JobSystemThreadPool job_system;

	explicit JoltStepper(uint32_t temp_allocator_bytes)
		: temp_allocator(temp_allocator_bytes),
		  // -1 worker threads = hardware concurrency - 1.
		  job_system(JPH::cMaxPhysicsJobs, JPH::cMaxPhysicsBarriers, -1)
	{
	}
};
