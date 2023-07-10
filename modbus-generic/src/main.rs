use busrt::rpc::Rpc;
use eva_common::prelude::*;
use eva_sdk::prelude::*;
use once_cell::sync::{Lazy, OnceCell};
use rmodbus::server::context::ModbusContextFull;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

const AUTHOR: &str = "Bohemia Automation";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = "SIM Virtual Modbus generic context";

static RPC: OnceCell<Arc<RpcClient>> = OnceCell::new();
static CONTEXT: Lazy<Mutex<ModbusContextFull>> = Lazy::new(<_>::default);
static DATA_FILE: OnceCell<PathBuf> = OnceCell::new();

#[cfg(not(feature = "std-alloc"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

err_logger!();

struct Handlers {
    info: ServiceInfo,
    unit: u8,
}

#[async_trait::async_trait]
impl RpcHandlers for Handlers {
    // Handle RPC call
    async fn handle_call(&self, event: RpcEvent) -> RpcResult {
        svc_rpc_need_ready!();
        let method = event.parse_method()?;
        let payload = event.payload();
        match method {
            "save" => {
                if payload.is_empty() {
                    save_context().await?;
                    Ok(None)
                } else {
                    Err(RpcError::params(None))
                }
            }
            _ => svc_handle_default_rpc(method, &self.info),
        }
    }
    async fn handle_frame(&self, frame: Frame) {
        eva_sim_modbus::process_modbus_frame::<10_000, 10_000, 10_000, 10_000>(
            frame,
            &mut *CONTEXT.lock().await,
            self.unit,
            RPC.get().unwrap().as_ref(),
        )
        .await;
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    port_svc: String,
    unit: u8,
    #[serde(default)]
    persistent: bool,
}

async fn save_context() -> EResult<()> {
    if let Some(data_file) = DATA_FILE.get() {
        let mut f = tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(data_file)
            .await?;
        let ctx = pack(&*CONTEXT.lock().await)?;
        f.write_all(&ctx).await?;
        f.sync_all().await?;
        info!("context saved");
        Ok(())
    } else {
        Err(Error::failed(
            "context not saved: running under a restricted user or no persistent configured",
        ))
    }
}

async fn load_context_data() -> Result<Option<Vec<u8>>, std::io::Error> {
    if let Some(data_file) = DATA_FILE.get() {
        let mut data = Vec::new();
        let mut f = tokio::fs::OpenOptions::new()
            .read(true)
            .open(data_file)
            .await?;
        f.read_to_end(&mut data).await?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

#[svc_main]
async fn main(mut initial: Initial) -> EResult<()> {
    let config: Config = Config::deserialize(
        initial
            .take_config()
            .ok_or_else(|| Error::invalid_data("config not specified"))?,
    )?;
    let mut info = ServiceInfo::new(AUTHOR, VERSION, DESCRIPTION);
    info.add_method(ServiceMethod::new("save"));
    let rpc = initial
        .init_rpc(Handlers {
            info,
            unit: config.unit,
        })
        .await?;
    initial.drop_privileges()?;
    let client = rpc.client().clone();
    RPC.set(rpc.clone())
        .map_err(|_| Error::core("Unable to set RPC"))?;
    eva_sim_modbus::init(&config.port_svc, &mut *client.lock().await).await?;
    svc_init_logs(&initial, client.clone())?;
    if config.persistent {
        if let Some(data_path) = initial.data_path() {
            let mut data_file = Path::new(data_path).to_owned();
            data_file.push("ctx.dat");
            DATA_FILE
                .set(data_file.clone())
                .map_err(|_| Error::core("Unable to set DATA_FILE"))?;
            match load_context_data().await {
                Ok(Some(data)) => {
                    info!("context loaded");
                    *CONTEXT.lock().await = unpack(&data)?;
                }
                Ok(None) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    info!("context not loaded (file not found), empty context created");
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }
    svc_start_signal_handlers();
    svc_mark_ready(&client).await?;
    info!("{} started ({})", DESCRIPTION, initial.id());
    svc_block(&rpc).await;
    svc_mark_terminating(&client).await?;
    if DATA_FILE.get().is_some() {
        save_context().await?;
    }
    Ok(())
}
