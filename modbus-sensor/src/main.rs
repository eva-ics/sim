use eva_common::prelude::*;
use eva_sdk::prelude::*;
use once_cell::sync::{Lazy, OnceCell};
use rmodbus::server::context::ModbusContext;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

const AUTHOR: &str = "Bohemia Automation";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = "SIM Virtual Modbus sensor";

static RPC: OnceCell<Arc<RpcClient>> = OnceCell::new();
static CONTEXT: Lazy<Mutex<ModbusContext<0, 0, 4, 4>>> = Lazy::new(<_>::default);

#[cfg(not(feature = "std-alloc"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

err_logger!();

trait SwapModbusEndianess {
    fn to_swapped_modbus_endianness(&self) -> Self;
}

impl SwapModbusEndianess for f32 {
    fn to_swapped_modbus_endianness(&self) -> Self {
        let b = self.to_be_bytes();
        Self::from_be_bytes([b[2], b[3], b[0], b[1]])
    }
}

struct Handlers {
    info: ServiceInfo,
    unit: u8,
    tp: DataType,
    reg: Reg,
}

#[derive(Serialize, Deserialize)]
struct ValuePayload {
    value: Value,
}

#[async_trait::async_trait]
impl RpcHandlers for Handlers {
    #[allow(clippy::cast_sign_loss, clippy::too_many_lines)]
    async fn handle_call(&self, event: RpcEvent) -> RpcResult {
        svc_rpc_need_ready!();
        let method = event.parse_method()?;
        let payload = event.payload();
        #[allow(clippy::match_single_binding)]
        match method {
            "var.get" =>
            {
                #[allow(clippy::cast_possible_wrap)]
                if payload.is_empty() {
                    let ctx = CONTEXT.lock().await;
                    let value = match self.tp {
                        DataType::Int => Value::I16(match self.reg {
                            Reg::Holding => ctx.get_holding(0).unwrap() as i16,
                            Reg::Input => ctx.get_input(0).unwrap() as i16,
                        }),
                        DataType::Uint => Value::U16(match self.reg {
                            Reg::Holding => ctx.get_holding(0).unwrap(),
                            Reg::Input => ctx.get_input(0).unwrap(),
                        }),
                        DataType::Dint => Value::I32(match self.reg {
                            Reg::Holding => ctx.get_holdings_as_u32(0).unwrap() as i32,
                            Reg::Input => ctx.get_inputs_as_u32(0).unwrap() as i32,
                        }),
                        DataType::Udint => Value::U32(match self.reg {
                            Reg::Holding => ctx.get_holdings_as_u32(0).unwrap(),
                            Reg::Input => ctx.get_inputs_as_u32(0).unwrap(),
                        }),
                        DataType::Lint => Value::I64(match self.reg {
                            Reg::Holding => ctx.get_holdings_as_u64(0).unwrap() as i64,
                            Reg::Input => ctx.get_inputs_as_u64(0).unwrap() as i64,
                        }),
                        DataType::Ulint => Value::U64(match self.reg {
                            Reg::Holding => ctx.get_holdings_as_u64(0).unwrap(),
                            Reg::Input => ctx.get_inputs_as_u64(0).unwrap(),
                        }),
                        DataType::Real => Value::F32(
                            match self.reg {
                                Reg::Holding => ctx.get_holdings_as_f32(0).unwrap(),
                                Reg::Input => ctx.get_inputs_as_f32(0).unwrap(),
                            }
                            .to_swapped_modbus_endianness(),
                        ),
                        DataType::Realb => Value::F32(match self.reg {
                            Reg::Holding => ctx.get_holdings_as_f32(0).unwrap(),
                            Reg::Input => ctx.get_inputs_as_f32(0).unwrap(),
                        }),
                    };
                    Ok(Some(pack(&ValuePayload { value })?))
                } else {
                    Err(RpcError::params(None))
                }
            }
            "var.set" => {
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let p: ValuePayload = unpack(payload)?;
                    let mut ctx = CONTEXT.lock().await;
                    match self.tp {
                        DataType::Int => {
                            let val: i16 = p.value.try_into()?;
                            match self.reg {
                                Reg::Holding => ctx.set_holding(0, val as u16).unwrap(),
                                Reg::Input => ctx.set_input(0, val as u16).unwrap(),
                            }
                        }
                        DataType::Uint => {
                            let val: u16 = p.value.try_into()?;
                            match self.reg {
                                Reg::Holding => ctx.set_holding(0, val).unwrap(),
                                Reg::Input => ctx.set_input(0, val).unwrap(),
                            }
                        }
                        DataType::Dint => {
                            let val: i32 = p.value.try_into()?;
                            match self.reg {
                                Reg::Holding => ctx.set_holdings_from_u32(0, val as u32).unwrap(),
                                Reg::Input => ctx.set_inputs_from_u32(0, val as u32).unwrap(),
                            }
                        }
                        DataType::Udint => {
                            let val: u32 = p.value.try_into()?;
                            match self.reg {
                                Reg::Holding => ctx.set_holdings_from_u32(0, val).unwrap(),
                                Reg::Input => ctx.set_inputs_from_u32(0, val).unwrap(),
                            }
                        }
                        DataType::Lint => {
                            let val: i64 = p.value.try_into()?;
                            match self.reg {
                                Reg::Holding => ctx.set_holdings_from_u64(0, val as u64).unwrap(),
                                Reg::Input => ctx.set_inputs_from_u64(0, val as u64).unwrap(),
                            }
                        }
                        DataType::Ulint => {
                            let val: u64 = p.value.try_into()?;
                            match self.reg {
                                Reg::Holding => ctx.set_holdings_from_u64(0, val).unwrap(),
                                Reg::Input => ctx.set_inputs_from_u64(0, val).unwrap(),
                            }
                        }
                        DataType::Real => {
                            let v: f32 = p.value.try_into()?;
                            let val = v.to_swapped_modbus_endianness();
                            match self.reg {
                                Reg::Holding => ctx.set_holdings_from_f32(0, val).unwrap(),
                                Reg::Input => ctx.set_inputs_from_f32(0, val).unwrap(),
                            }
                        }
                        DataType::Realb => {
                            let val: f32 = p.value.try_into()?;
                            match self.reg {
                                Reg::Holding => ctx.set_holdings_from_f32(0, val).unwrap(),
                                Reg::Input => ctx.set_inputs_from_f32(0, val).unwrap(),
                            }
                        }
                    };
                    Ok(None)
                }
            }
            _ => svc_handle_default_rpc(method, &self.info),
        }
    }
    async fn handle_frame(&self, frame: Frame) {
        eva_sim_modbus::process_modbus_frame::<0, 0, 4, 4>(
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
    #[serde(default, rename = "type")]
    tp: DataType,
    #[serde(default)]
    reg: Reg,
}

#[derive(Deserialize, Default, Copy, Clone)]
#[serde(rename_all = "UPPERCASE")]
enum DataType {
    #[default]
    Int,
    Uint,
    Dint,
    Udint,
    Lint,
    Ulint,
    Real,
    Realb,
}

#[derive(Deserialize, Default, Copy, Clone)]
#[serde(rename_all = "lowercase")]
enum Reg {
    #[default]
    #[serde(alias = "h")]
    Holding,
    #[serde(alias = "i")]
    Input,
}

#[svc_main]
async fn main(mut initial: Initial) -> EResult<()> {
    let config: Config = Config::deserialize(
        initial
            .take_config()
            .ok_or_else(|| Error::invalid_data("config not specified"))?,
    )?;
    let mut info = ServiceInfo::new(AUTHOR, VERSION, DESCRIPTION);
    info.add_method(ServiceMethod::new("var.get"));
    info.add_method(ServiceMethod::new("var.set").required("value"));
    let rpc = initial
        .init_rpc(Handlers {
            info,
            unit: config.unit,
            tp: config.tp,
            reg: config.reg,
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
