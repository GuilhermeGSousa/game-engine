#pragma once

/* Umbrella over the shim's C API, split by area into world.h, body.h, and
 * ray.h. The Rust declarations in src/lib.rs must mirror these headers
 * exactly. */

#include "body.h"
#include "ray.h"
#include "world.h"
