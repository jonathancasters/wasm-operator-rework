use wasmtime::{
    component::{Component, Linker, ResourceTable},
    Engine, Store,
};
use wasmtime_wasi::{
    bindings::Command, WasiCtx, WasiCtxBuilder, WasiView, IoView, add_to_linker_async,
};
use tracing::{info, error};
use crate::config::metadata::WasmComponentMetadata;

pub struct ComponentCtx {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl IoView for ComponentCtx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

impl WasiView for ComponentCtx {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

pub async fn run_component(engine: Engine, metadata: WasmComponentMetadata) -> anyhow::Result<()> {
    info!("Start component: {}", metadata.name);

    let component = Component::from_file(&engine, &metadata.wasm)
        .map_err(|e| anyhow::anyhow!("Failed to load component '{}': {}", metadata.name, e))?;

    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .args(&metadata.args)
        .envs(
            &metadata
                .env
                .iter()
                .map(|e| (e.name.as_str(), e.value.as_str()))
                .collect::<Vec<_>>(),
        )
        .build();

    let state = ComponentCtx {
        wasi_ctx,
        resource_table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);

    let mut linker = Linker::new(&engine);
    add_to_linker_async(&mut linker)?;

    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    let result = command.wasi_cli_run().call_run(&mut store).await;

    match result {
        Ok(Ok(())) => {
            info!("Module '{}' exited successfully", metadata.name);
            Ok(())
        }
        Ok(Err(())) => {
            error!("Module '{}' exited with failure", metadata.name);
            std::process::exit(1);
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed running command interface for '{}': {e}",
            metadata.name
        )),
    }
}
