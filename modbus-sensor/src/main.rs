use eva_common::prelude::*;
use eva_sdk::prelude::*;
use eva_sdk::service::poc;
use ieee754::Ieee754;
use once_cell::sync::{Lazy, OnceCell};
use rand::Rng;
use rmodbus::server::context::ModbusContext;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

const AUTHOR: &str = "Bohemia Automation";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = "SIM Virtual Modbus sensor";

const UPDATE_INTERVAL: Duration = Duration::from_secs(1);

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

#[derive(Serialize)]
struct ValueResult {
    value: Value,
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
                    Ok(Some(pack(&ValueResult { value })?))
                } else {
                    Err(RpcError::params(None))
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
    #[serde(default)]
    source: Source,
    #[serde(default, rename = "type")]
    tp: DataType,
    #[serde(default)]
    reg: Reg,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase", tag = "kind", deny_unknown_fields)]
enum Source {
    #[default]
    None,
    Random,
    Timestamp,
    Counter,
    Udp {
        bind: String,
        #[serde(default)]
        long_float: bool,
    },
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

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::too_many_lines)]
async fn update_sensor_udp_server(
    bind: &str,
    long_float: bool,
    tp: DataType,
    reg: Reg,
) -> EResult<()> {
    let sock = UdpSocket::bind(bind).await?;
    info!("listening UDP at {}", bind);
    while !svc_is_terminating() {
        let mut buf = [0u8; 32];
        let _ = sock.recv_from(&mut buf).await?;
        let mut ctx = CONTEXT.lock().await;
        match tp {
            DataType::Int => {
                let val = if long_float {
                    f64::from_le_bytes(buf[0..8].try_into().unwrap()) as i16 as u16
                } else {
                    u16::from_le_bytes(buf[0..2].try_into().unwrap())
                };
                match reg {
                    Reg::Holding => ctx.set_holding(0, val).log_ef(),
                    Reg::Input => {
                        ctx.set_input(0, val).log_ef();
                    }
                }
            }
            DataType::Uint => {
                let val = if long_float {
                    f64::from_le_bytes(buf[0..8].try_into().unwrap()) as u16
                } else {
                    u16::from_le_bytes(buf[0..2].try_into().unwrap())
                };
                match reg {
                    Reg::Holding => ctx.set_holding(0, val).log_ef(),
                    Reg::Input => {
                        ctx.set_input(0, val).log_ef();
                    }
                }
            }
            DataType::Dint => {
                let val = if long_float {
                    f64::from_le_bytes(buf[0..8].try_into().unwrap()) as i32 as u32
                } else {
                    u32::from_le_bytes(buf[0..4].try_into().unwrap())
                };
                match reg {
                    Reg::Holding => ctx.set_holdings_from_u32(0, val).log_ef(),
                    Reg::Input => {
                        ctx.set_inputs_from_u32(0, val).log_ef();
                    }
                }
            }
            DataType::Udint => {
                let val = if long_float {
                    f64::from_le_bytes(buf[0..8].try_into().unwrap()) as u32
                } else {
                    u32::from_le_bytes(buf[0..4].try_into().unwrap())
                };
                match reg {
                    Reg::Holding => ctx.set_holdings_from_u32(0, val).log_ef(),
                    Reg::Input => {
                        ctx.set_inputs_from_u32(0, val).log_ef();
                    }
                }
            }
            DataType::Lint => {
                let val = if long_float {
                    f64::from_le_bytes(buf[0..8].try_into().unwrap()) as i64 as u64
                } else {
                    u64::from_le_bytes(buf[0..8].try_into().unwrap())
                };
                match reg {
                    Reg::Holding => ctx.set_holdings_from_u64(0, val).log_ef(),
                    Reg::Input => {
                        ctx.set_inputs_from_u64(0, val).log_ef();
                    }
                }
            }
            DataType::Ulint => {
                let val = if long_float {
                    f64::from_le_bytes(buf[0..8].try_into().unwrap()) as u64
                } else {
                    u64::from_le_bytes(buf[0..8].try_into().unwrap())
                };
                match reg {
                    Reg::Holding => ctx.set_holdings_from_u64(0, val).log_ef(),
                    Reg::Input => {
                        ctx.set_inputs_from_u64(0, val).log_ef();
                    }
                }
            }
            DataType::Real => {
                let val = if long_float {
                    f64::from_le_bytes(buf[0..8].try_into().unwrap()) as f32
                } else {
                    let val_i = u32::from_le_bytes(buf[0..4].try_into().unwrap());
                    Ieee754::from_bits(val_i)
                }
                .to_swapped_modbus_endianness();
                match reg {
                    Reg::Holding => ctx.set_holdings_from_f32(0, val).log_ef(),
                    Reg::Input => {
                        ctx.set_inputs_from_f32(0, val).log_ef();
                    }
                }
            }
            DataType::Realb => {
                let val = if long_float {
                    f64::from_le_bytes(buf[0..8].try_into().unwrap()) as f32
                } else {
                    let val_i = u32::from_le_bytes(buf[0..4].try_into().unwrap());
                    Ieee754::from_bits(val_i)
                };
                match reg {
                    Reg::Holding => ctx.set_holdings_from_f32(0, val).log_ef(),
                    Reg::Input => {
                        ctx.set_inputs_from_f32(0, val).log_ef();
                    }
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::too_many_lines)]
async fn update_sensor(source: Source, tp: DataType, reg: Reg) {
    let mut int = tokio::time::interval(UPDATE_INTERVAL);
    while !svc_is_terminating() {
        {
            int.tick().await;
            let mut ctx = CONTEXT.lock().await;
            macro_rules! set_val {
                ($val: expr) => {
                    match tp {
                        DataType::Int => match reg {
                            Reg::Holding => {
                                ctx.set_holding(0, $val as i16 as u16).log_ef();
                            }
                            Reg::Input => {
                                ctx.set_input(0, $val as i16 as u16).log_ef();
                            }
                        },
                        DataType::Uint => match reg {
                            Reg::Holding => {
                                ctx.set_holding(0, $val as u16).log_ef();
                            }
                            Reg::Input => {
                                ctx.set_input(0, $val as u16).log_ef();
                            }
                        },
                        DataType::Dint => match reg {
                            Reg::Holding => {
                                ctx.set_holdings_from_u32(0, $val as i32 as u32).log_ef();
                            }
                            Reg::Input => {
                                ctx.set_inputs_from_u32(0, $val as i32 as u32).log_ef();
                            }
                        },
                        DataType::Udint => match reg {
                            Reg::Holding => {
                                ctx.set_holdings_from_u32(0, $val as u32).log_ef();
                            }
                            Reg::Input => {
                                ctx.set_inputs_from_u32(0, $val as u32).log_ef();
                            }
                        },
                        DataType::Lint => match reg {
                            Reg::Holding => {
                                ctx.set_holdings_from_u64(0, $val as i64 as u64).log_ef();
                            }
                            Reg::Input => {
                                ctx.set_inputs_from_u64(0, $val as i64 as u64).log_ef();
                            }
                        },
                        DataType::Ulint => match reg {
                            Reg::Holding => {
                                ctx.set_holdings_from_u64(0, $val as u64).log_ef();
                            }
                            Reg::Input => {
                                ctx.set_inputs_from_u64(0, $val as u64).log_ef();
                            }
                        },
                        DataType::Real => {
                            let value = ($val as f32).to_swapped_modbus_endianness();
                            match reg {
                                Reg::Holding => {
                                    ctx.set_holdings_from_f32(0, value).log_ef();
                                }
                                Reg::Input => {
                                    ctx.set_inputs_from_f32(0, value).log_ef();
                                }
                            }
                        }
                        DataType::Realb => match reg {
                            Reg::Holding => {
                                ctx.set_holdings_from_f32(0, $val as f32).log_ef();
                            }
                            Reg::Input => {
                                ctx.set_inputs_from_f32(0, $val as f32).log_ef();
                            }
                        },
                    }
                };
            }
            match source {
                Source::None | Source::Udp { .. } => {}
                Source::Random => {
                    let mut rngen = rand::thread_rng();
                    let val: f64 =
                        rngen.gen_range(f64::from(std::u16::MIN)..f64::from(std::u16::MAX));
                    set_val!(val);
                }
                Source::Timestamp => {
                    let val = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64();
                    set_val!(val);
                }
                Source::Counter => {
                    let mut val = match tp {
                        DataType::Int | DataType::Uint => match reg {
                            Reg::Holding => f64::from(ctx.get_holding(0).unwrap()),
                            Reg::Input => f64::from(ctx.get_input(0).unwrap()),
                        },
                        DataType::Dint | DataType::Udint => match reg {
                            Reg::Holding => f64::from(ctx.get_holdings_as_u32(0).unwrap()),
                            Reg::Input => f64::from(ctx.get_inputs_as_u32(0).unwrap()),
                        },
                        DataType::Lint | DataType::Ulint => match reg {
                            Reg::Holding => ctx.get_holdings_as_u64(0).unwrap() as f64,
                            Reg::Input => ctx.get_inputs_as_u64(0).unwrap() as f64,
                        },
                        DataType::Real => match reg {
                            Reg::Holding => f64::from(
                                ctx.get_holdings_as_f32(0)
                                    .unwrap()
                                    .to_swapped_modbus_endianness(),
                            ),
                            Reg::Input => f64::from(
                                ctx.get_inputs_as_f32(0)
                                    .unwrap()
                                    .to_swapped_modbus_endianness(),
                            ),
                        },
                        DataType::Realb => match reg {
                            Reg::Holding => f64::from(ctx.get_holdings_as_f32(0).unwrap()),
                            Reg::Input => f64::from(ctx.get_inputs_as_f32(0).unwrap()),
                        },
                    };
                    if val > f64::from(u16::MAX) {
                        val = 0.0;
                    } else {
                        val += 1.0;
                    }
                    set_val!(val);
                }
            }
        }
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
    info.add_method(ServiceMethod::new("get"));
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
    match config.source {
        Source::None => {}
        Source::Udp { bind, long_float } => {
            let tp = config.tp;
            let reg = config.reg;
            tokio::spawn(async move {
                if let Err(e) = update_sensor_udp_server(&bind, long_float, tp, reg).await {
                    error!("source error {}", e);
                    poc();
                }
            });
        }
        _ => {
            tokio::spawn(update_sensor(config.source, config.tp, config.reg));
        }
    }
    svc_mark_ready(&client).await?;
    info!("{} started ({})", DESCRIPTION, initial.id());
    svc_block(&rpc).await;
    svc_mark_terminating(&client).await?;
    Ok(())
}
