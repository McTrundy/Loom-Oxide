# Native computation experiment

Loom Oxide investigates the cost of Minecraft's processing paths and replaces
individual computations only when their behavior is understood and a measured
benefit survives integration. Rust is a candidate implementation language, not
a requirement for every optimization.

## Starting point

- Upstream: https://github.com/AnxeloK/Loom
- Branch: `ver/26.3.x`
- Base commit: `5a2ef7237307c0da445d2aaacbc58e00a066b717`
- Minecraft: `26.3`
- Paper revision: `643a11a413c3f6afb19f0ada2db43c78ae3f2607`
- Java toolchain: `25`

This is a separate experimental development line. Changes stay in this fork;
upstream integration and production deployment require their own scope. The
initial commit changes documentation only. No native library, build integration
or runtime replacement has been implemented.

## Compatibility contract

Preserve world generation, random-number consumption, stored data, protocol,
Bukkit/Paper API behavior, event decisions, ordering and Loom owner domains.
Compatibility is a target to establish for each change, not an assertion that
finite tests can prove every possible behavior.

Keep Java implementations available as the reference and supported fallback.
Reject unsupported native versions, architectures and CPU features before use.
Preserve existing licenses and notices when translating existing algorithms.

## Selecting the first target

1. Trace generation, existing-terrain simulation and packet processing separately.
   Identify expensive leaf computations, their inputs, state owners and callers.
2. Capture a repeatable baseline on a pinned artifact, workload and machine.
   Separate profiling runs from acceptance measurements.
3. Prefer one self-contained computation over moving an entire subsystem. Batched
   noise or density calculations are candidates to investigate, not a selected
   first implementation. Establish their actual cost before choosing them.
4. Compare an improved Java implementation where appropriate. Native code must
   earn its boundary and maintenance cost.

## Java and native code

Evaluate Java 25's Foreign Function & Memory API with a small, versioned C ABI
exported by a Rust `cdylib`. JFFI and JNI remain alternatives if a concrete need
justifies them. Choose the binding from measured total cost and compatibility.

Batch substantial computation per call. Include input preparation, copies,
native transitions, output conversion and allocation in measurements. Avoid a
native call for every tiny scalar operation.

Java retains ownership of mutable world state, plugin events and lifecycle.
Pass bounded buffers with explicit layouts, lengths and lifetimes. Native code
must not retain borrowed pointers or mutate Java objects. Avoid introducing a
second scheduler; use Loom's existing owners and executors.

Validate the ABI, sizes, alignment and authority before side effects. Define
native allocation ownership, release, cancellation and shutdown. Convert
recoverable native failures to explicit results; never unwind a Rust panic
through the ABI. A process crash or memory corruption cannot be repaired by
switching to the Java fallback. Publish outputs only after successful completion.

## Evidence required for each replacement

- Differential tests against Java over recorded inputs, generated cases, boundary
  values, seeds, dimensions, coordinates and supported configurations.
- Exact output where Minecraft semantics require it. Preserve overflow, shifts,
  floating-point evaluation and random-number ordering. Do not introduce
  approximate math, reassociation or fused operations without proving equivalent
  behavior across supported targets.
- Integration checks for affected events, plugin callers, ownership, concurrent
  access, failure, cancellation and retirement. Check final chunk data when a
  generation computation changes.
- Equivalent before/after runs covering total CPU, latency tails, throughput,
  allocation/native memory and queues. Report generation separately from
  pregenerated gameplay. Distinguish kernel improvements from server improvements.
- Native-off behavior, unsupported-platform behavior and distribution/loading
  checks. Record the Java, Rust, compiler, CPU feature and artifact versions.

Keep experiments small and independently reversible. Document mismatches and
negative results. Retain a replacement only when its compatibility evidence and
integrated benefit justify its complexity.

## Source workflow

The inherited patch directories remain authoritative. Capture generated Java
source changes in the correct outer patch layers and verify replay. Follow
[the patch workflow](patch-and-release-workflow.md) and
[runtime ownership](runtime-ownership-model.md). Future native source belongs in
this repository, with pinned build inputs and no dependency on a private checkout.

Technical references:

- [Java 25 Foreign Function & Memory API](https://docs.oracle.com/en/java/javase/25/core/foreign-function-and-memory-api.html)
- [Rust foreign function interfaces](https://doc.rust-lang.org/nomicon/ffi.html)
- [JFFI](https://github.com/jnr/jffi)
