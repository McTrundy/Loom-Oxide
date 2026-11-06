#!/usr/bin/env bash
set -euo pipefail

oxide_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$oxide_root"
oxide_library="${1:-$oxide_root/native/noise/target/release/libloom_oxide_noise.so}"
test -f "$oxide_library"
oxide_library="$(realpath -- "$oxide_library")"

for oxide_backend in ffm jffi; do
    ./gradlew :loom-server:test --init-script scripts/native-noise-tests.init.gradle \
        -Doxide.test.backend="$oxide_backend" -Doxide.test.library="$oxide_library" \
        --no-daemon --max-workers="${LOOM_BUILD_WORKERS:-2}"
done
