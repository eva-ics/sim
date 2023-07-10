use eva_common::prelude::*;
use eva_sdk::bitman::BitMan;
use eva_sdk::prelude::*;
use once_cell::sync::{Lazy, OnceCell};
use rmodbus::server::context::ModbusContext;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const AUTHOR: &str = "Bohemia Automation";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = "SIM Virtual Modbus relay";

static RPC: OnceCell<Arc<RpcClient>> = OnceCell::new();
static CONTEXT: Lazy<Mutex<ModbusContext<8, 0, 0, 1>>> = Lazy::new(<_>::default);

#[cfg(not(feature = "std-alloc"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

err_logger!();

struct Handlers {
    info: ServiceInfo,
    unit: u8,
    reg: Reg,
    output_type: OutputType,
}

#[async_trait::async_trait]
impl RpcHandlers for Handlers {
    // Handle RPC call
    async fn handle_call(&self, event: RpcEvent) -> RpcResult {
        svc_rpc_need_ready!();
        let method = event.parse_method()?;
        let payload = event.payload();
        #[allow(clippy::match_single_binding)]
        match method {
            "get" =>
            {
                #[allow(clippy::cast_possible_wrap)]
                if payload.is_empty() {
                    let ctx = CONTEXT.lock().await;
                    let mut result: BTreeMap<String, Value> = BTreeMap::new();
                    let mut data = Vec::with_capacity(8);
                    match self.reg {
                        Reg::Holding => {
                            let val = ctx.get_holding(0).unwrap();
                            for i in 0..8 {
                                data.push(val.get_bit(i));
                            }
                        }
                        Reg::Coil => {
                            ctx.get_coils_bulk(0, 8, &mut data).unwrap();
                        }
                    }
                    for (port, val) in data.into_iter().enumerate() {
                        let value = match self.output_type {
                            OutputType::Boolean => Value::Bool(val),
                            OutputType::Number => Value::U8(u8::from(val)),
                        };
                        result.insert(format!("port{}", port + 1), value);
                    }
                    Ok(Some(pack(&result)?))
                } else {
                    Err(RpcError::params(None))
                }
            }
            _ => svc_handle_default_rpc(method, &self.info),
        }
    }
    async fn handle_frame(&self, frame: Frame) {
        eva_sim_modbus::process_modbus_frame::<8, 0, 0, 1>(
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
    reg: Reg,
    #[serde(default)]
    output_type: OutputType,
}

#[derive(Deserialize, Default, Copy, Clone)]
#[serde(rename_all = "lowercase")]
enum Reg {
    #[default]
    #[serde(alias = "h")]
    Holding,
    #[serde(alias = "c")]
    Coil,
}

#[derive(Deserialize, Default, Copy, Clone)]
#[serde(rename_all = "lowercase")]
enum OutputType {
    #[default]
    Boolean,
    Number,
}

#[svc_main]
async fn main(mut initial: Initial) -> EResult<()> {
    let config: Config = Config::deserialize(
        initial
            .take_config()
            .ok_or_else(|| Error::invalid_data("config not specified"))?,
    )?;
    let mut info = ServiceInfo::new(AUTHOR, VERSION, DESCRIPTION);
    info.add_method(ServiceMethod::new("get"));
    let rpc = initial
        .init_rpc(Handlers {
            info,
            unit: config.unit,
            reg: config.reg,
            output_type: config.output_type,
        })
        .await?;
    initial.drop_privileges()?;
    let client = rpc.client().clone();
    RPC.set(rpc.clone())
        .map_err(|_| Error::core("Unable to set RPC"))?;
    eva_sim_modbus::init(&config.port_svc, &mut *client.lock().await).await?;
    svc_init_logs(&initial, client.clone())?;
    svc_start_signal_handlers();
    svc_mark_ready(&client).await?;
    info!("{} started ({})", DESCRIPTION, initial.id());
    svc_block(&rpc).await;
    svc_mark_terminating(&client).await?;
    Ok(())
}
