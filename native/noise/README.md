# Experimental noise backend

This module reconstructs Minecraft 26.3's regular and smeared Perlin volume
evaluation in scalar Rust. It keeps the existing algorithm, layer order,
coordinate wrapping, float arithmetic and lattice-cell reuse. Java initializes
the seeded permutations and offsets. Scalar point sampling and other generation
stages continue to use Java.

The runtime defaults to Java. The native paths are experimental and require a
separately built library. No performance improvement is assumed.

## Build and check

With the pinned Rust toolchain installed:

```sh
cd native/noise
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

The Dockerfile provides the same compiler, formatter and checker in a container.
Run the normal Loom patch/build verification, then check both bindings against
the retained Java implementations:

```sh
./scripts/verify-ci.sh
./scripts/verify-native-noise.sh /absolute/path/to/libloom_oxide_noise.so
```

The focused suite is enabled only when its native backend and library are
explicitly supplied. It covers initialized layers, full-height volumes, negative
and large coordinates, wrapping and integer boundaries, float amplitudes,
existing buffer contents, concurrent callers and Java fallback paths. Rust tests
cover the ABI layout and malformed metadata.

## Select a backend

Pass these JVM options before `-jar`:

```text
--enable-native-access=ALL-UNNAMED
-Dloom.oxide.noiseBackend=ffm
-Dloom.oxide.nativeLibrary=/absolute/path/to/libloom_oxide_noise.so
```

Use `jffi` for the other binding or `java` for the original implementation. The
choice is fixed at process startup. An explicitly selected native backend fails
startup if its library cannot load or its ABI does not match. Native evaluation
is limited to the existing exact Perlin/smeared stack types; custom classes,
mixed stacks and unsupported nonfinite parameters retain Java evaluation.

The current native ABI is validated on Linux x86-64. Other platforms require
their own build and compatibility validation before native use.

## Boundary and lifetime

The C ABI is declared in [oxide_noise.h](include/oxide_noise.h). Inputs are
initialized, immutable layer records and a volume descriptor. Output is an
in/out float buffer. Layer state belongs to its Java noise stack; its automatic
arena becomes reclaimable with that stack. Per-call scratch memory is closed
after the synchronous call. Native code retains no pointers and calls no Java
or Bukkit APIs. JFFI calls fence the layer segment's reachability while raw
addresses are in use.

The adapter copies existing output into native scratch memory, invokes the
kernel and copies results back only after success. A reported native failure
cannot publish a partial buffer. This does not make a native process crash or
memory corruption recoverable. Input preparation, allocations and both copies
must be included in performance comparisons.

Regular and smeared volumes have different Y calculations. The smeared path
retains the original double calculation and widened float epsilon. This is a
version-specific reconstruction, not a substitute noise algorithm or an
approximate terrain generator. The repository's GPLv3 license applies.
