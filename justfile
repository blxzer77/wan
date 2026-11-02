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

supply-chain:
    cargo deny check
    pnpm audit --registry=https://registry.npmjs.org/ --audit-level high

licenses:
    tmp="$$(mktemp)" && trap 'rm -f "$$tmp"' EXIT && cargo about generate --workspace --frozen --fail --config about.toml --output-file NOTICE about.hbs && pnpm exec license-checker-rseidelsohn --failOn 'GPL;AGPL' --json --out "$$tmp" && python3 scripts/generate-third-party-licenses.py "$$tmp" manual.toml THIRD_PARTY_LICENSES.html

licenses-check:
    just licenses
    git diff --exit-code -- NOTICE THIRD_PARTY_LICENSES.html

sbom:
    cargo cyclonedx --manifest-path Cargo.toml --format json --spec-version 1.5 --override-filename sbom

vendor-check:
    python3 scripts/check-vendor-hashes.py

errors:
    python3 scripts/generate-errors.py

errors-check:
    just errors
    python3 scripts/check-errors.py
    git diff --exit-code HEAD -- docs/errors.md

ci: fmt lint test deps-check debug-port-check supply-chain licenses-check sbom vendor-check errors-check
    pnpm exec tsc --noEmit
