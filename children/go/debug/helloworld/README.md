# Helloworld WASM Module

This repository contains a minimal Go module designed for debugging within the `wasm-operator` framework.
It serves as a simple example to demonstrate launching a WebAssembly (WASM) module from a parent operator.

## Overview

The module prints a message to stdout and is intended for early experimentation.
It does not implement the expected interface for child modules in the `wasm-operator` framework.
Future examples will provide compliant modules.

## Building the WASM Module

Use TinyGo to compile the Go code into a WebAssembly module:

```sh
tinygo build --target wasip2 -o target/wasm/hello.wasm
```

This will produce a hello.wasm binary.

## Running the Module

You can run the compiled WASM module using Wasmtime:

```sh
wasmtime target/wasm/hello.wasm
```

## Purpose

This module serves as an early test to verify that a parent operator can successfully start and execute a WASM module.
While this example does not conform to the expected interface for interoperable modules, it lays the groundwork for further development.
