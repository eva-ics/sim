use binrw::prelude::*;
use binrw::PosValue;
use bmart_derive::EnumStr;
use eva_common::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Cursor;
use std::net::SocketAddr;
use std::str::FromStr;

mod arr_idx;
pub mod context;
mod types;

pub use types::{AdsError, AdsIGrp, Command, DataType, DATA_TYPES, DATA_TYPES_NAMES_LEN};

pub const ADS_OK: &[u8] = &[0, 0, 0, 0];
pub const ADS_SUM_MAX: u32 = 500;

pub type ClientId = SocketAddr;

#[derive(EnumStr, Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u16)]
#[serde(rename_all = "lowercase")]
pub enum AdsState {
    Invalid = 0,
    Idle = 1,
    Reset = 2,
    Init = 3,
    Start = 4,
    Run = 5,
    Stop = 6,
    SaveCfg = 7,
    LoadCfg = 8,
    PowerFail = 9,
    PowerGood = 10,
    Error = 11,
    Shutdown = 12,
    Suspend = 13,
    Resume = 14,
    Config = 15,
    Reconfig = 16,
    Stopping = 17,
    Incompatible = 18,
    Exception = 19,
}

#[derive(PartialOrd, PartialEq, Ord, Eq, Serialize, Deserialize, Copy, Clone)]
pub struct AmsAddr {
    pub net_id: AmsNetId,
    pub port: u16,
}

#[binrw]
#[brw(little)]
pub struct SymUploadInfo {
    pub symbols: u32,
    pub symbols_len: u32,
    pub types: u32,
    pub types_len: u32,
}

impl AmsAddr {
    #[inline]
    pub fn new(net_id: AmsNetId, port: u16) -> Self {
        Self { net_id, port }
    }
}

impl FromStr for AmsAddr {
    type Err = Error;
    fn from_str(s: &str) -> EResult<Self> {
        let mut sp = s.splitn(2, ':');
        let net_id = sp.next().unwrap().parse()?;
        let port = sp
            .next()
            .ok_or_else(|| Error::invalid_params("AMS port missing"))?
            .parse()?;
        Ok(Self { net_id, port })
    }
}

impl fmt::Display for AmsAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.net_id, self.port)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdsResult {
    Ok(Vec<u8>),
    Err(AdsError),
}

impl From<AdsResult> for Result<Vec<u8>, AdsError> {
    fn from(res: AdsResult) -> Self {
        match res {
            AdsResult::Ok(v) => Ok(v),
            AdsResult::Err(e) => Err(e),
        }
    }
}

impl From<Result<Vec<u8>, AdsError>> for AdsResult {
    fn from(res: Result<Vec<u8>, AdsError>) -> Self {
        match res {
            Ok(v) => AdsResult::Ok(v),
            Err(e) => AdsResult::Err(e),
        }
    }
}

#[binrw]
#[brw(little)]
#[derive(Default)]
pub struct AmsPacketHeader {
    pub ams_cmd: u16,
    pub length: u32,
}

impl fmt::Display for AmsPacketHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AMS_CMD: {}", self.ams_cmd)
    }
}

#[derive(EnumStr, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum AmsCmd {
    Command = 0x0,
}

impl TryFrom<u16> for AmsCmd {
    type Error = Error;
    fn try_from(v: u16) -> EResult<Self> {
        Ok(match v {
            x if x == AmsCmd::Command as u16 => AmsCmd::Command,
            x => {
                return Err(Error::invalid_data(format!(
                    "unsupported AMS cmd code: {}",
                    x
                )));
            }
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[binrw]
pub struct AmsNetId([u8; 6]);

impl FromStr for AmsNetId {
    type Err = Error;
    fn from_str(s: &str) -> EResult<Self> {
        let mut result = Vec::with_capacity(6);
        let mut sp = s.split('.');
        for _ in 0..6 {
            result.push(
                sp.next()
                    .ok_or_else(|| Error::invalid_params("invalid AMS NetId"))?
                    .parse()?,
            );
        }
        Ok(AmsNetId(result.try_into().unwrap()))
    }
}

impl fmt::Display for AmsNetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, v) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{}", v)?;
        }
        Ok(())
    }
}

#[binrw]
#[brw(little)]
#[derive(Serialize, Deserialize)]
pub struct AmsPacket {
    pub dest_netid: AmsNetId,
    pub dest_port: u16,
    pub src_netid: AmsNetId,
    pub src_port: u16,
    pub command: u16,
    pub state_flags: u16,
    pub data_length: u32,
    pub error_code: u32,
    pub invoke_id: u32,
    #[br(count = data_length)]
    pub data: Vec<u8>,
    #[brw(ignore)]
    pub client_id: Option<ClientId>,
}

impl AmsPacket {
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    #[inline]
    pub fn data_as_cursor(&self) -> Cursor<&[u8]> {
        Cursor::new(&self.data)
    }
    pub fn data_at_pos<T>(&self, pos: &PosValue<T>) -> Result<&[u8], Box<dyn std::error::Error>> {
        let buf = &self.data[usize::try_from(pos.pos)?..];
        Ok(buf)
    }
    pub fn data_at_pos_sized<T>(
        &self,
        pos: &PosValue<T>,
        length: u32,
    ) -> Result<&[u8], Box<dyn std::error::Error>> {
        let buf = &self.data
            [usize::try_from(pos.pos)?..usize::try_from(u32::try_from(pos.pos)? + length)?];
        Ok(buf)
    }
    #[inline]
    pub fn command(&self) -> types::Command {
        self.command.into()
    }
    #[inline]
    pub fn dest_netid(&self) -> AmsNetId {
        self.dest_netid
    }
    #[inline]
    pub fn dest_port(&self) -> u16 {
        self.dest_port
    }
    pub fn route_back(&mut self) {
        std::mem::swap(&mut self.src_netid, &mut self.dest_netid);
        std::mem::swap(&mut self.src_port, &mut self.dest_port);
    }
    pub fn generate_response(&mut self, error_code: u32, data: Option<Vec<u8>>) {
        self.state_flags |= 1;
        if let Ok(len) = u32::try_from(data.as_ref().map_or(0, Vec::len)) {
            self.error_code = error_code;
            self.data_length = len;
            self.data = data.unwrap_or_default();
        } else {
            self.error_code = AdsError::InternalError as u32;
            self.data_length = 0;
            self.data = vec![];
        }
    }
    #[inline]
    pub fn response(&mut self, data: Vec<u8>) {
        self.generate_response(0, Some(data));
    }
    #[inline]
    pub fn response_err(&mut self, error: AdsError) {
        self.generate_response(error as u32, None);
    }
}

impl fmt::Display for AmsPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DEST: {}:{}, ", self.dest_netid, self.dest_port)?;
        write!(f, "SRC: {}:{}, ", self.src_netid, self.src_port)?;
        write!(f, "CMD: {}, ", self.command)?;
        write!(f, "SF: {}, ", self.state_flags)?;
        write!(f, "DLEN: {}, ", self.data_length)?;
        write!(f, "ERR: {}, ", self.error_code)?;
        write!(f, "ID: {}, ", self.invoke_id)?;
        Ok(())
    }
}

#[derive(BinRead, Debug)]
#[brw(little)]
pub struct AdsReq {
    pub index_group: u32,
    pub index_offset: u32,
    pub length: u32,
    pub data_offset: PosValue<()>,
}

impl fmt::Display for AdsReq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "G: 0x{:x}, O: 0x{:x}, LEN: {}",
            self.index_group, self.index_offset, self.length
        )
    }
}

#[derive(BinRead)]
#[brw(little)]
pub struct AdsRwReq {
    pub index_group: u32,
    pub index_offset: u32,
    pub read_length: u32,
    pub write_length: u32,
    pub data_offset: PosValue<()>,
}

impl fmt::Display for AdsRwReq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "G: 0x{:x}, O: 0x{:x}, RLEN: {}, WLEN: {}",
            self.index_group, self.index_offset, self.read_length, self.write_length
        )
    }
}
