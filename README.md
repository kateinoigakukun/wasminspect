# Wasminspect: An Interactive Debugger for WebAssembly

Wasminspect is an interactive debugger for WebAssembly like lldb. It can be used for WebAssembly code and WASI applications also.

![Check](https://github.com/kateinoigakukun/wasminspect/workflows/Check/badge.svg)

![demo](./assets/demo.gif)

## [Tutorial](./docs/tutorial.md)

Let's try to debug your WebAssembly binary!

## Features

- Full WASI supports
- Breakpoints
- Process control
  - step-in, step-over and step-out
- Dump memory space
- Parse and evaluate DWARF debug information
- [more detail](./docs/tutorial.md)

## Swift Extension

wasminspect support some Swift specific features. To enable these features, please build on your machine because it requires swift runtime library.

On macOS:

```sh
$ cargo build --features swift-extension
```

On Linux:

```sh
$ export SWIFT_RUNTIME_LIB_DIR=/path/to/lib/swift/linux # e.g. $HOME/.swiftenv/versions/5.2-RELEASE/usr/lib/swift/linux
$ RUSTFLAGS="-C link-args=-Wl,-rpath,$SWIFT_RUNTIME_LIB_DIR" cargo build --features swift-extension
```
