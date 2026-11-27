# HotWire Release Checklist

## Version policy

- `hotwire-core` uses semver tags (`v0.6.0`).
- Apps pin an exact core tag in `Cargo.toml` and commit `Cargo.lock`.
- Firmware versions use `major.minor.patch[-tool[-chip]]` and must be >=
  `ToolId::min_firmware_version()`.
- Bump firmware patch for protocol-compatible changes, minor for new commands,
  major for breaking wire changes.

## 1. Core release

```bash
cd hotwire-core
cargo fmt --all
cargo test
cargo test --no-default-features
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
node --check js/web_serial_bridge.js
git add -A && git commit -m "feat(core): ..."
git push origin main
git tag vX.Y.Z && git push origin vX.Y.Z
```

## 2. App dependency bump

```bash
python .github-org/scripts/bump_core.py --version vX.Y.Z
```

Then in each app:

```bash
cargo fmt --all
cargo test
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
python scripts/verify_firmware.py
trunk build --release
```

## 3. Firmware

- Update `HOTWIRE_FW_VERSION` in every sketch.
- Update `ToolId::min_firmware_version()` and READMEs/site labels.
- Rebuild with `arduino-cli` (see below), copy the `.ino.bin`,
  `.bootloader.bin` and `.partitions.bin` into `public/firmware/`.
- Regenerate `firmware/manifest.json`.
- Verify the embedded version string.

### Build commands

```powershell
# XIAO ESP32-S3
arduino-cli compile --fqbn "esp32:esp32:XIAO_ESP32S3:USBMode=hwcdc,CDCOnBoot=default" --output-dir firmware/<Sketch>/build firmware/<Sketch>

# XIAO ESP32-S3 with PSRAM (Scope/SDR)
arduino-cli compile --fqbn "esp32:esp32:XIAO_ESP32S3:USBMode=hwcdc,CDCOnBoot=default,PSRAM=opi" --output-dir firmware/<Sketch>/build firmware/<Sketch>

# XIAO ESP32-C6
arduino-cli compile --fqbn "esp32:esp32:XIAO_ESP32C6:CDCOnBoot=cdc" --output-dir firmware/<Sketch>/build firmware/<Sketch>
```


> **wasm-opt note:** HotWire apps set `data-wasm-opt="0"` because the Trunk
> bundled binaryen 131 cannot validate Rust 1.100 bulk-memory output. Re-enable
> wasm-opt only after verifying a newer binaryen release against the CI Rust
> toolchain.
## 4. Cross-repo verification

```bash
python .github-org/scripts/verify_suite.py .
```

## 5. Push order

1. `hotwire-core` + tag
2. Apps (all five)
3. `.github-org` (workflows/scripts)
4. `hotwire-lab.github.io`
5. Verify GitHub Actions deploys for every repo.
