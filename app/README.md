# App desktop

Prerequisiti: Node.js 24, Rust stabile con toolchain MSVC, strumenti di compilazione C++ di Visual Studio e WebView2 Runtime.

```powershell
npm ci
npm start             # frontend nel browser
npm run tauri -- dev  # app desktop
npm test -- --watch=false
npm run build
```

Dalla radice: `cargo test --workspace`, `cargo clippy --workspace --all-targets` e `cargo fmt --all -- --check`.
