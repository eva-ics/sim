use crate::arr_idx::{parse_array_index, ArrayIndex};
use crate::types::{AdsError, DataType};
use crate::ClientId;
use binrw::prelude::*;
use double_map::DHashMap;
use eva_common::value::Value;
use eva_common::{EResult, Error};
use serde::{Deserialize, Serialize};
use std::collections::{btree_map::Entry, BTreeMap};
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Write};
use std::ops::RangeInclusive;
use unicase::UniCase;

const SUM_IDX_GROUP_RESERVED_MIN: u32 = 0xF000;
const SUM_IDX_GROUP_RESERVED_MAX: u32 = 0xFFFF;

const SUM_IDX_GROUP_RESERVED: &RangeInclusive<u32> =
    &(SUM_IDX_GROUP_RESERVED_MIN..=SUM_IDX_GROUP_RESERVED_MAX);

const IDX_GROUP_DEFAULT: u32 = 0x4040;
const MAX_HANDLE_ID: u32 = 0xF_FFFF;

#[derive(Debug)]
struct ClientHandles {
    handles: DHashMap<u32, Handle, Handle>,
}

#[derive(Default, Debug)]
pub struct Context {
    groups: BTreeMap<u32, IndexGroup>,
    variables: BTreeMap<UniCase<String>, VariableData>,
    handles: BTreeMap<ClientId, ClientHandles>,
}

macro_rules! get_var {
    ($vars: expr, $path: expr) => {{
        let (name, array_index) = parse_array_index($path)?;
        if let Some(var) = $vars.get(&UniCase::from(name)) {
            var.as_entry(array_index)?
        } else {
            return Err(AdsError::SymbolNotFound);
        }
    }};
}

impl Context {
    #[inline]
    pub fn variables(&self) -> &BTreeMap<UniCase<String>, VariableData> {
        &self.variables
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.variables.len()
    }
    pub fn list_handles(&self) -> BTreeMap<ClientId, Vec<Handle>> {
        self.handles
            .iter()
            .map(|(id, v)| (*id, v.handles.values().copied().collect()))
            .collect()
    }
    /// # Panics
    ///
    /// Should not panic
    pub fn packed_var_info_ex(&self) -> Result<Vec<u8>, AdsError> {
        struct Index {
            idx: [u32; 2],
            i: usize,
        }
        let mut buf = Vec::with_capacity(self.info_ex_size_len());
        let mut indexes = Vec::with_capacity(self.variables.len());
        let mut infos = Vec::with_capacity(self.variables.len());
        for (i, var) in self.variables.values().enumerate() {
            infos.push(var.as_entry(None)?.packed_info_ex()?);
            indexes.push(Index { idx: [0, 0], i });
        }
        indexes.sort_by(|a, b| a.idx.partial_cmp(&b.idx).unwrap());
        for ix in indexes {
            buf.extend(&infos[ix.i]);
        }
        Ok(buf)
    }
    /// # Panics
    ///
    /// Will panic on usize < 32
    pub fn info_ex_size_len(&self) -> usize {
        self.variables
            .values()
            .map(|v| v.as_entry(None).unwrap().packed_info_ex_len())
            .sum()
    }
    pub fn get_handle(&self, handle_id: u32, client_id: ClientId) -> Result<Handle, AdsError> {
        self.handles
            .get(&client_id)
            .and_then(|h| h.handles.get_key1(&handle_id))
            .copied()
            .ok_or(AdsError::SymbolNotFound)
    }
    pub fn create_handle(&mut self, path: &str, client_id: ClientId) -> Result<Handle, AdsError> {
        let var_entry = get_var!(self.variables, path);
        match self.handles.entry(client_id) {
            Entry::Vacant(o) => {
                let handle_id = 1;
                let handle = var_entry.to_handle(handle_id);
                let mut m = DHashMap::new();
                m.insert(handle_id, handle, handle);
                o.insert(ClientHandles { handles: m });
                Ok(handle)
            }
            Entry::Occupied(mut o) => {
                let mut handle = var_entry.to_handle(0);
                let h = o.get_mut();
                if let Some(handle) = h.handles.get_key2(&handle) {
                    Ok(*handle)
                } else {
                    for id in 1..=MAX_HANDLE_ID {
                        if !h.handles.contains_key1(&id) {
                            handle.id = id;
                            h.handles.insert(handle.id, handle, handle);
                            return Ok(handle);
                        }
                    }
                    Err(AdsError::NoFreeSemaphores)
                }
            }
        }
    }
    pub fn release_handle_by_id(&mut self, handle_id: u32, client_id: ClientId) {
        if let Some(h) = self.handles.get_mut(&client_id) {
            h.handles.remove_key1(&handle_id);
        }
    }
    pub fn release_handles_by_client(&mut self, client_id: ClientId) {
        self.handles.remove(&client_id);
    }
    pub fn release_empty_client(&mut self, client_id: ClientId) {
        if let Some(handles) = self.handles.get(&client_id) {
            if handles.handles.is_empty() {
                self.handles.remove(&client_id);
            }
        }
    }
    #[inline]
    pub fn add_variable(&mut self, v: Variable) -> EResult<u32> {
        self.add_variable_to_group(v, IDX_GROUP_DEFAULT)
    }
    pub fn add_variable_to_group(&mut self, v: Variable, index_group: u32) -> EResult<u32> {
        if SUM_IDX_GROUP_RESERVED.contains(&index_group) {
            return Err(Error::access("index group is reserved"));
        }
        if let Entry::Vacant(var_entry) = self.variables.entry(v.name.clone().into()) {
            let index_offset = u32::try_from(match self.groups.entry(index_group) {
                Entry::Vacant(o) => {
                    o.insert(IndexGroup::new(v.size));
                    0
                }
                Entry::Occupied(mut o) => {
                    let ig = o.get_mut();
                    if ig.data.len() + v.size >= usize::try_from(u32::MAX)? {
                        return Err(Error::failed("index group out of space"));
                    }
                    let ofs = ig.data.len();
                    ig.expand(v.size);
                    ofs
                }
            })?;
            let vd = v.into_variable_data(index_group, index_offset);
            var_entry.insert(vd);
            Ok(index_offset)
        } else {
            Err(Error::busy("the varialbe already exists"))
        }
    }
    pub fn get_variable_entry_by_path(&self, path: &str) -> Result<VariableEntry, AdsError> {
        Ok(get_var!(self.variables, path))
    }
    #[inline]
    pub fn read_by_handle(&self, handle: Handle) -> Result<Vec<u8>, AdsError> {
        self.read(handle.index_group, handle.index_offset, handle.size)
    }
    pub fn read(
        &self,
        index_group: u32,
        index_offset: u32,
        length: usize,
    ) -> Result<Vec<u8>, AdsError> {
        if let Some(group) = self.groups.get(&index_group) {
            let pos = usize::try_from(index_offset)?;
            let end = pos + length;
            if end <= group.data.len() {
                Ok(group.data[pos..end].to_vec())
            } else {
                Err(AdsError::InvalidIndexOffset)
            }
        } else {
            Err(AdsError::InvalidIndexGroup)
        }
    }
    #[inline]
    pub fn write_by_handle(&mut self, handle: Handle, data: &[u8]) -> Result<(), AdsError> {
        if handle.size >= data.len() {
            self.write(handle.index_group, handle.index_offset, data)
        } else {
            Err(AdsError::InvalidAlignment)
        }
    }
    pub fn write(
        &mut self,
        index_group: u32,
        index_offset: u32,
        data: &[u8],
    ) -> Result<(), AdsError> {
        if let Some(group) = self.groups.get_mut(&index_group) {
            let pos = usize::try_from(index_offset)?;
            if pos + data.len() <= group.data.len() {
                let mut c = Cursor::new(&mut group.data[pos..]);
                c.write_all(data)?;
                Ok(())
            } else {
                Err(AdsError::InvalidIndexOffset)
            }
        } else {
            Err(AdsError::InvalidIndexGroup)
        }
    }
}

#[derive(Debug)]
pub struct VariableData {
    name: String,
    comment: Option<String>,
    data_type: DataType,
    index_group: u32,
    index_offset: u32,
    size: usize,
    array_len: usize,
}

impl VariableData {
    pub fn as_entry(&self, array_index: Option<ArrayIndex>) -> Result<VariableEntry, AdsError> {
        let dt_size = self.data_type.size();
        let (pos, size, array_len) = if let Some(idx) = array_index {
            if let Some(length) = idx.length() {
                let length = usize::try_from(length)?;
                if usize::try_from(idx.index())? + length > self.array_len {
                    return Err(AdsError::InvalidArrayIndex);
                }
                (
                    usize::try_from(idx.index())? * dt_size,
                    length * dt_size,
                    length,
                )
            } else {
                let pos = usize::try_from(idx.index())? * dt_size;
                if pos >= self.size {
                    return Err(AdsError::InvalidArrayIndex);
                }
                (pos, self.size - pos, (self.size - pos) / dt_size)
            }
        } else {
            (0, self.size, self.array_len)
        };
        Ok(VariableEntry {
            name: &self.name,
            comment: self.comment.as_deref(),
            data_type: self.data_type,
            index_group: self.index_group,
            index_offset: u32::try_from(pos)? + self.index_offset,
            size,
            array_len,
        })
    }
}

#[derive(Debug)]
pub struct Variable {
    name: String,
    comment: Option<String>,
    data_type: DataType,
    size: usize,
    array_len: usize,
}

impl Variable {
    /// # Panics
    ///
    /// Will panic on usize < 32
    pub fn new(name: &str, data_type: DataType, array_len: u32) -> Self {
        let size = if array_len > 0 {
            data_type.size() * usize::try_from(array_len).unwrap()
        } else {
            data_type.size()
        };
        Self {
            name: name.to_owned(),
            comment: None,
            data_type,
            size,
            array_len: usize::try_from(array_len).unwrap(),
        }
    }
    #[inline]
    pub fn comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_owned());
        self
    }
    fn into_variable_data(self, index_group: u32, index_offset: u32) -> VariableData {
        VariableData {
            name: self.name,
            comment: self.comment,
            data_type: self.data_type,
            index_group,
            index_offset,
            size: self.size,
            array_len: self.array_len,
        }
    }
}

#[derive(Debug)]
pub struct VariableEntry<'a> {
    pub name: &'a str,
    pub comment: Option<&'a str>,
    pub data_type: DataType,
    pub index_group: u32,
    pub index_offset: u32,
    pub size: usize,
    pub array_len: usize,
}

#[binrw]
#[brw(little)]
struct VarInfo {
    index_group: u32,
    index_offset: u32,
    size: u32,
}

#[binrw]
#[brw(little)]
struct VarInfoEx {
    length: u32,
    index_group: u32,
    index_offset: u32,
    size: u32,
    data_type: DataType,
    flags: u16,
    legacy_array_dim: u16,
    name_len: u16,
    symbol_len: u16,
    comment_len: u16,
}

impl<'a> VariableEntry<'a> {
    pub fn packed_info(&self) -> Result<Vec<u8>, AdsError> {
        let mut buf = Cursor::new(Vec::with_capacity(12));
        let info = VarInfo {
            index_group: self.index_group,
            index_offset: self.index_offset,
            size: u32::try_from(self.size)?,
        };
        info.write(&mut buf)?;
        Ok(buf.into_inner())
    }
    pub fn packed_info_ex_len(&self) -> usize {
        let var_name = self.name.as_bytes();
        let data_type_name = self.data_type.as_str().as_bytes();
        let var_comment = self.comment.as_ref().map(|v| v.as_bytes());
        33 + var_name.len() + data_type_name.len() + var_comment.map_or(0, <[u8]>::len)
    }
    pub fn packed_info_ex(&self) -> Result<Vec<u8>, AdsError> {
        let var_name = self.name.as_bytes();
        let data_type_name = self.data_type.as_str().as_bytes();
        let var_comment = self.comment.as_ref().map(|v| v.as_bytes());
        let length =
            33 + var_name.len() + data_type_name.len() + var_comment.map_or(0, <[u8]>::len);
        let mut buf = Cursor::new(Vec::with_capacity(length));
        let info = VarInfoEx {
            length: u32::try_from(length)?,
            index_group: self.index_group,
            index_offset: self.index_offset,
            size: u32::try_from(self.size)?,
            data_type: self.data_type,
            flags: 0,
            legacy_array_dim: u16::try_from(self.array_len)?,
            name_len: u16::try_from(var_name.len())?,
            symbol_len: u16::try_from(data_type_name.len())?,
            comment_len: u16::try_from(var_comment.map_or(0, <[u8]>::len))?,
        };
        info.write(&mut buf)?;
        buf.write_all(var_name)?;
        buf.write_all(&[0x20])?;
        buf.write_all(data_type_name)?;
        buf.write_all(&[0x20])?;
        if let Some(c) = var_comment {
            buf.write_all(c)?;
        }
        buf.write_all(&[0x20])?;
        Ok(buf.into_inner())
    }
    pub fn to_handle(&self, id: u32) -> Handle {
        Handle {
            id,
            index_group: self.index_group,
            index_offset: self.index_offset,
            size: self.size,
        }
    }
    pub fn data_to_value(&self, data: &[u8]) -> Value {
        if self.array_len == 0 {
            convert_to_value(data, self.data_type)
        } else {
            let mut result = Vec::with_capacity(self.array_len);
            for d in data.chunks(self.data_type.size()) {
                result.push(convert_to_value(d, self.data_type));
            }
            Value::Seq(result)
        }
    }
    pub fn value_to_data(&self, value: Value) -> EResult<Vec<u8>> {
        if let Value::Seq(seq) = value {
            let mut result = Vec::new();
            for s in seq {
                result.extend(convert_from_value(s, self.data_type)?);
            }
            Ok(result)
        } else {
            convert_from_value(value, self.data_type)
        }
    }
}

fn convert_to_value(data: &[u8], data_type: DataType) -> Value {
    match data_type {
        #[allow(clippy::cast_possible_wrap)]
        DataType::Int8 => Value::I8(data[0] as i8),
        DataType::Uint8 => Value::U8(data[0]),
        DataType::Int16 => Value::I16(i16::from_le_bytes(data.try_into().unwrap())),
        DataType::Uint16 => Value::U16(u16::from_le_bytes(data.try_into().unwrap())),
        DataType::Int32 => Value::I32(i32::from_le_bytes(data.try_into().unwrap())),
        DataType::Uint32 => Value::U32(u32::from_le_bytes(data.try_into().unwrap())),
        DataType::Int64 => Value::I64(i64::from_le_bytes(data.try_into().unwrap())),
        DataType::Uint64 => Value::U64(u64::from_le_bytes(data.try_into().unwrap())),
        DataType::Real32 => Value::F32(f32::from_le_bytes(data.try_into().unwrap())),
        DataType::Real64 => Value::F64(f64::from_le_bytes(data.try_into().unwrap())),
        DataType::String | DataType::Wstring => Value::Char(data[0] as char),
        _ => Value::Unit,
    }
}

fn convert_from_value(value: Value, data_type: DataType) -> EResult<Vec<u8>> {
    match data_type {
        #[allow(clippy::cast_sign_loss)]
        DataType::Int8 => Ok(vec![i8::try_from(value)? as u8]),
        DataType::Uint8 => Ok(vec![u8::try_from(value)?]),
        DataType::Int16 => Ok(i16::try_from(value)?.to_le_bytes().to_vec()),
        DataType::Uint16 => Ok(u16::try_from(value)?.to_le_bytes().to_vec()),
        DataType::Int32 => Ok(i32::try_from(value)?.to_le_bytes().to_vec()),
        DataType::Uint32 => Ok(u32::try_from(value)?.to_le_bytes().to_vec()),
        DataType::Int64 => Ok(i64::try_from(value)?.to_le_bytes().to_vec()),
        DataType::Uint64 => Ok(u64::try_from(value)?.to_le_bytes().to_vec()),
        DataType::Real32 => Ok(f32::try_from(value)?.to_le_bytes().to_vec()),
        DataType::Real64 => Ok(f64::try_from(value)?.to_le_bytes().to_vec()),
        DataType::String | DataType::Wstring => Ok(value.to_string().as_bytes().to_vec()),
        _ => Err(Error::not_implemented("unsupported data type")),
    }
}

#[derive(Debug)]
struct IndexGroup {
    data: Vec<u8>,
}

impl IndexGroup {
    fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }
    fn expand(&mut self, size: usize) {
        self.data.resize(self.data.len() + size, 0);
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Handle {
    id: u32,
    pub index_group: u32,
    pub index_offset: u32,
    pub size: usize,
}

impl Handle {
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }
}

impl Hash for Handle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index_group.hash(state);
        self.index_offset.hash(state);
        self.size.hash(state);
    }
}

impl PartialEq for Handle {
    fn eq(&self, other: &Self) -> bool {
        self.index_group == other.index_group
            && self.index_offset == other.index_offset
            && self.size == other.size
    }
}
impl Eq for Handle {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_context() -> Result<(), Box<dyn std::error::Error>> {
        let client_id = "127.0.0.1:20000".parse()?;
        let mut ctx = Context::default();
        let var = Variable::new("test", DataType::Int32, 0);
        ctx.add_variable(var)?;
        let var = Variable::new("test2", DataType::Int32, 2);
        ctx.add_variable(var)?;
        let var = ctx.get_variable_entry_by_path("test2[0-1]")?;
        assert_eq!(var.index_group, IDX_GROUP_DEFAULT);
        assert_eq!(var.index_offset, 4);
        assert_eq!(var.size, 8);
        assert_eq!(var.array_len, 2);
        let var = ctx.get_variable_entry_by_path("TEST2[0-1]")?;
        assert_eq!(var.index_group, IDX_GROUP_DEFAULT);
        assert_eq!(var.index_offset, 4);
        assert_eq!(var.size, 8);
        assert_eq!(var.array_len, 2);
        let handle = ctx.create_handle("test2[1]", client_id)?;
        assert_eq!(handle.id(), 1);
        assert_eq!(handle.index_group, IDX_GROUP_DEFAULT);
        assert_eq!(handle.index_offset, 8);
        assert_eq!(handle.size, 4);
        let buf = &[1, 2, 3, 4];
        ctx.write_by_handle(handle, buf)?;
        assert_eq!(
            &ctx.groups.get(&IDX_GROUP_DEFAULT).unwrap().data[8..12],
            buf
        );
        let data = ctx.read_by_handle(handle)?;
        assert_eq!(data, buf);
        let handle = ctx.create_handle("test2", client_id)?;
        assert_eq!(handle.id(), 2);
        assert_eq!(handle.index_group, IDX_GROUP_DEFAULT);
        assert_eq!(handle.index_offset, 4);
        assert_eq!(handle.size, 8);
        let buf = &[9, 8, 7, 6, 5, 4, 3, 2];
        ctx.write_by_handle(handle, buf)?;
        assert_eq!(
            &ctx.groups.get(&IDX_GROUP_DEFAULT).unwrap().data[4..12],
            buf
        );
        let data = ctx.read_by_handle(handle)?;
        assert_eq!(data, buf);
        ctx.release_handle_by_id(handle.id(), client_id);
        let handle = ctx.create_handle("test2[0-1]", client_id)?;
        assert_eq!(handle.id(), 2);
        assert_eq!(handle.index_group, IDX_GROUP_DEFAULT);
        assert_eq!(handle.index_offset, 4);
        assert_eq!(handle.size, 8);
        let buf = &[1, 8, 7, 1, 1, 4, 3, 1];
        ctx.write_by_handle(handle, buf)?;
        assert_eq!(
            &ctx.groups.get(&IDX_GROUP_DEFAULT).unwrap().data[4..12],
            buf
        );
        let data = ctx.read_by_handle(handle)?;
        assert_eq!(data, buf);
        let var = ctx.get_variable_entry_by_path("test2[2-1]")?;
        assert_eq!(var.size, 0);
        let handle = ctx.create_handle("test2[1-0]", client_id)?;
        assert_eq!(handle.id(), 3);
        assert_eq!(handle.index_group, IDX_GROUP_DEFAULT);
        assert_eq!(handle.index_offset, 8);
        assert_eq!(handle.size, 0);
        Ok(())
    }
}
