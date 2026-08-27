# OpenDFM

A browser-based frontend and WebAssembly processing shell for checking Gerber fabrication files against PCB design-for-manufacturing rules.

## Development

```sh
npm install
npm run dev
```

Create and preview a production build:

```sh
npm run build
npm run preview
```

Run the Rust unit test, build the WebAssembly package, and execute its smoke test:

```sh
npm run test:wasm
```

Development requires Node.js, Rust with the `wasm32-unknown-unknown` target, and `wasm-pack`. The current Rust crate exposes an empty Gerber-analysis response schema; Gerber parsing, DFM rules, and SVG generation remain intentionally unimplemented.

The Node integration in `scripts/gerber-wasm.mjs` initializes the generated package from `wasm/pkg` and exposes helpers for byte sources and Gerber file paths.
