# pdfium-light Platform Support

pdfium-light targets a smaller platform set than upstream PDFium. The goal is to
keep the static PDF library portable across the platforms that matter for server
and desktop embedding, while avoiding maintenance for legacy or interactive
viewer-oriented configurations.

## Supported matrix

| Tier | Platform | CPU / ABI | Status | Validation expectation |
| --- | --- | --- | --- | --- |
| Primary | Linux glibc | x64 | Supported | Build `pdfium` and retained light validation targets. |
| Primary | Linux glibc | arm64 | Supported | Build `pdfium` and retained light validation targets. |
| Primary | Linux musl | x64 or arm64 where the Chromium toolchain supports it | Supported | Build `pdfium`; document any toolchain-specific linker/sysroot notes. |
| Primary | macOS | arm64 | Supported | Build `pdfium` and retained light validation targets. |
| Primary | Windows | x64 | Supported | Build `pdfium` and retained light validation targets with the Chromium Windows toolchain. |
| Probe | Windows | arm64 | Probe target | Keep Windows code that is shared with x64. Record a build probe result before promoting or rejecting this target. |

## Out of scope

The following targets are not supported by pdfium-light and should not be
advertised in public docs:

- macOS x64 / Intel;
- Windows x86 / 32-bit;
- Android, iOS, ChromeOS, and other mobile/browser-integrated targets;
- big-endian architectures;
- legacy MSVC builds.

Out of scope does not mean every conditional branch disappears immediately. Code
that is shared with retained targets should stay. For example, `is_win` branches
are still required for Windows x64 and the Windows arm64 probe. Only code,
configuration, docs, or test assets that are separable and specific to unsupported
platforms should be removed.

## Windows arm64 probe

Current result: blocked in this checkout, not proven unsupported.

Attempted command:

```sh
gn gen out/win-arm64-probe --args='target_os="win" target_cpu="arm64" pdf_enable_light=true pdf_enable_v8=false pdf_enable_xfa=false is_component_build=false clang_use_chrome_plugins=false'
```

Observed result in the repository worktree on 2026-07-09:

```text
zsh:1: command not found: gn
```

This checkout also lacks the full Chromium `build/` metadata used by GN, such as
`build/dotfile_settings.gni`. A meaningful Windows arm64 result therefore needs
a full gclient/depot_tools environment or a CI job that provides the Chromium
GN/Ninja toolchain and Windows SDK. Until that probe exists, Windows arm64 stays
as a probe target and Windows-specific source code must not be removed merely
because it is Windows-specific.

## Maintenance rules

- Public docs should list only the supported matrix above and the Windows arm64
  probe.
- New validation gates should use this matrix as their source of truth.
- Platform-specific removals need an audit-log entry when they delete files,
  build settings, or documented support.
- Do not remove Windows rendering, file, compiler, or manifest code unless it is
  demonstrably Windows x86-only and not needed by Windows x64 or arm64.
