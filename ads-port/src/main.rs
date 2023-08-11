use binrw::{BinRead, BinWrite};
use eva_ads_common::{AdsError, AdsResult, AmsAddr, AmsCmd, AmsPacket, AmsPacketHeader, ClientId};
use eva_common::err_logger;
use eva_sdk::prelude::*;
use eva_sdk::service::{poc, set_poc};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::atomic;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const AUTHOR: &str = "Bohemia Automation";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = "SIM Virtual TwinCAT/ADS port";

const ROUTE_CLEAN_INTERVAL: Duration = Duration::from_secs(1);
const ROUTE_EXPIRED: Duration = Duration::from_secs(30);

static RPC: OnceCell<Arc<RpcClient>> = OnceCell::new();
static TIMEOUT: OnceCell<Duration> = OnceCell::new();

struct RouteEntry {
    svc_id: Arc<String>,
    last_alive: Instant,
}

impl RouteEntry {
    fn new(svc_id: &str) -> Self {
        Self {
            svc_id: svc_id.to_owned().into(),
            last_alive: Instant::now(),
        }
    }
}

type RouteMap = BTreeMap<AmsAddr, RouteEntry>;

static ADS_ROUTES: Lazy<Mutex<RouteMap>> = Lazy::new(<_>::default);

#[cfg(not(feature = "std-alloc"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

static VERBOSE: atomic::AtomicBool = atomic::AtomicBool::new(false);

err_logger!();

#[inline]
fn is_verbose() -> bool {
    VERBOSE.load(atomic::Ordering::Relaxed)
}

struct Handlers {
    info: ServiceInfo,
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
struct RouteInfo {
    ams_addr: String,
    svc_id: Arc<String>,
}

#[async_trait::async_trait]
impl RpcHandlers for Handlers {
    async fn handle_call(&self, event: RpcEvent) -> RpcResult {
        svc_rpc_need_ready!();
        let method = event.parse_method()?;
        let payload = event.payload();
        #[allow(clippy::match_single_binding)]
        match method {
            "route.ping" => {
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let route: AmsAddr = unpack(payload)?;
                    let mut ads_routes = ADS_ROUTES.lock();
                    ads_routes
                        .entry(route)
                        .or_insert_with(|| RouteEntry::new(event.sender()));
                    Ok(None)
                }
            }
            "route.unregister" => {
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let route: AmsAddr = unpack(payload)?;
                    ADS_ROUTES.lock().remove(&route);
                    Ok(None)
                }
            }
            "list" => {
                if payload.is_empty() {
                    let mut result: Vec<RouteInfo> = ADS_ROUTES
                        .lock()
                        .iter()
                        .map(|(k, v)| RouteInfo {
                            ams_addr: k.to_string(),
                            svc_id: v.svc_id.clone(),
                        })
                        .collect();
                    result.sort();
                    Ok(Some(pack(&result)?))
                } else {
                    Err(RpcError::params(None))
                }
            }
            _ => svc_handle_default_rpc(method, &self.info),
        }
    }
}

async fn handle_packet(packet: &AmsPacket) -> Result<Vec<u8>, AdsError> {
    let svc_id_opt = ADS_ROUTES
        .lock()
        .get(&AmsAddr::new(packet.dest_netid(), packet.dest_port()))
        .map(|v| v.svc_id.clone());
    if let Some(svc_id) = svc_id_opt {
        let timeout = *TIMEOUT.get().unwrap();
        let rpc = RPC.get().unwrap();
        let res = safe_rpc_call(
            rpc,
            &svc_id,
            "ads.call",
            pack(&packet)
                .map_err(|_| AdsError::GeneralDeviceError)?
                .into(),
            QoS::Processed,
            timeout,
        )
        .await
        .map_err(|_| AdsError::HostUnreachable)?;
        let result: AdsResult = unpack(res.payload()).map_err(|_| AdsError::GeneralClientError)?;
        result.into()
    } else {
        Err(AdsError::TargetMachineNotFoundPossiblyMissingAdsRoutes)
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    timeout: Duration,
    client_id: ClientId,
) -> EResult<()> {
    let verbose = is_verbose();
    loop {
        let mut data = [0_u8; 6];
        if let Err(e) = stream.read_exact(&mut data).await {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                break;
            }
            return Err(e.into());
        }
        let header = AmsPacketHeader::read(&mut Cursor::new(data)).map_err(Error::io)?;
        if header.length < 32 {
            return Err(Error::io("invalid client packet, too small"));
        }
        let mut buf = vec![0_u8; usize::try_from(header.length)?];
        tokio::time::timeout(timeout, stream.read_exact(&mut buf)).await??;
        let mut packet = AmsPacket::read(&mut Cursor::new(buf)).map_err(Error::io)?;
        if verbose {
            info!("{} IN {}, {}", client_id, header, packet);
        }
        if header.ams_cmd == AmsCmd::Command as u16 {
            packet.client_id.replace(client_id);
            match handle_packet(&packet).await {
                Ok(v) => packet.response(v),
                Err(e) => {
                    error!("AMS call error {}: {}", addr, e);
                    packet.response_err(e);
                }
            }
        } else {
            packet.response_err(AdsError::UnknownAmsCommand);
        };
        packet.route_back();
        if verbose {
            info!("OUT {}", packet);
        }
        let mut buf = Cursor::new(Vec::with_capacity(
            36 + usize::try_from(packet.data_length)?,
        ));
        let reply_header = AmsPacketHeader {
            ams_cmd: 0,
            length: 32 + packet.data_length,
        };
        reply_header.write(&mut buf).map_err(Error::io)?;
        packet.write(&mut buf).map_err(Error::io)?;
        tokio::time::timeout(timeout, stream.write_all(&buf.into_inner())).await??;
    }
    Ok(())
}

async fn run_server(addr: &str, me: &str, timeout: Duration) -> EResult<()> {
    #[derive(Serialize)]
    struct ClientInfo {
        client_id: ClientId,
    }
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, addr) = listener.accept().await?;
        let disconnect_topic = format!("SVE/{}/disconnect", me);
        tokio::spawn(async move {
            let client_id = addr;
            info!("client connected: {} {}", addr, client_id);
            if let Err(e) = handle_connection(stream, addr, timeout, client_id).await {
                error!("handler error {}: {}", addr, e);
            }
            info!("client disconnected: {} {}", addr, client_id);
            RPC.get()
                .unwrap()
                .client()
                .lock()
                .await
                .publish(
                    &disconnect_topic,
                    pack(&ClientInfo { client_id }).unwrap().into(),
                    QoS::Processed,
                )
                .await
                .log_ef();
        });
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    listen: String,
    #[serde(default)]
    verbose: bool,
}

async fn route_cleaner() {
    let mut int = tokio::time::interval(ROUTE_CLEAN_INTERVAL);
    loop {
        ADS_ROUTES
            .lock()
            .retain(|_, v| v.last_alive.elapsed() < ROUTE_EXPIRED);
        int.tick().await;
    }
}

#[svc_main]
async fn main(mut initial: Initial) -> EResult<()> {
    let config: Config = Config::deserialize(
        initial
            .take_config()
            .ok_or_else(|| Error::invalid_data("config not specified"))?,
    )?;
    VERBOSE.store(config.verbose, atomic::Ordering::Relaxed);
    let timeout = initial.timeout();
    TIMEOUT
        .set(timeout)
        .map_err(|_| Error::core("Unable to set TIMEOUT"))?;
    set_poc(Some(Duration::from_secs(1)));
    let mut info = ServiceInfo::new(AUTHOR, VERSION, DESCRIPTION);
    info.add_method(ServiceMethod::new("list"));
    let rpc = initial.init_rpc(Handlers { info }).await?;
    initial.drop_privileges()?;
    let client = rpc.client().clone();
    RPC.set(rpc.clone())
        .map_err(|_| Error::core("Unable to set RPC"))?;
    svc_init_logs(&initial, client.clone())?;
    tokio::spawn(route_cleaner());
    let me = initial.id().to_owned();
    tokio::spawn(async move {
        loop {
            if run_server(&config.listen, &me, timeout)
                .await
                .log_err()
                .is_err()
            {
                poc();
            }
        }
    });
    svc_start_signal_handlers();
    svc_mark_ready(&client).await?;
    info!("{} started ({})", DESCRIPTION, initial.id());
    svc_block(&rpc).await;
    svc_mark_terminating(&client).await?;
    Ok(())
}
