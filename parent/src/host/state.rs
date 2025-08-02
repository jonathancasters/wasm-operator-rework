//! # Host State Module
//!
//! This module defines the `State` struct, which holds the necessary context and resources
//! for a WebAssembly (Wasm) component instance. It provides access to WASI (WebAssembly
//! System Interface) functionalities, the Kubernetes service, and a resource table for
//! managing host-defined resources, enabling Wasm modules to interact with the host
//! environment.

use std::sync::Arc;

use wasmtime::component::{HasData, ResourceTable};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiView};

use crate::kubernetes::KubernetesService;

pub struct State {
    pub wasi_ctx: WasiCtx,
    pub kubernetes_service: Arc<KubernetesService>,
    pub resources: ResourceTable,
}

impl WasiView for State {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl IoView for State {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resources
    }
}

impl HasData for State {
    type Data<'a> = ();
}
