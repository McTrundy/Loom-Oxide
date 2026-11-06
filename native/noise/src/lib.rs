//! Scalar reconstruction of Minecraft 26.3's regular and smeared Perlin volumes.
//! Java retains noise initialization and ownership of every supplied buffer.

use std::mem::{align_of, size_of};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::slice;

const OK: i32 = 0;
const INVALID: i32 = 1;
const PANIC: i32 = 2;
const HALF_ROUND_OFF: f64 = f64::from_bits(0x416f_ffff_ffff_ffff);
const ROUND_OFF: f64 = 33_554_432.0;
const GRADIENTS: [[i8; 3]; 16] = [
    [1, 1, 0],
    [-1, 1, 0],
    [1, -1, 0],
    [-1, -1, 0],
    [1, 0, 1],
    [-1, 0, 1],
    [1, 0, -1],
    [-1, 0, -1],
    [0, 1, 1],
    [0, -1, 1],
    [0, 1, -1],
    [0, -1, -1],
    [1, 1, 0],
    [0, -1, 1],
    [-1, 1, 0],
    [0, -1, -1],
];

/// Initialized Java state, copied once for the owning noise stack.
#[repr(C)]
pub struct NoiseLayer {
    pub offset_x: f64,
    pub offset_y: f64,
    pub offset_z: f64,
    pub frequency: f64,
    pub fudge_y_scale: f64,
    pub amplitude: f32,
    /// Zero selects PerlinNoise; one selects SmearedPerlinNoise.
    pub kind: u32,
    pub permutation: [u8; 256],
}

#[repr(C)]
pub struct NoiseVolume {
    pub size_x: i32,
    pub size_y: i32,
    pub size_z: i32,
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub step_x: i32,
    pub step_y: i32,
    pub step_z: i32,
    /// Bit zero enables the reference coordinate wrapping.
    pub flags: u32,
    pub xz_scale: f64,
    pub y_scale: f64,
    pub amplitude: f32,
    pub reserved: u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn oxide_noise_abi_version() -> u32 {
    1
}

/// Layout sizes let a caller reject a mismatched native library before use.
#[unsafe(no_mangle)]
pub extern "C" fn oxide_noise_layout() -> u64 {
    ((size_of::<NoiseLayer>() as u64) << 32) | size_of::<NoiseVolume>() as u64
}

/// Adds ordered noise layers into the supplied output prefix.
///
/// An error makes the output unspecified; callers must stage output separately
/// and publish it only after OK. No supplied pointer is retained.
///
/// # Safety
/// Non-null pointers must identify live, correctly aligned allocations covering
/// their declared elements. Inputs must not overlap mutable output. The caller
/// must retain exclusive ownership of output for the whole call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxide_noise_volume_v1(
    layers: *const NoiseLayer,
    layer_count: u64,
    volume: *const NoiseVolume,
    output: *mut f32,
    output_count: u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if volume.is_null() || !(volume as usize).is_multiple_of(align_of::<NoiseVolume>()) {
            return INVALID;
        }
        // SAFETY: allocation validity is the documented caller contract.
        let volume = unsafe { &*volume };
        let Some(count) = volume_count(volume) else {
            return INVALID;
        };
        if output.is_null()
            || !(output as usize).is_multiple_of(align_of::<f32>())
            || output_count < count as u64
            || count > isize::MAX as usize / size_of::<f32>()
        {
            return INVALID;
        }
        let Ok(layer_count) = usize::try_from(layer_count) else {
            return INVALID;
        };
        if layer_count > isize::MAX as usize / size_of::<NoiseLayer>() {
            return INVALID;
        }
        if layer_count == 0 {
            return OK;
        }
        if layers.is_null() || !(layers as usize).is_multiple_of(align_of::<NoiseLayer>()) {
            return INVALID;
        }
        // SAFETY: bounds and alignment were checked; allocation extents and
        // disjointness remain the caller's documented responsibilities.
        let layers = unsafe { slice::from_raw_parts(layers, layer_count) };
        if layers.iter().any(|layer| layer.kind > 1) {
            return INVALID;
        }
        let output = unsafe { slice::from_raw_parts_mut(output, count) };
        for layer in layers {
            let xz_scale = volume.xz_scale * layer.frequency;
            let y_scale = volume.y_scale * layer.frequency;
            let amplitude = volume.amplitude * layer.amplitude;
            match layer.kind {
                0 => add_volume::<false>(layer, volume, output, xz_scale, y_scale, amplitude),
                1 => add_volume::<true>(layer, volume, output, xz_scale, y_scale, amplitude),
                _ => unreachable!(),
            }
        }
        OK
    }))
    .unwrap_or(PANIC)
}

fn volume_count(volume: &NoiseVolume) -> Option<usize> {
    if volume.size_x <= 0
        || volume.size_y <= 0
        || volume.size_z <= 0
        || volume.step_x <= 0
        || volume.step_y <= 0
        || volume.step_z <= 0
        || volume.flags & !1 != 0
        || volume.reserved != 0
    {
        return None;
    }
    (volume.size_x as usize)
        .checked_mul(volume.size_y as usize)?
        .checked_mul(volume.size_z as usize)
}

fn block(minimum: i32, index: i32, step: i32) -> i32 {
    minimum.wrapping_add(index.wrapping_mul(step))
}

fn wrap(value: f64, enabled: bool) -> f64 {
    if !enabled || (-HALF_ROUND_OFF..HALF_ROUND_OFF).contains(&value) {
        value
    } else {
        value - (value / ROUND_OFF + 0.5).floor() * ROUND_OFF
    }
}

fn permute(layer: &NoiseLayer, coordinate: i32) -> i32 {
    i32::from(layer.permutation[(coordinate & 255) as usize])
}

fn gradient(layer: &NoiseLayer, coordinate: i32) -> [i8; 3] {
    GRADIENTS[(permute(layer, coordinate) & 15) as usize]
}

fn fade(value: f32) -> f32 {
    value * value * value * (value * (value * 6.0 - 15.0) + 10.0)
}

fn lerp(alpha: f32, low: f32, high: f32) -> f32 {
    low + alpha * (high - low)
}

fn lerp2(x: f32, y: f32, corners: &[f32]) -> f32 {
    lerp(
        y,
        lerp(x, corners[0], corners[1]),
        lerp(x, corners[2], corners[3]),
    )
}

fn lerp3(x: f32, y: f32, z: f32, corners: &[f32; 8]) -> f32 {
    lerp(z, lerp2(x, y, &corners[..4]), lerp2(x, y, &corners[4..]))
}

fn fudge_y(original_y: f64, relative_y: f64, scale: f64) -> f64 {
    let limit = if original_y >= 0.0 && original_y < relative_y {
        original_y
    } else {
        relative_y
    };
    // The reference epsilon is a float literal widened to double.
    let cells = (limit / scale + f64::from(1.0e-7_f32)).floor() as i32;
    f64::from(cells) * scale
}

fn add_volume<const SMEARED: bool>(
    layer: &NoiseLayer,
    volume: &NoiseVolume,
    output: &mut [f32],
    xz_scale: f64,
    y_scale: f64,
    amplitude: f32,
) {
    let wrapping = volume.flags & 1 != 0;
    let mut horizontal = [0.0_f32; 8];
    let mut vertical = [0.0_f32; 8];
    let mut index = 0;
    for iz in 0..volume.size_z {
        let z = wrap(
            f64::from(block(volume.min_z, iz, volume.step_z)) * xz_scale,
            wrapping,
        ) + layer.offset_z;
        let floor_z = z.floor() as i32;
        let relative_z = (z - f64::from(floor_z)) as f32;
        let alpha_z = fade(relative_z);
        for ix in 0..volume.size_x {
            let x = wrap(
                f64::from(block(volume.min_x, ix, volume.step_x)) * xz_scale,
                wrapping,
            ) + layer.offset_x;
            let floor_x = x.floor() as i32;
            let relative_x = (x - f64::from(floor_x)) as f32;
            let x0 = permute(layer, floor_x);
            let x1 = permute(layer, floor_x.wrapping_add(1));
            let alpha_x = fade(relative_x);
            let mut last_floor_y = i32::MIN;
            for iy in 0..volume.size_y {
                let original_y = f64::from(block(volume.min_y, iy, volume.step_y)) * y_scale;
                let y = wrap(original_y, wrapping) + layer.offset_y;
                let floor_y = y.floor() as i32;
                let relative_y = y - f64::from(floor_y);
                let alpha_y = fade(relative_y as f32);
                if last_floor_y != floor_y {
                    let xy = [
                        permute(layer, x0.wrapping_add(floor_y)),
                        permute(layer, x1.wrapping_add(floor_y)),
                        permute(layer, x0.wrapping_add(floor_y).wrapping_add(1)),
                        permute(layer, x1.wrapping_add(floor_y).wrapping_add(1)),
                    ];
                    for corner in 0..8 {
                        let g = gradient(
                            layer,
                            xy[corner & 3]
                                .wrapping_add(floor_z)
                                .wrapping_add((corner >> 2) as i32),
                        );
                        let rx = if corner & 1 == 0 {
                            relative_x
                        } else {
                            relative_x - 1.0
                        };
                        let rz = if corner & 4 == 0 {
                            relative_z
                        } else {
                            relative_z - 1.0
                        };
                        horizontal[corner] = f32::from(g[0]) * rx + f32::from(g[2]) * rz;
                        vertical[corner] = f32::from(g[1]);
                    }
                    last_floor_y = floor_y;
                }
                let dot_y = if SMEARED {
                    (relative_y - fudge_y(original_y, relative_y, layer.fudge_y_scale)) as f32
                } else {
                    relative_y as f32
                };
                let mut corners = [0.0_f32; 8];
                for corner in 0..8 {
                    let ry = if corner & 2 == 0 { dot_y } else { dot_y - 1.0 };
                    corners[corner] = horizontal[corner] + vertical[corner] * ry;
                }
                output[index] += amplitude * lerp3(alpha_x, alpha_y, alpha_z, &corners);
                index += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume() -> NoiseVolume {
        NoiseVolume {
            size_x: 1,
            size_y: 1,
            size_z: 1,
            min_x: 0,
            min_y: 0,
            min_z: 0,
            step_x: 1,
            step_y: 1,
            step_z: 1,
            flags: 1,
            xz_scale: 1.0,
            y_scale: 1.0,
            amplitude: 1.0,
            reserved: 0,
        }
    }

    #[test]
    fn layouts_are_the_versioned_contract() {
        assert_eq!(oxide_noise_abi_version(), 1);
        assert_eq!(size_of::<NoiseLayer>(), 304);
        assert_eq!(align_of::<NoiseLayer>(), 8);
        assert_eq!(size_of::<NoiseVolume>(), 64);
        assert_eq!(oxide_noise_layout(), (304_u64 << 32) | 64);
    }

    #[test]
    fn empty_stack_preserves_existing_output() {
        let mut output = [7.5];
        let result = unsafe {
            oxide_noise_volume_v1(std::ptr::null(), 0, &volume(), output.as_mut_ptr(), 1)
        };
        assert_eq!(result, OK);
        assert_eq!(output, [7.5]);
    }

    #[test]
    fn malformed_volume_is_rejected_before_output_changes() {
        let mut v = volume();
        v.size_x = -1;
        let mut output = [7.5];
        assert_eq!(
            unsafe { oxide_noise_volume_v1(std::ptr::null(), 0, &v, output.as_mut_ptr(), 1) },
            INVALID
        );
        assert_eq!(output, [7.5]);
        v.size_x = 1;
        v.reserved = 1;
        assert_eq!(
            unsafe { oxide_noise_volume_v1(std::ptr::null(), 0, &v, output.as_mut_ptr(), 1) },
            INVALID
        );
        assert_eq!(output, [7.5]);
    }

    #[test]
    fn null_and_short_buffers_are_rejected() {
        let mut output = [7.5];
        assert_eq!(
            unsafe {
                oxide_noise_volume_v1(
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    output.as_mut_ptr(),
                    1,
                )
            },
            INVALID
        );
        assert_eq!(
            unsafe {
                oxide_noise_volume_v1(std::ptr::null(), 0, &volume(), output.as_mut_ptr(), 0)
            },
            INVALID
        );
        assert_eq!(
            unsafe {
                oxide_noise_volume_v1(std::ptr::null(), 1, &volume(), output.as_mut_ptr(), 1)
            },
            INVALID
        );
        assert_eq!(output, [7.5]);
    }

    #[test]
    fn integer_lattice_samples_preserve_existing_output() {
        let mut permutation = [0_u8; 256];
        for (index, value) in permutation.iter_mut().enumerate() {
            *value = index as u8;
        }
        for kind in [0, 1] {
            let layer = NoiseLayer {
                offset_x: 0.0,
                offset_y: 0.0,
                offset_z: 0.0,
                frequency: 1.0,
                fudge_y_scale: 0.5,
                amplitude: 1.0,
                kind,
                permutation,
            };
            let mut output = [7.5];
            assert_eq!(
                unsafe { oxide_noise_volume_v1(&layer, 1, &volume(), output.as_mut_ptr(), 1) },
                OK
            );
            assert_eq!(output, [7.5]);
        }
    }
}
