#![allow(dead_code)]
use alloc::{
    boxed::Box,
    string::String,
    vec::Vec,
};

pub trait SearchPattern: Sync {
    fn length(&self) -> usize;
    fn is_matching(&self, target: &[u8]) -> bool;

    fn find(&self, buffer: &[u8]) -> Option<usize> {
        if self.length() > buffer.len() {
            return None;
        }

        for (index, window) in buffer.windows(self.length()).enumerate() {
            if !self.is_matching(window) {
                continue;
            }

            return Some(index as usize);
        }

        None
    }
}

#[derive(Debug)]
pub enum BytePattern {
    Any,
    Value(u8),
}

impl BytePattern {
    pub fn matches_byte(&self, target: u8) -> bool {
        match self {
            BytePattern::Any => true,
            BytePattern::Value(expected) => target == *expected,
        }
    }

    pub fn parse(pattern: &str) -> Option<BytePattern> {
        if pattern == "?" || pattern == "??" {
            Some(BytePattern::Any)
        } else if let Ok(value) = u8::from_str_radix(pattern, 16) {
            Some(BytePattern::Value(value))
        } else {
            None
        }
    }
}

impl SearchPattern for BytePattern {
    fn length(&self) -> usize {
        1
    }

    fn is_matching(&self, target: &[u8]) -> bool {
        self.matches_byte(target[0])
    }
}

#[derive(Debug)]
pub struct ByteSequencePattern {
    bytes: Vec<BytePattern>,
}

impl ByteSequencePattern {
    pub fn parse(pattern: &str) -> Option<ByteSequencePattern> {
        pattern
            .split(" ")
            .map(BytePattern::parse)
            .collect::<Option<Vec<_>>>()
            .map(|bytes| Self { bytes })
    }
}

impl SearchPattern for ByteSequencePattern {
    fn length(&self) -> usize {
        self.bytes.len()
    }

    fn is_matching(&self, target: &[u8]) -> bool {
        self.bytes
            .iter()
            .zip(target.iter())
            .find(|(pattern, value)| !pattern.matches_byte(**value))
            .is_none()
    }
}

pub enum SignatureType {
    /// The value is an address relative to the current instruction.
    /// When resolved the absolute address the instruction pointed towards will be returned.
    RelativeAddress { inst_length: usize },

    /// The value is an offset within a struct.
    /// (Offsets are assumed to be u32)
    Offset,

    /// Resolve the absolute address of the beginning of the target pattern
    Pattern,
}

/// A signature which leads to an offset or address
/// based on a sequence of instructions.
pub struct Signature {
    pub debug_name: String,
    pub pattern: Box<dyn SearchPattern>,
    pub offset: u64,
    pub value_type: SignatureType,
}

impl Signature {
    /// Create a new relative address signature from a byte sequence pattern.
    /// Note: If the pattern is invalid this will panic!
    pub fn relative_address(
        debug_name: impl Into<String>,
        pattern: &str,
        offset: u64,
        inst_length: usize,
    ) -> Self {
        let pattern = Box::new(ByteSequencePattern::parse(pattern).expect("to be a valid pattern"));

        Self {
            debug_name: debug_name.into(),
            pattern,
            offset,
            value_type: SignatureType::RelativeAddress { inst_length },
        }
    }

    pub fn offset(debug_name: impl Into<String>, pattern: &str, offset: u64) -> Self {
        let pattern = Box::new(ByteSequencePattern::parse(pattern).expect("to be a valid pattern"));

        Self {
            debug_name: debug_name.into(),
            pattern,
            offset,
            value_type: SignatureType::Offset,
        }
    }

    pub fn pattern(debug_name: impl Into<String>, pattern: &str) -> Self {
        let pattern = Box::new(ByteSequencePattern::parse(pattern).expect("to be a valid pattern"));

        Self {
            debug_name: debug_name.into(),
            pattern,
            offset: 0x00,
            value_type: SignatureType::Pattern,
        }
    }
}
