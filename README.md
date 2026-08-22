# Mytube

Local video library. Add folders, browse, watch. No ads, no algorithm, no network.

**Version:** 0.1.2  
**License:** [MIT](LICENSE)

## Install (Windows)

1. Build (needs Node 20+, Rust, WebView2 — already on recent Windows):

   ```powershell
   npm install
   npm run tauri build
   ```

2. Run the installer:

   `src-tauri\target\release\bundle\nsis\Mytube_0.1.2_x64-setup.exe`

   Or run the portable exe:

   `src-tauri\target\release\mytube.exe`

After install, launch **Mytube** from the Start menu.

## Dev

```powershell
npm install
npm run tauri dev
```

Rust unit tests:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```
