#ifndef LOOM_OXIDE_NOISE_H
#define LOOM_OXIDE_NOISE_H
#include <stdint.h>

typedef struct {
    double offset_x, offset_y, offset_z, frequency, fudge_y_scale;
    float amplitude;
    uint32_t kind;
    uint8_t permutation[256];
} OxideNoiseLayer;

typedef struct {
    int32_t size_x, size_y, size_z, min_x, min_y, min_z;
    int32_t step_x, step_y, step_z;
    uint32_t flags;
    double xz_scale, y_scale;
    float amplitude;
    uint32_t reserved;
} OxideNoiseVolume;

uint32_t oxide_noise_abi_version(void);
uint64_t oxide_noise_layout(void);

/* Borrow all pointers for this call only. Inputs and mutable output must be
 * live, aligned and disjoint. Return 0 means success, 1 invalid metadata,
 * 2 a caught panic. Discard staged output on any error. */
int32_t oxide_noise_volume_v1(const OxideNoiseLayer *layers, uint64_t layer_count,
    const OxideNoiseVolume *volume, float *output, uint64_t output_count);
#endif
