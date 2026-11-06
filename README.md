# Ferrum

Ferrum is an experimental fork of [Loom](https://github.com/AnxeloK/Loom)
for studying expensive Minecraft computations and evaluating compatible native
implementations, including Rust. Compatibility is a requirement; performance
improvements must be measured. No native implementation is included yet.

See the [experiment plan](docs/native-experiment.md) for scope, validation and
the proposed Java/native boundary. Development is led by McTrundy, with AnxeloK
credited as co-author on development commits. Existing contributor attribution
and licensing are preserved.

The instructions below describe the inherited Loom runtime. The download links
refer to upstream Loom releases.

## Upstream Loom

Loom is a multithreaded Minecraft server based on [Paper](https://papermc.io).

It runs world work in parallel while keeping mutable state inside the owner domain that is responsible for it. That lets Loom use more of the machine without treating unsafe cross-thread access as normal plugin behaviour.

## Download Loom

New Loom releases target one latest stable Minecraft line. The `mc` suffix states the exact Minecraft version required by each jar.

Download the jar and checksum from [Loom releases](https://github.com/AnxeloK/Loom/releases).

Historical releases through 2.0.9 retain their older multi-version assets. They remain downloadable, but only the repository's default version branch is maintained.

Loom requires Java 25 or newer. A basic launch command looks like this:

```bash
java -Xms4G -Xmx4G -XX:+UseG1GC -jar loom-X.Y.Z-mcX.Y.Z.jar nogui
```

Choose the heap size for the machine and player load. Leave enough RAM for the operating system, native memory, networking, and the server's direct buffers.

## What to expect

Ordinary Bukkit and Paper plugins should continue to use their normal APIs. Loom automatically selects the safe execution route for each operation. That can mean direct execution, a serialized plugin callback, an owner-domain handoff, or a refusal when the operation cannot be made safe.

The important boundary is simple: a thread may only mutate state that it owns. Loom does not weaken that rule to make an error disappear.

## First checks after startup

Once the server reaches `Done (...)`, check:

```text
/loom tps
/loom compatibility
```

Then exercise the real paths your server depends on: joins, teleports, commands, inventories, plugin menus, chunk movement, deaths, and dimension changes. Inspect the logs before accepting repeated owner-domain errors, async access errors, or refusals as normal.

## Documentation

- [Getting started](docs/getting-started.md) explains installation and a first smoke test.
- [Performance](docs/performance.md) explains how to profile and tune a real workload.
- [Compatibility diagnostics](docs/compatibility-kernel.md) explains `/loom compatibility`.
- [Runtime ownership](docs/runtime-ownership-model.md) explains the safety model for contributors.
- [Patch and release workflow](docs/patch-and-release-workflow.md) explains how the latest Minecraft line is maintained and published.
- [FAQ](docs/faq.md) gives short operational answers.

## Build from source

```bash
./patch.sh
./gradlew :loom-server:compileJava --no-daemon
./gradlew :loom-server:createPaperclipJar --no-daemon
```

Before pushing a runtime change, run the same verification used by GitHub:

```bash
./scripts/verify-ci.sh
```

This applies the patches, builds the pinned Paper ABI baseline, runs the full checks and packages the runtime.
Set `PAPER_ABI_BASELINE_DIR` to choose a different baseline workspace.

To run this check automatically before each push from the current checkout:

```bash
git config extensions.worktreeConfig true
git config --worktree core.hooksPath .githooks
```

## License

Loom is distributed under the GNU General Public License v3.0. See [LICENSE](LICENSE).
