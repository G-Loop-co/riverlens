# 2026-11-12 build, runtime and size baseline

## CI baseline

Evidence: [Desktop validation run 43069663200](https://github.com/G-Loop-co/riverlens/actions/runs/43069663200), commit `6609b40475068bf7d79430753836fc0bf06f22a7`.

| Platform | Queue wait | Job duration | Key finding |
|---|---:|---:|---|
| Windows x64 | 0s | 57m28s | Rust cache miss; DuckDB compiled 3 times; 100k audit 647.81s |
| macOS ARM64 | 66m13s | 34m16s | DuckDB compiled 3 times; 100k audit 396.42s |
| macOS Intel | 66m13s | 13m47s | Cross-build only; no native runtime acceptance |

Queue wait is release-run creation at 17:35:37Z minus desktop job start at 18:41:50Z. Windows job start and run creation were effectively simultaneous.

## Frontend

Local host: Apple M3 Pro, arm64, Node 22, Rust 1.99.1.

| Measure | Before | After | Median source |
|---|---:|---:|---|
| `npm run build` wall time | 11.24s | 7.69s | 3 after-runs: 7.69s, 7.33s, 7.86s |
| Main JS | 1131.31KB | 1036.13KB | Vite output |
| Main CSS | 103.10KB | 46.12KB | Workspace CSS moved with lazy chunks |

The change lazy-loads Learn, Research and AI Coach and imports only Latin Geist WOFF/WOFF2 faces. Chinese text uses the existing system font fallback.

## Release codegen units

Both variants used Thin LTO and `strip = "symbols"` in an isolated Cargo target directory. The resulting app binary was identical at 49,797,168 bytes, SHA-256 `a52cc7cf99eb9df6f9fa103bd915d988aae5329bf449828396cf2f7928643ea0`.

| Release build | Clean | Warm median | 10k aggregate p95 | 10k contextual p95 | Peak RSS |
|---|---:|---:|---:|---:|---:|
| CGU 1 | 5m24s | 64.72s | 8.729ms | 3.637ms | 261,963,776B |
| CGU 16 | 3m43s | 63.70s | 7.413ms | 3.495ms | 257,441,792B |

CGU 16 is retained: same size, faster clean build, and no runtime or memory regression. Each benchmark generated 10,000 synthetic GG hands and ran each query family 21 times.

## CI test profile

The new Cargo `ci` profile uses optimization level 1, line tables only, and no dependency debug info. It keeps the full 100k assertions.

The local ARM64 100k audit completed in 43.84s on first execution and 41.52s on the immediate warm rerun, versus 647.81s in the Windows CI baseline and 396.42s on macOS ARM64 CI. The comparison is evidence for the selected profile, not a cross-platform guarantee.

## Package

The local ARM64 build passed ad-hoc signing and deep archive signature verification.

| ARM64 macOS package | Bytes | Change |
|---|---:|---:|
| v0.6.1 release asset | 19,487,844 | baseline |
| Branch build `.app.zip` | 18,644,247 | -843,597B / -4.33% |

Platform CI remains the acceptance gate for Windows and Intel bundles.
