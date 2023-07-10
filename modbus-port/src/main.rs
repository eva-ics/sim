use bmart_derive::EnumStr;
use busrt::QoS;
use crc16::{State, MODBUS};
use eva_common::prelude::*;
use eva_sdk::prelude::*;
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_serial::{DataBits, Parity, SerialPortBuilderExt, StopBits};
use uuid::Uuid;

const AUTHOR: &str = "Bohemia Automation";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = "SIM Virtual Modbus port";

static RPC: OnceCell<Arc<RpcClient>> = OnceCell::new();
static BUS_TOPIC_IN: OnceCell<String> = OnceCell::new();
static BUS_TOPIC_OUT: OnceCell<String> = OnceCell::new();
static CLIENTS: Lazy<Mutex<BTreeMap<Uuid, async_channel::Sender<Vec<u8>>>>> =
    Lazy::new(<_>::default);

#[cfg(not(feature = "std-alloc"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

err_logger!();

struct Handlers {
    info: ServiceInfo,
}

#[async_trait::async_trait]
impl RpcHandlers for Handlers {
    // Handle RPC call
    async fn handle_call(&self, event: RpcEvent) -> RpcResult {
        svc_rpc_need_ready!();
        let method = event.parse_method()?;
        #[allow(clippy::match_single_binding)]
        match method {
            _ => svc_handle_default_rpc(method, &self.info),
        }
    }
    async fn handle_frame(&self, frame: Frame) {
        svc_need_ready!();
        if let Some(topic) = frame.topic() {
            if let Some(cid) = topic.strip_prefix(BUS_TOPIC_OUT.get().unwrap()) {
                match cid.parse::<Uuid>() {
                    Ok(client_id) => {
                        let tx_o = CLIENTS.lock().get(&client_id).map(Clone::clone);
                        if let Some(tx) = tx_o {
                            tx.send(frame.payload().to_vec()).await.log_ef();
                        }
                    }
                    Err(e) => {
                        error!("invalid incoming topic {}: {}", topic, e);
                    }
                }
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    listen: Vec<ListenConfig>,
    #[serde(default)]
    verbose: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListenConfig {
    path: String,
    protocol: Protocol,
}

#[derive(Deserialize, Copy, Clone, EnumStr)]
#[serde(rename_all = "lowercase")]
#[enumstr(rename_all = "lowercase")]
enum Protocol {
    Tcp,
    Rtu,
}

#[derive(EnumStr)]
#[enumstr(rename_all = "lowercase")]
enum Direction {
    In,
    Out,
}

fn log_packet(protocol: Protocol, direction: Direction, client_id: Uuid, data: &[u8]) {
    let packet: Vec<String> = data.iter().map(|v| hex::encode([*v])).collect();
    info!(
        "{} client {} {} packet: {}",
        protocol,
        direction,
        client_id,
        packet.join(" ")
    );
}

async fn launch_tcp_server(listen: &str, verbose: bool) -> EResult<()> {
    let listener = TcpListener::bind(&listen).await?;
    info!("tcp port ready {}", listen);
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    tokio::spawn(async move {
                        let client_id = Uuid::new_v4();
                        if verbose {
                            info!("tcp client connected ({}): {}", addr, client_id);
                        }
                        let (tx, rx) = async_channel::bounded(1024);
                        CLIENTS.lock().insert(client_id, tx);
                        loop {
                            let mut buf = vec![0; 256];
                            tokio::select! {
                                ev = rx.recv() => {
                                    if let Ok(data) = ev {
                                        if verbose {
                                            log_packet(
                                                Protocol::Tcp, Direction::Out, client_id, &data);
                                        }
                                        if let Err(e) = stream.write_all(&data).await {
                                            error!("tcp client {} write error: {}", client_id, e);
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                                read = stream.read(&mut buf) => {
                                    let len = read.unwrap_or(0);
                                    if len < 6 {
                                        if verbose {
                                            info!("tcp client disconnected: {}", client_id);
                                        }
                                        break;
                                    }
                                    buf.truncate(len);
                                    if verbose {
                                        log_packet(
                                            Protocol::Tcp, Direction::In, client_id, &buf);
                                    }
                                    let topic = format!("{}{}",
                                        BUS_TOPIC_IN.get().unwrap(), client_id);
                                    RPC.get()
                                        .unwrap()
                                        .client()
                                        .lock()
                                        .await
                                        .publish(&topic, buf.into(), QoS::Processed)
                                        .await
                                        .log_ef();
                                        }
                            }
                        }
                        CLIENTS.lock().remove(&client_id);
                    });
                }
                Err(e) => {
                    error!("listener error: {}", e);
                }
            }
        }
    });
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn launch_rtu_server(listen: String, verbose: bool) -> EResult<()> {
    let mut sp = listen.split(':');
    let port_path = sp.next().unwrap();
    let baud_rate: u32 = sp
        .next()
        .ok_or_else(|| Error::invalid_params("baud rate missing"))?
        .parse()?;
    let data_bits: DataBits = match sp
        .next()
        .ok_or_else(|| Error::invalid_params("data bits missing"))?
    {
        "5" => DataBits::Five,
        "6" => DataBits::Six,
        "7" => DataBits::Seven,
        "8" => DataBits::Eight,
        v => {
            return Err(Error::invalid_params(format!(
                "unsupported data bits value: {}",
                v
            )))
        }
    };
    let parity: Parity = match sp
        .next()
        .ok_or_else(|| Error::invalid_params("parity missing"))?
    {
        "N" => Parity::None,
        "E" => Parity::Even,
        "O" => Parity::Odd,
        v => {
            return Err(Error::invalid_params(format!(
                "unsupported parity value: {}",
                v
            )))
        }
    };
    let stop_bits: StopBits = match sp
        .next()
        .ok_or_else(|| Error::invalid_params("stop bits missing"))?
    {
        "1" => StopBits::One,
        "2" => StopBits::Two,
        v => {
            return Err(Error::invalid_params(format!(
                "unsupported stop bits value: {}",
                v
            )))
        }
    };
    let mut stream = tokio_serial::new(port_path, baud_rate)
        .data_bits(data_bits)
        .parity(parity)
        .stop_bits(stop_bits)
        .open_native_async()
        .map_err(Error::io)?;
    let client_id = Uuid::new_v4();
    if verbose {
        info!("rtu client id: {}", client_id);
    }
    let (tx, rx) = async_channel::bounded(1024);
    CLIENTS.lock().insert(client_id, tx);
    info!("rtu port ready {}", listen);
    tokio::spawn(async move {
        loop {
            let mut buf = vec![0; 256];
            tokio::select! {
                ev = rx.recv() => {
                    if let Ok(mut data) = ev {
                        if data.len() < 7 {
                            error!("invalid bus/rt packet");
                            continue;
                        }
                        data = data.split_off(6);
                        let crc: u16 = State::<MODBUS>::calculate(&data);
                        data.extend(crc.to_le_bytes());
                        if verbose {
                            log_packet(
                                Protocol::Tcp, Direction::Out, client_id, &data);
                        }
                        if let Err(e) = stream.write_all(&data).await {
                            error!("rtu client {} write error: {}", client_id, e);
                            continue;
                        }
                    } else {
                        break;
                    }
                }
                read = stream.read(&mut buf) => {
                    let len = read.unwrap_or(0);
                    if len < 3 {
                        continue;
                    }
                    buf.truncate(len);
                    if verbose {
                        log_packet(
                            Protocol::Rtu, Direction::In, client_id, &buf);
                    }
                    let crc: u16 = State::<MODBUS>::calculate(&buf[..len-2]);
                    if crc.to_le_bytes() != buf[len-2..] {
                        error!("rtu client {} frame checksum error", client_id);
                        continue;
                    }
                    buf.truncate(len-2);
                    let mut req = Vec::with_capacity(len + 4);
                    req.extend(&[0, 0, 0, 0]);
                    #[allow(clippy::cast_possible_truncation)]
                    req.extend((len as u16).to_be_bytes());
                    req.extend(&buf);
                    let topic = format!("{}{}",
                        BUS_TOPIC_IN.get().unwrap(), client_id);
                    RPC.get()
                        .unwrap()
                        .client()
                        .lock()
                        .await
                        .publish(&topic, req.into(), QoS::Processed)
                        .await
                        .log_ef();
                        }
            }
        }
        CLIENTS.lock().remove(&client_id);
        info!("rtu port closed {}", listen);
    });
    Ok(())
}

#[svc_main]
async fn main(mut initial: Initial) -> EResult<()> {
    let config: Config = Config::deserialize(
        initial
            .take_config()
            .ok_or_else(|| Error::invalid_data("config not specified"))?,
    )?;
    let info = ServiceInfo::new(AUTHOR, VERSION, DESCRIPTION);
    let rpc = initial.init_rpc(Handlers { info }).await?;
    initial.drop_privileges()?;
    let client = rpc.client().clone();
    RPC.set(rpc.clone())
        .map_err(|_| Error::core("Unable to set RPC"))?;
    let bus_topic_in = format!("SVE/{}/bus/{}/", initial.id(), Direction::In);
    let bus_topic_out = format!("SVE/{}/bus/{}/", initial.id(), Direction::Out);
    client
        .lock()
        .await
        .subscribe(&format!("{}#", bus_topic_out), QoS::Processed)
        .await?;
    BUS_TOPIC_IN
        .set(bus_topic_in)
        .map_err(|_| Error::core("Unable to set BUS_TOPIC_IN"))?;
    BUS_TOPIC_OUT
        .set(bus_topic_out)
        .map_err(|_| Error::core("Unable to set BUS_TOPIC_OUT"))?;
    svc_init_logs(&initial, client.clone())?;
    svc_start_signal_handlers();
    for listen in config.listen {
        match listen.protocol {
            Protocol::Tcp => {
                launch_tcp_server(&listen.path, config.verbose).await?;
            }
            Protocol::Rtu => {
                launch_rtu_server(listen.path, config.verbose).await?;
            }
        }
    }
    svc_mark_ready(&client).await?;
    info!("{} started ({})", DESCRIPTION, initial.id());
    svc_block(&rpc).await;
    svc_mark_terminating(&client).await?;
    Ok(())
}
