# pdfium-light Size Measurements

This note records the size evidence summarized in the README. It is intended
to make the comparison reproducible and to keep its limits explicit.

## Static library artifact

Both archives were built on macOS arm64 as complete, non-component static
libraries using the same GN arguments:

```gn
is_debug = false
is_component_build = false
pdf_is_complete_lib = true
use_remoteexec = false
clang_use_chrome_plugins = false
symbol_level = 0
```

| Revision | `libpdfium.a` bytes | Size | Difference from baseline |
| --- | ---: | ---: | ---: |
| Pre-pruning baseline `70054f26b` | 93,395,824 | 89.07 MiB | — |
| Current light runtime `89f04a751` | 17,187,760 | 16.39 MiB | -72.68 MiB (-81.6%) |

The library target was `pdfium`; the measured artifact was
`out/<name>/obj/libpdfium.a`. These figures describe the archive size only.
They do not measure runtime performance, peak memory, generated executable
size after linker dead stripping, or feature equivalence with full upstream
PDFium.

## Repository footprint

The first pdfium-light reduction snapshot is `31fae8b6`. Its documented source
tree measurements are compared with the current checkout below. The source
set includes tracked `.c`, `.cc`, `.cpp`, `.h`, `.gn`, `.gni`, and `.md` files
under the first-party directories; generated font data and CMaps are excluded.

| Metric | Issue #1 snapshot `31fae8b6` | Current `89f04a751` | Change |
| --- | ---: | ---: | ---: |
| Tracked files | 3,470 | 2,725 | -745 (-21.5%) |
| First-party source files | 1,008 | 894 | -114 (-11.3%) |
| First-party LOC | 215,187 | 192,633 | -22,554 (-10.5%) |
| Checkout size (`du -sh`) | 269M | 263M | -6M |

`du -sh` is intentionally shown in its filesystem-reported units, so the
checkout-size change is coarse rather than a byte-precise artifact metric.

## Historical-build limitation

`31fae8b6` is useful for the documented tree comparison, but it does not
complete the repeat static-library build under the configuration above: it
still contains stale references to already removed form-fill headers and
helpers. For that reason, this document does not present an invented archive
size for that commit. The archive comparison is instead between the original
pre-pruning baseline and the current fully built light runtime.
