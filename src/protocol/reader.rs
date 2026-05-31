
use crate::{error::Result, util::hex_upper};

pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        let byte = *self
            .data
            .get(self.pos)
            .ok_or("Unexpected end of Protocol18 stream")?;
        self.pos += 1;
        Ok(byte)
    }

    pub(crate) fn read_bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        if self.remaining() < count {
            return Err(format!("Expected {count} bytes, got {}", self.remaining()).into());
        }
        let start = self.pos;
        self.pos += count;
        Ok(&self.data[start..self.pos])
    }

    pub(crate) fn read_i16_le(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.read_bytes(2)?.try_into().unwrap()))
    }

    pub(crate) fn read_u16_le(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_bytes(2)?.try_into().unwrap()))
    }

    pub(crate) fn read_f32_le(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.read_bytes(4)?.try_into().unwrap()))
    }

    pub(crate) fn read_f64_le(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.read_bytes(8)?.try_into().unwrap()))
    }

    pub(crate) fn peek_hex(&self) -> String {
        let end = (self.pos + 16).min(self.data.len());
        let mut text = hex_upper(&self.data[self.pos..end]);
        if end < self.data.len() {
            text.push_str(" ...");
        }
        text
    }
}
