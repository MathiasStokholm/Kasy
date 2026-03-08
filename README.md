# Kasy – Floating Islands

A family game built with **Rust** and **[Bevy](https://bevyengine.org/)**.
Explore floating islands in an isometric world and zap things with your water gun!

---

## Controls

| Key / Button | Action |
|---|---|
| **W A S D** | Move (strafing-style – independent of aim direction) |
| **Mouse** | Aim / rotate character |
| **Left Click** | Fire water gun |

---

## Platforms

| Platform | How |
|---|---|
| Linux (desktop) | `cargo run` |
| Web (WebGL 2 / WebGPU) | GitHub Pages (auto-deployed from `main`) |

---

## Development

### Prerequisites

- [Rust](https://rustup.rs/) stable toolchain
- For web builds: the `wasm32-unknown-unknown` target and `wasm-bindgen-cli`

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked
```

### Run on desktop (Linux)

```bash
cargo run
```

### Build and run in a browser (local)

```bash
# 1. Compile to WASM
cargo build --release --target wasm32-unknown-unknown

# 2. Generate JS glue code
mkdir -p dist
wasm-bindgen \
  --out-dir dist \
  --target web \
  target/wasm32-unknown-unknown/release/kasy.wasm

# 3. Copy the HTML entry-point
cp index.html dist/

# 4. Serve locally (any static file server works)
python3 -m http.server --directory dist
# then open http://localhost:8000
```

---

## Deployment

Every push to `main` triggers the GitHub Actions workflow
(`.github/workflows/deploy.yml`) which builds the WASM bundle and deploys it
to **GitHub Pages** automatically.