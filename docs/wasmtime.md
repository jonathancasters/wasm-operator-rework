# Wasmtime TL;DR

This document serves as a concise summary of the most essential Wasmtime concepts used in this project.  
For full documentation, see [the official Wasmtime docs](https://docs.rs/wasmtime/32.0.0/wasmtime/index.html).

## Core Concepts

### Engine

- The **Engine** is the global compiler/runtime context for Wasm.
- All compiled modules or components belong to an Engine.
- Engines are cheap to clone and thread-safe.
- Typically, one Engine is created and shared per process.

### Store

- The **Store<T>** holds all instantiated WebAssembly objects like functions, memories, etc.
- It also stores per-instance host data (`T`), accessible via `Caller<'_, T>`.
- Stores are cheap to create and short-lived; drop the store to free all Wasm resources.

### Linker

- The **Linker** maps host functions to string names that Wasm modules/components can import.
- Populate once at startup, then reuse across instantiations.
- Host functions should not close over mutable state—use `Store<T>` for that.

### Component

- A **Component** is a compiled WebAssembly component (WIT + Wasm).
- Components are expensive to compile but cheap to share or serialize.
- Instantiate via `Component::from_file`, `Component::new`, etc.

### Instance

- An **Instance** is a live, instantiated component.
- This is where you get access to exported functions (`Func`, `TypedFunc`) and other wasm objects.

## WASI Traits: `WasiView` and `IoView`

To support WASI in Wasmtime, your per instance host data `T` must implement two key traits: `WasiView` and `IoView`. 
These traits provide access to the internal WASI state (`WasiCtx`) and the resource management system (`ResourceTable`) needed for running WASI modules.

### `IoView`

- Provides access to the **`ResourceTable`**, which holds host-side resources like files and sockets.
- You must implement `IoView` on the context type `T` by returning a mutable reference to the `ResourceTable`.

```rust
impl IoView for T {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
```

### `WasiView`

- Provides access to the **`WasiCtx`**, which contains the WASI environment: args, env vars, stdio, etc.
- `WasiView` **requires** `IoView`, so type `T` must implement both
- You imlement `WasiView` similarly by returning a mutable reference to the `WasiCtx`.

```rust
impl WasiView for T {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}
```

### Usage

- You define your own struct  (e.g. `MyCtx`) that includes both `ResourceTable` and `WasiCtx`.
- This struct becomes `T` inside your `Store<T>`
- You can then call `wasmtime_wasi::add_to_linker_async(&mut linker)`, which requires `T: WasiView`

```rust
use wasmtime::{Config, Engine};
use wasmtime::component::{ResourceTable, Linker};
use wasmtime_wasi_io::{IoView, add_to_linker_async};

struct MyCtx {
    table: ResourceTable,
    wasi: WasiCtx,
}

impl IoView for MyState {
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
}

impl WasiView for MyState {
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.wasi }
}

let mut config = Config::new();
config.async_support(true);
let engine = Engine::new(&config).unwrap();
let mut linker: Linker<MyCtx> = Linker::new(&engine);
add_to_linker_async(&mut linker).unwrap();
```
