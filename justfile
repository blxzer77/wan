set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo nextest run --workspace --no-tests=pass

deps-check:
    python3 scripts/check-cargo-dependencies.py

debug-port-check:
    python3 scripts/check-no-debug-port.py

ci: fmt lint test deps-check debug-port-check
    pnpm exec tsc --noEmit
