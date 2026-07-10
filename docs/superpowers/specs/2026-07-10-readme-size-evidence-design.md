# README size evidence design

## Goal

Make the reduction in pdfium-light's delivery footprint immediately legible to
people evaluating a static PDF library, without implying a general performance
claim or hiding the measurement conditions.

## Placement and copy structure

Add a compact `Why pdfium-light?` section directly below the README title.
It will lead with the measured static-library contrast, then state the
practical outcome for static PDF embedding and deployment:

- 16.39 MiB current static library versus 89.07 MiB before the light-runtime
  reduction (81.6% smaller, 72.68 MiB less).
- A smaller delivery footprint for applications that render, inspect, edit,
  and save static PDF documents.
- An explicit boundary: this is an artifact-size measurement, not a claim
  about speed, memory use, or suitability for interactive PDF workloads.

The section will link to a concise measurement note containing the exact GN
configuration, platform, commits compared, archive paths, and the documented
source/checkout deltas. The README will keep only the decision-relevant
numbers, avoiding a large metrics table before the getting-started material.

## Evidence and integrity

The comparison uses `libpdfium.a` built with the same static macOS arm64
release GN configuration (`is_debug=false`, `is_component_build=false`,
`pdf_is_complete_lib=true`, `use_remoteexec=false`,
`clang_use_chrome_plugins=false`, and `symbol_level=0`).

The note will record the baseline (`70054f26b`, 93,395,824 bytes / 89.07 MiB)
and current result (`89f04a751`, 17,187,760 bytes / 16.39 MiB). It will also
record the current checkout reduction relative to the documented Issue #1
snapshot (`31fae8b6`): 2,725 tracked files, 894 first-party source files,
192,633 LOC, and a 263M checkout.

The historical Issue #1 commit is not presented as a built-library benchmark:
it does not complete a repeat build because it contains stale references to
already removed form-fill code. This limitation is stated explicitly.

## Psychology and ethics

The headline uses contrast framing to turn an otherwise abstract artifact
size into an evaluable choice. The nearby measurement note provides the
authority signal: a reader can reproduce the result and assess its scope.
The copy avoids artificial urgency, vague superlatives, and any implication
that size alone predicts runtime performance.

## Verification

- Check Markdown rendering and links locally.
- Verify every displayed number against the recorded measurement output.
- Run `python3 testing/tools/validate_light.py` after the README and note are
  added.
