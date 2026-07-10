# PDFium

## Prerequisites

PDFium uses the same build tooling as Chromium. See the platform-specific
Chromium build instructions to get started, but replace Chromium's
"Get the code" instructions with [PDFium's](#get-the-code).

*   [Chromium Linux build instructions](https://chromium.googlesource.com/chromium/src/+/main/docs/linux/build_instructions.md)
*   [Chromium Mac build instructions](https://chromium.googlesource.com/chromium/src/+/main/docs/mac_build_instructions.md)
*   [Chromium Windows build instructions](https://chromium.googlesource.com/chromium/src/+/main/docs/windows_build_instructions.md)

### Platforms supported

pdfium-light supports a deliberately smaller platform matrix than upstream
PDFium:

* Linux x64 glibc;
* Linux arm64 glibc;
* Linux musl where the Chromium toolchain supports the target ABI;
* macOS arm64;
* Windows x64.

Windows arm64 is a probe target. It should not be treated as unsupported until a
full Chromium/PDFium toolchain probe has been recorded.

macOS x64 / Intel and Windows x86 / 32-bit are out of scope and should not be
advertised as supported targets. Mobile/browser-integrated targets, big-endian
architectures, and legacy MSVC builds are also out of scope.

See [Platform Support](docs/platform-support.md) for the detailed matrix and the
current Windows arm64 probe status.

### Compilers supported

PDFium aims to be compliant with the [Chromium policy](https://chromium.googlesource.com/chromium/src/+/main/docs/toolchain_support.md#existing-toolchain-support).

Currently this means Clang. Former MSVC users should consider using clang-cl
if needed. Community-contributed patches for gcc will be allowed. No MSVC
patches will be taken.

#### Google employees

Run: `download_from_google_storage --config` and follow the
authentication instructions. **Note that you must authenticate with your
@google.com credentials**. Enter "0" if asked for a project-id.

Once you've done this, the toolchain will be installed automatically for
you in the [Generate the build files](#generate-the-build-files) step below.

The toolchain will be in `depot_tools\win_toolchain\vs_files\<hash>`, and
windbg can be found in
`depot_tools\win_toolchain\vs_files\<hash>\win_sdk\Debuggers`.

If you want the IDE for debugging and editing, you will need to install
it separately, but this is optional and not needed for building PDFium.

## Get the code

The name of the top-level directory does not matter. In the following example,
the directory name is "repo". This directory must not have been used before by
`gclient config` as each directory can only house a single gclient
configuration.

```
mkdir repo
cd repo
gclient config --unmanaged https://pdfium.googlesource.com/pdfium.git
gclient sync
cd pdfium
```

On Linux, additional build dependencies need to be installed by running the
following from the `pdfium` directory.

```
./build/install-build-deps.sh
```

## Generate the build files

PDFium uses GN to generate the build files and [Ninja](https://ninja-build.org/)
to execute the build files.  Both of these are included with the
depot\_tools checkout.

### Selecting build configuration

pdfium-light does not include JavaScript or XFA form support. Its supported
configuration is a static PDF render, inspect, edit, and save API.

Configuration is done by executing `gn args <directory>` to configure the build.
This will launch an editor in which you can set the following arguments.
By convention, `<directory>` should be named `out/foo`, and some tools / test
support code only works if one follows this convention.
A typical `<directory>` name is `out/Debug`.

```
use_remoteexec = false # Approved users only.  Do necessary setup & authentication first.
is_debug = true  # Enable debugging features.

pdf_enable_light = true  # Default. Exposes only the supported static API.
pdf_enable_xfa = false   # XFA has been removed.
pdf_enable_v8 = false    # JavaScript execution has been removed.
is_component_build = false # Disable component build (Though it should work)
```


By default, the entire project builds with C++20.

By default, PDFium expects to build with a clang compiler that provides
additional chrome plugins. To build against a vanilla one lacking these,
one must set
`clang_use_chrome_plugins = false`.

When complete the arguments will be stored in `<directory>/args.gn`, and
GN will automatically use the new arguments to generate build files.
Should your files fail to generate, please double-check that you have set
use\_sysroot as indicated above.

## Building the code

Build the library with:
`ninja -C <directory> pdfium`

Build the retained light validation targets with:
`ninja -C <directory> pdfium_light_validation`


## Testing

The repeatable pdfium-light validation gate is documented in
[Validation](docs/validation.md). In this reduced checkout, run:

```bash
python3 testing/tools/validate_light.py
```

The retained light validation targets are:

 * pdfium\_light\_public\_headers\_test
 * pdfium\_unittests
 * pdfium\_embeddertests

Run them through `pdfium_light_validation` or individually from the configured
Ninja output directory. Use `pdfium_all` when you also want the broader retained
diff/fuzzer set. The legacy `pdfium_test` executable and the corpus,
JavaScript, and pixel runners that depend on it are not part of pdfium-light.

### `.in` files

`.in` files are PDF template files. PDF files contain many byte offsets that
have to be kept correct or the file won't be valid. The template makes this
easier by replacing the byte offsets with certain keywords.

This saves space and also allows an easy way to reduce the test case to the
essentials as you can simply remove everything that is not necessary.

A simple example can be found [here](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/testing/resources/rectangles.in).

To transform this into a PDF, you can use the `fixup_pdf_template.py` tool:

```bash
$ ./testing/tools/fixup_pdf_template.py your_file.in
```

This will create a `your_file.pdf` in the same directory as `your_file.in`.

There is no official style guide for the .in file, but a consistent style is
preferred simply to help with readability. If possible, object numbers should
be consecutive and `/Type` and `/SubType` should be on top of a dictionary to
make object identification easier.

## Embedding PDFium in your own projects

The public/ directory contains header files for the APIs available for use by
embedders of PDFium. The PDFium project endeavors to keep these as stable as
possible.

Outside of the public/ directory, code may change at any time, and embedders
should not directly call these routines.

## Code Coverage

Code coverage reports for PDFium can be generated in Linux development
environments. Details can be found [here](/docs/code-coverage.md).

Chromium provides code coverage reports for PDFium
[here](https://chromium-coverage.appspot.com/). PDFium is located in
`third_party/pdfium` in Chromium's source code.
This includes code coverage from PDFium's fuzzers.

## Waterfall

The current health of the source tree can be found
[here](https://ci.chromium.org/p/pdfium/g/main/console).

## Community

There are several mailing lists that are setup:

 * [PDFium](https://groups.google.com/forum/#!forum/pdfium)
 * [PDFium Reviews](https://groups.google.com/forum/#!forum/pdfium-reviews)
 * [PDFium Bugs](https://groups.google.com/forum/#!forum/pdfium-bugs)

Note, the Reviews and Bugs lists are typically read-only.

## Bugs

PDFium uses this [bug tracker](https://crbug.com/pdfium/new).

Report security bugs via [Google Bughunters (Chrome VRP)](https://bughunters.google.com/report/vrp).

Project members and embedders can directly report security bugs using
[Chromium's security bug template](https://crbug.com/new?component=1586257&noWizard=True&template=1922342).

## Contributing code

See the [CONTRIBUTING](CONTRIBUTING.md) document for more information on
contributing to the PDFium project.
