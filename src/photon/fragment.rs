// src/photon/fragment.rs

use crate::error::Result;
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct PendingSegment {
    payload: Vec<u8>,
    written: usize,
    total_length: usize,
}

#[derive(Clone, Debug, Default)]
pub struct FragmentReassembler {
    pending_segments: HashMap<i32, PendingSegment>,
}

impl FragmentReassembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_fragment(
        &mut self,
        start_sequence_number: i32,
        total_length: usize,
        fragment_offset: usize,
        fragment: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        if fragment_offset + fragment.len() > total_length {
            return Err(format!(
                "Fragment out of bounds: offset={} length={} total={}",
                fragment_offset,
                fragment.len(),
                total_length,
            )
            .into());
        }

        let pending = self
            .pending_segments
            .entry(start_sequence_number)
            .or_insert_with(|| PendingSegment {
                payload: vec![0; total_length],
                written: 0,
                total_length,
            });

        if pending.total_length != total_length {
            return Err(format!(
                "Fragment total length mismatch for sequence {}: expected {}, got {}",
                start_sequence_number, pending.total_length, total_length,
            )
            .into());
        }

        pending.payload[fragment_offset..fragment_offset + fragment.len()]
            .copy_from_slice(fragment);

        pending.written += fragment.len();

        if pending.written >= pending.total_length {
            let payload = self
                .pending_segments
                .remove(&start_sequence_number)
                .expect("pending segment should exist")
                .payload;

            return Ok(Some(payload));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::FragmentReassembler;

    #[test]
    fn reassembles_complete_fragment_sequence() {
        let mut fragments = FragmentReassembler::new();

        assert_eq!(fragments.push_fragment(7, 5, 0, b"he").unwrap(), None);
        assert_eq!(
            fragments.push_fragment(7, 5, 2, b"llo").unwrap(),
            Some(b"hello".to_vec())
        );
    }

    #[test]
    fn rejects_out_of_bounds_fragment() {
        let mut fragments = FragmentReassembler::new();

        assert!(fragments.push_fragment(7, 5, 4, b"xx").is_err());
    }
}
