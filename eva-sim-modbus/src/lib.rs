use busrt::client::AsyncClient;
use busrt::{rpc::Rpc, Frame, QoS};
use eva_common::{EResult, Error};
use log::error;
use once_cell::sync::OnceCell;
use rmodbus::{
    server::{context::ModbusContext, ModbusFrame},
    ModbusFrameBuf, ModbusProto,
};
use uuid::Uuid;

static BUS_TOPIC_IN: OnceCell<String> = OnceCell::new();
static BUS_TOPIC_OUT: OnceCell<String> = OnceCell::new();

pub async fn init(port_svc: &str, client: &mut dyn AsyncClient) -> EResult<()> {
    let bus_topic_in = format!("SVE/{}/bus/in/", port_svc);
    let bus_topic_out = format!("SVE/{}/bus/out/", port_svc);
    client
        .subscribe(&format!("{}#", bus_topic_in), QoS::Processed)
        .await?;
    BUS_TOPIC_IN
        .set(bus_topic_in)
        .map_err(|_| Error::core("Unable to set BUS_TOPIC_IN"))?;
    BUS_TOPIC_OUT
        .set(bus_topic_out)
        .map_err(|_| Error::core("Unable to set BUS_TOPIC_OUT"))?;
    Ok(())
}

pub async fn process_modbus_frame<
    const C: usize,
    const D: usize,
    const I: usize,
    const H: usize,
>(
    frame: Frame,
    ctx: &mut ModbusContext<C, D, I, H>,
    unit: u8,
    rpc: &impl Rpc,
) {
    if let Some(topic) = frame.topic() {
        if let Some(cid) = topic.strip_prefix(BUS_TOPIC_IN.get().unwrap()) {
            match cid.parse::<Uuid>() {
                Ok(client_id) => {
                    let mut response = Vec::new();
                    let mut buf = frame.payload().to_vec();
                    buf.resize(256, 0);
                    let frame_buf: ModbusFrameBuf = buf.try_into().unwrap();
                    let mut frame =
                        ModbusFrame::new(unit, &frame_buf, ModbusProto::TcpUdp, &mut response);
                    if let Err(e) = frame.parse() {
                        error!("client {} frame parse error: {}", client_id, e);
                        return;
                    }
                    if frame.processing_required {
                        let result = if frame.readonly {
                            frame.process_read(ctx)
                        } else {
                            frame.process_write(ctx)
                        };
                        if let Err(e) = result {
                            error!("client {} frame process error: {}", client_id, e);
                            return;
                        }
                    }
                    if frame.response_required {
                        frame.finalize_response().unwrap();
                        let _ = rpc
                            .client()
                            .lock()
                            .await
                            .publish(
                                &format!("{}{}", BUS_TOPIC_OUT.get().unwrap(), client_id),
                                response.into(),
                                QoS::Processed,
                            )
                            .await;
                    }
                }
                Err(e) => {
                    error!("invalid incoming topic {}: {}", topic, e);
                }
            }
        }
    }
}
