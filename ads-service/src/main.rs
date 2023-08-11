use binrw::prelude::*;
use eva_ads_common::{
    context::{Context, Handle, Variable},
    AdsError, AdsIGrp, AdsReq, AdsResult, AdsRwReq, AdsState, AmsAddr, AmsPacket, ClientId,
    Command, DataType, SymUploadInfo, ADS_OK, ADS_SUM_MAX, DATA_TYPES, DATA_TYPES_NAMES_LEN,
};
use eva_common::common_payloads::ParamsIdOwned;
use eva_common::prelude::*;
use eva_sdk::prelude::*;
use eva_sdk::service::{poc, set_poc};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::atomic;
use std::sync::Arc;
use std::time::Duration;

// TODO: notifications

const AUTHOR: &str = "Bohemia Automation";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = "SIM Virtual ADS service";
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

const UPDATE_ROUTE_INTERVAL: Duration = Duration::from_secs(5);

static RPC: OnceCell<Arc<RpcClient>> = OnceCell::new();
static REG: OnceCell<Registry> = OnceCell::new();

static CONTEXT: Lazy<Mutex<Context>> = Lazy::new(<_>::default);

static DISCONNECT_TOPIC: OnceCell<String> = OnceCell::new();

#[cfg(not(feature = "std-alloc"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

static VERBOSE: atomic::AtomicBool = atomic::AtomicBool::new(false);
static AUTO_CLEANUP: atomic::AtomicBool = atomic::AtomicBool::new(false);

static DEVICE_STATE: Lazy<Mutex<AdsState>> = Lazy::new(|| Mutex::new(AdsState::Run));

#[inline]
fn is_running() -> bool {
    *DEVICE_STATE.lock() == AdsState::Run
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceState {
    state: AdsState,
}

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
    me: AmsAddr,
}

fn unpack_str(data: &[u8]) -> Result<&str, std::str::Utf8Error> {
    Ok(std::str::from_utf8(data)?.trim_end_matches(char::from(0)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    port_svc: String,
    ams_addr: String,
    #[serde(default)]
    verbose: bool,
    #[serde(default)]
    auto_cleanup: bool,
    #[serde(default)]
    symbols: Vec<Symbol>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Symbol {
    name: String,
    #[serde(default)]
    size: u32,
    #[serde(rename = "type")]
    data_type: DataType,
}

#[inline]
fn is_verbose() -> bool {
    VERBOSE.load(atomic::Ordering::Relaxed)
}

fn device_info(name: &str, version: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut result = Vec::with_capacity(20);
    let mut sp = version.split('.');
    result.extend(ADS_OK);
    result.push(sp.next().map_or(Ok(0), str::parse)?);
    result.push(sp.next().map_or(Ok(0), str::parse)?);
    result.extend(sp.next().map_or(Ok(0u16), str::parse)?.to_le_bytes());
    let mut n = name.as_bytes().to_vec();
    n.resize(15, 0);
    result.extend(n);
    result.push(0);
    Ok(result)
}

fn device_state(state: AdsState) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend(ADS_OK);
    data.extend((state as u16).to_le_bytes());
    data.extend(0u16.to_le_bytes());
    data
}

fn ads_rw_result(data: &[u8], length: u32) -> Result<Vec<u8>, AdsError> {
    let data = &data[..std::cmp::min(data.len(), usize::try_from(length)?)];
    let mut result = Vec::with_capacity(8 + data.len());
    result.extend(ADS_OK);
    result.extend(u32::try_from(data.len())?.to_le_bytes());
    result.extend(data);
    Ok(result)
}

fn handle_ads_read(
    index_group: u32,
    index_offset: u32,
    read_length: u32,
    client_id: ClientId,
    ctx: &mut Context,
) -> Result<Vec<u8>, AdsError> {
    match index_group {
        x if x == AdsIGrp::SymValbyhnd as u32 => {
            let handle = ctx.get_handle(index_offset, client_id)?;
            ctx.read_by_handle(handle)
        }
        x if x == AdsIGrp::SymUploadinfo2 as u32 => {
            let mut buf = Cursor::new(Vec::with_capacity(64));
            let info = SymUploadInfo {
                symbols: u32::try_from(ctx.len())?,
                symbols_len: u32::try_from(ctx.info_ex_size_len())?,
                types: u32::try_from(DATA_TYPES.len())?,
                types_len: u32::try_from(45 * DATA_TYPES.len() + *DATA_TYPES_NAMES_LEN * 2)?,
            };
            info.write(&mut buf)?;
            let mut result = buf.into_inner();
            result.resize(64, 0);
            Ok(result)
        }
        x if x == AdsIGrp::SymDtUpload as u32 => {
            let types_len = 45 * DATA_TYPES.len() + *DATA_TYPES_NAMES_LEN * 2;
            let mut buf = Vec::with_capacity(types_len);
            for data_type in DATA_TYPES {
                buf.extend(data_type.packed_info_ex()?);
            }
            Ok(buf)
        }
        x if x == AdsIGrp::SymUpload as u32 => ctx.packed_var_info_ex(),
        _ => ctx.read(index_group, index_offset, usize::try_from(read_length)?),
    }
}

fn handle_ads_write(
    index_group: u32,
    index_offset: u32,
    data: &[u8],
    client_id: ClientId,
    ctx: &mut Context,
) -> Result<(), AdsError> {
    match index_group {
        x if x == AdsIGrp::SymReleasehnd as u32 => {
            if index_offset == 0 {
                let handle_id =
                    u32::from_le_bytes(data.try_into().map_err(|_| AdsError::InvalidAmsLength)?);
                ctx.release_handle_by_id(handle_id, client_id);
            } else {
                return Err(AdsError::InvalidIndexOffset);
            }
        }
        x if x == AdsIGrp::SymValbyhnd as u32 => {
            let handle = ctx.get_handle(index_offset, client_id)?;
            ctx.write_by_handle(handle, data)?;
        }
        _ => {
            ctx.write(index_group, index_offset, data)?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn handle_ads_read_write(
    index_group: u32,
    index_offset: u32,
    read_length: u32,
    data: &[u8],
    client_id: ClientId,
    ctx: &mut Context,
    allow_sumup: bool,
) -> Result<Vec<u8>, AdsError> {
    match index_group {
        x if x == AdsIGrp::SymHndbyname as u32 => {
            if index_offset == 0 {
                if let Ok(name) = unpack_str(data) {
                    let handle = ctx.create_handle(name, client_id)?;
                    Ok(handle.id().to_le_bytes().to_vec())
                } else {
                    Err(AdsError::InvalidParameterValueS)
                }
            } else {
                Err(AdsError::InvalidIndexOffset)
            }
        }
        x if x == AdsIGrp::SymInfobyname as u32 => {
            if index_offset == 0 {
                if let Ok(name) = unpack_str(data) {
                    let var = ctx.get_variable_entry_by_path(name)?;
                    var.packed_info()
                } else {
                    Err(AdsError::InvalidParameterValueS)
                }
            } else {
                Err(AdsError::InvalidIndexOffset)
            }
        }
        x if x == AdsIGrp::SymInfobynameex as u32 => {
            if index_offset == 0 {
                if let Ok(name) = unpack_str(data) {
                    let var = ctx.get_variable_entry_by_path(name)?;
                    var.packed_info_ex()
                } else {
                    Err(AdsError::InvalidParameterValueS)
                }
            } else {
                Err(AdsError::InvalidIndexOffset)
            }
        }
        x if x == AdsIGrp::SumupRead as u32 => {
            if index_offset > ADS_SUM_MAX {
                return Err(AdsError::InvalidAmsLength);
            }
            let mut result = Vec::with_capacity(usize::try_from(index_offset)? * 4);
            let mut result_data = Vec::new();
            let mut requests = Vec::new();
            let mut c = Cursor::new(data);
            for _ in 0..index_offset {
                requests.push(AdsReq::read(&mut c)?);
            }
            for req in requests {
                if is_verbose() {
                    info!("{} Read {}", client_id, req);
                }
                match handle_ads_read(
                    req.index_group,
                    req.index_offset,
                    req.length,
                    client_id,
                    ctx,
                ) {
                    Ok(v) => {
                        result.extend(ADS_OK);
                        result_data.extend(v);
                    }
                    Err(e) => {
                        result.extend((e as u32).to_le_bytes());
                        result_data.resize(result_data.len() + usize::try_from(req.length)?, 0);
                    }
                }
            }
            result.extend(&result_data);
            Ok(result)
        }
        x if x == AdsIGrp::SumupReadEx as u32 => {
            if index_offset > ADS_SUM_MAX {
                return Err(AdsError::InvalidAmsLength);
            }
            let mut result = Vec::with_capacity(usize::try_from(index_offset)? * 8);
            let mut result_data = Vec::new();
            let mut requests = Vec::new();
            let mut c = Cursor::new(data);
            for _ in 0..index_offset {
                requests.push(AdsReq::read(&mut c)?);
            }
            for req in requests {
                if is_verbose() {
                    info!("{} Read {}", client_id, req);
                }
                match handle_ads_read(
                    req.index_group,
                    req.index_offset,
                    req.length,
                    client_id,
                    ctx,
                ) {
                    Ok(v) => {
                        result.extend(ADS_OK);
                        result.extend(u32::try_from(v.len())?.to_le_bytes());
                        result_data.extend(v);
                    }
                    Err(e) => {
                        result.extend((e as u32).to_le_bytes());
                        result.extend(req.length.to_le_bytes());
                        result_data.resize(result_data.len() + usize::try_from(req.length)?, 0);
                    }
                }
            }
            result.extend(&result_data);
            Ok(result)
        }
        x if x == AdsIGrp::SumupWrite as u32 => {
            if index_offset > ADS_SUM_MAX {
                return Err(AdsError::InvalidAmsLength);
            }
            let mut result = Vec::with_capacity(usize::try_from(index_offset)? * 4);
            let mut requests = Vec::new();
            let mut c = Cursor::new(data);
            for _ in 0..index_offset {
                requests.push(AdsReq::read(&mut c)?);
            }
            let mut offset = index_offset * 12;
            for req in requests {
                if is_verbose() {
                    info!("{} Write {}", client_id, req);
                }
                let to = usize::try_from(offset + req.length)?;
                if to > data.len() {
                    return Err(AdsError::InvalidAmsLength);
                }
                let data_s = &data[usize::try_from(offset)?..to];
                match handle_ads_write(req.index_group, req.index_offset, data_s, client_id, ctx) {
                    Ok(()) => result.extend(ADS_OK),
                    Err(e) => result.extend((e as u32).to_le_bytes()),
                }
                offset += req.length;
            }
            Ok(result)
        }
        x if x == AdsIGrp::SumupReadWrite as u32 && allow_sumup => {
            if index_offset > ADS_SUM_MAX {
                return Err(AdsError::InvalidAmsLength);
            }
            let mut result = Vec::with_capacity(usize::try_from(index_offset)? * 8);
            let mut result_data = Vec::new();
            let mut requests = Vec::new();
            let mut c = Cursor::new(data);
            for _ in 0..index_offset {
                requests.push(AdsRwReq::read(&mut c)?);
            }
            let mut offset = index_offset * 16;
            for req in requests {
                if is_verbose() {
                    info!("{} ReadWrite {}", client_id, req);
                }
                let to = usize::try_from(offset + req.write_length)?;
                if to > data.len() {
                    return Err(AdsError::InvalidAmsLength);
                }
                let data_s = &data[usize::try_from(offset)?..to];
                match handle_ads_read_write(
                    req.index_group,
                    req.index_offset,
                    req.read_length,
                    data_s,
                    client_id,
                    ctx,
                    false,
                ) {
                    Ok(v) => {
                        result.extend(ADS_OK);
                        result.extend(u32::try_from(v.len())?.to_le_bytes());
                        result_data.extend(v);
                    }
                    Err(e) => {
                        result.extend((e as u32).to_le_bytes());
                        result.extend(req.read_length.to_le_bytes());
                        result_data
                            .resize(result_data.len() + usize::try_from(req.read_length)?, 0);
                    }
                }
                offset += req.write_length;
            }
            result.extend(&result_data);
            Ok(result)
        }
        _ => {
            let res = ctx.read(index_group, index_offset, usize::try_from(read_length)?)?;
            ctx.write(index_group, index_offset, data)?;
            Ok(res)
        }
    }
}

fn handle_packet(raw_packet: &[u8], me: AmsAddr) -> Result<Vec<u8>, AdsError> {
    macro_rules! need_run {
        () => {
            if !is_running() {
                return (Err(AdsError::InvalidIndexGroup));
            }
        };
    }
    let packet: AmsPacket = unpack(raw_packet).map_err(|_| AdsError::InvalidAmsFragment)?;
    if packet.dest_netid() == me.net_id && packet.dest_port() == me.port {
        let client_id = packet.client_id.ok_or(AdsError::InvalidAmsFragment)?;
        match packet.command() {
            Command::ReadState => Ok(device_state(*DEVICE_STATE.lock())),
            Command::DevInfo => device_info(PKG_NAME, VERSION).map_err(Into::into),
            Command::Read => {
                let params = AdsReq::read(&mut packet.data_as_cursor())
                    .map_err(|_| AdsError::InvalidAmsLength)?;
                if is_verbose() {
                    info!("{} Read {}", client_id, params);
                }
                need_run!();
                ads_rw_result(
                    &handle_ads_read(
                        params.index_group,
                        params.index_offset,
                        params.length,
                        client_id,
                        &mut CONTEXT.lock(),
                    )?,
                    params.length,
                )
            }
            Command::Write => {
                let params = AdsReq::read(&mut packet.data_as_cursor())
                    .map_err(|_| AdsError::InvalidAmsLength)?;
                if is_verbose() {
                    info!("{} Write {}", client_id, params);
                }
                need_run!();
                let data = packet.data_at_pos_sized(&params.data_offset, params.length)?;
                handle_ads_write(
                    params.index_group,
                    params.index_offset,
                    data,
                    client_id,
                    &mut CONTEXT.lock(),
                )
                .map(|_| ADS_OK.to_vec())
            }
            Command::ReadWrite => {
                let params = AdsRwReq::read(&mut packet.data_as_cursor())
                    .map_err(|_| AdsError::InvalidAmsLength)?;
                let data = packet.data_at_pos_sized(&params.data_offset, params.write_length)?;
                if is_verbose() {
                    info!("{} ReadWrite {}", client_id, params);
                }
                need_run!();
                ads_rw_result(
                    &handle_ads_read_write(
                        params.index_group,
                        params.index_offset,
                        params.read_length,
                        data,
                        client_id,
                        &mut CONTEXT.lock(),
                        true,
                    )?,
                    params.read_length,
                )
            }
            Command::WriteControl => {
                need_run!();
                Ok(ADS_OK.to_vec())
            }
            _ => Err(AdsError::UnknownCommandId),
        }
    } else {
        Err(AdsError::TargetPortNotFoundPossiblyAdsServerNotStarted)
    }
}

#[async_trait::async_trait]
#[allow(clippy::too_many_lines)]
impl RpcHandlers for Handlers {
    async fn handle_call(&self, event: RpcEvent) -> RpcResult {
        svc_rpc_need_ready!();
        let method = event.parse_method()?;
        let payload = event.payload();
        #[allow(clippy::match_single_binding)]
        match method {
            "ads.call" => {
                let result: AdsResult = handle_packet(payload, self.me).into();
                Ok(Some(pack(&result)?))
            }
            "handle.list" => {
                if payload.is_empty() {
                    Ok(Some(pack(
                        &CONTEXT
                            .lock()
                            .list_handles()
                            .into_iter()
                            .map(|(k, v)| (k.to_string(), v))
                            .collect::<BTreeMap<String, Vec<Handle>>>(),
                    )?))
                } else {
                    Err(RpcError::params(None))
                }
            }
            "var.get" => {
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let p: ParamsIdOwned = unpack(payload)?;
                    let ctx = CONTEXT.lock();
                    let entry = ctx.get_variable_entry_by_path(&p.i).map_err(Error::from)?;
                    let data = ctx
                        .read(entry.index_group, entry.index_offset, entry.size)
                        .map_err(Error::from)?;
                    Ok(Some(pack(&entry.data_to_value(&data))?))
                }
            }
            "var.set" => {
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    #[derive(Deserialize)]
                    struct ParamsSet {
                        i: String,
                        value: Value,
                    }
                    let p: ParamsSet = unpack(payload)?;
                    let mut ctx = CONTEXT.lock();
                    let entry = ctx.get_variable_entry_by_path(&p.i).map_err(Error::from)?;
                    let data = entry.value_to_data(p.value)?;
                    let index_group = entry.index_group;
                    let index_offset = entry.index_offset;
                    ctx.write(index_group, index_offset, &data)
                        .map_err(Error::from)?;
                    Ok(None)
                }
            }
            "var.list" => {
                #[derive(Deserialize)]
                struct Params {
                    #[serde(default)]
                    full: bool,
                }
                #[derive(Serialize)]
                struct VarInfo<'a> {
                    name: &'a str,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    value: Option<Value>,
                }
                let full = if payload.is_empty() {
                    false
                } else {
                    let p: Params = unpack(payload)?;
                    p.full
                };
                let ctx = CONTEXT.lock();
                let result: Result<Vec<VarInfo>, AdsError> = ctx
                    .variables()
                    .iter()
                    .map(|(name, var)| {
                        let entry = var.as_entry(None)?;
                        let data = ctx.read(entry.index_group, entry.index_offset, entry.size)?;
                        let value = entry.data_to_value(&data);
                        Ok(VarInfo {
                            name: name.as_str(),
                            value: if full { Some(value) } else { None },
                        })
                    })
                    .collect();
                Ok(Some(pack(&result.map_err(Error::from)?)?))
            }
            "state.get" => {
                if payload.is_empty() {
                    Ok(Some(pack(&DeviceState {
                        state: *DEVICE_STATE.lock(),
                    })?))
                } else {
                    Err(RpcError::params(None))
                }
            }
            "state.set" => {
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let p: DeviceState = unpack(payload)?;
                    *DEVICE_STATE.lock() = p.state;
                    let v = DeviceState { state: p.state };
                    REG.get().unwrap().key_set("device_state", v).await?;
                    Ok(None)
                }
            }
            _ => svc_handle_default_rpc(method, &self.info),
        }
    }
    async fn handle_frame(&self, frame: Frame) {
        svc_need_ready!();
        if let Some(topic) = frame.topic() {
            if topic == DISCONNECT_TOPIC.get().unwrap() {
                #[derive(Deserialize)]
                struct ClientInfo {
                    client_id: ClientId,
                }
                if let Ok(p) = unpack::<ClientInfo>(frame.payload()) {
                    if AUTO_CLEANUP.load(atomic::Ordering::Relaxed) {
                        CONTEXT.lock().release_handles_by_client(p.client_id);
                    } else {
                        CONTEXT.lock().release_empty_client(p.client_id);
                    }
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
    {
        let mut ctx = CONTEXT.lock();
        for symbol in &config.symbols {
            ctx.add_variable(Variable::new(&symbol.name, symbol.data_type, symbol.size))?;
        }
    }
    VERBOSE.store(config.verbose, atomic::Ordering::Relaxed);
    AUTO_CLEANUP.store(config.auto_cleanup, atomic::Ordering::Relaxed);
    let timeout = initial.timeout();
    let me: AmsAddr = config.ams_addr.parse()?;
    let ams_addr_payload = busrt::borrow::Cow::Referenced(pack(&me)?.into());
    let mut info = ServiceInfo::new(AUTHOR, VERSION, DESCRIPTION);
    info.add_method(ServiceMethod::new("handle.list"));
    info.add_method(ServiceMethod::new("state.get"));
    info.add_method(ServiceMethod::new("state.set").required("state"));
    info.add_method(ServiceMethod::new("var.get").required("i"));
    info.add_method(
        ServiceMethod::new("var.set")
            .required("i")
            .required("value"),
    );
    info.add_method(ServiceMethod::new("var.list").optional("full"));
    let rpc = initial.init_rpc(Handlers { info, me }).await?;
    initial.drop_privileges()?;
    let registry = initial.init_registry(&rpc);
    if let Ok(v) = registry.key_get("device_state").await {
        let device_state = DeviceState::deserialize(v)?;
        *DEVICE_STATE.lock() = device_state.state;
    }
    let client = rpc.client().clone();
    let disconnect_topic = format!("SVE/{}/disconnect", config.port_svc);
    client
        .lock()
        .await
        .subscribe(&disconnect_topic, QoS::Processed)
        .await?;
    DISCONNECT_TOPIC
        .set(disconnect_topic)
        .map_err(|_| Error::core("Unable to set DISCONNECT_TOPIC"))?;
    RPC.set(rpc.clone())
        .map_err(|_| Error::core("Unable to set RPC"))?;
    REG.set(registry)
        .map_err(|_| Error::core("unable to set registry object"))?;
    svc_init_logs(&initial, client.clone())?;
    svc_start_signal_handlers();
    set_poc(Some(Duration::from_secs(1)));
    let ams_addr_payload_c = ams_addr_payload.clone();
    let rpc_c = rpc.clone();
    let port_svc_c = config.port_svc.clone();
    tokio::spawn(async move {
        if svc_wait_core(&rpc_c, timeout, true)
            .await
            .log_err()
            .is_err()
        {
            poc();
        }
        let mut int = tokio::time::interval(UPDATE_ROUTE_INTERVAL);
        while svc_is_active() {
            safe_rpc_call(
                &rpc_c,
                &port_svc_c,
                "route.ping",
                ams_addr_payload_c.clone(),
                QoS::Processed,
                timeout,
            )
            .await
            .log_ef();
            int.tick().await;
        }
    });
    svc_mark_ready(&client).await?;
    info!("{} started ({})", DESCRIPTION, initial.id());
    svc_block(&rpc).await;
    svc_mark_terminating(&client).await?;
    let _ = safe_rpc_call(
        &rpc,
        &config.port_svc,
        "route.unregister",
        ams_addr_payload,
        QoS::Processed,
        timeout,
    )
    .await;
    Ok(())
}
