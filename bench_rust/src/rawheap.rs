use std::hint::black_box;

// Type tags
const TAG_NULL: u8 = 0x01;
const TAG_FALSE: u8 = 0x02;
const TAG_TRUE: u8 = 0x03;
const TAG_I64: u8 = 0x04;
const TAG_F64: u8 = 0x05;
const TAG_STRING: u8 = 0x06;
const TAG_ARRAY: u8 = 0x07;
const TAG_MAP: u8 = 0x08;

/// Builds a rawheap blob by appending values to a byte buffer.
pub struct HeapBuilder {
    buf: Vec<u8>,
}

impl HeapBuilder {
    pub fn new() -> Self {
        // Reserve 4 bytes for the root offset header (filled in finish())
        let mut buf = Vec::with_capacity(4096);
        buf.extend_from_slice(&[0u8; 4]);
        HeapBuilder { buf }
    }

    pub fn write_null(&mut self) -> u32 {
        let off = self.buf.len() as u32;
        self.buf.push(TAG_NULL);
        off
    }

    pub fn write_bool(&mut self, v: bool) -> u32 {
        let off = self.buf.len() as u32;
        self.buf.push(if v { TAG_TRUE } else { TAG_FALSE });
        off
    }

    pub fn write_i64(&mut self, v: i64) -> u32 {
        let off = self.buf.len() as u32;
        self.buf.push(TAG_I64);
        self.buf.extend_from_slice(&v.to_le_bytes());
        off
    }

    pub fn write_f64(&mut self, v: f64) -> u32 {
        let off = self.buf.len() as u32;
        self.buf.push(TAG_F64);
        self.buf.extend_from_slice(&v.to_le_bytes());
        off
    }

    pub fn write_string(&mut self, s: &str) -> u32 {
        let off = self.buf.len() as u32;
        self.buf.push(TAG_STRING);
        self.buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
        self.buf.extend_from_slice(s.as_bytes());
        off
    }

    pub fn write_array(&mut self, offsets: &[u32]) -> u32 {
        let off = self.buf.len() as u32;
        self.buf.push(TAG_ARRAY);
        self.buf
            .extend_from_slice(&(offsets.len() as u32).to_le_bytes());
        for &o in offsets {
            self.buf.extend_from_slice(&o.to_le_bytes());
        }
        off
    }

    /// Write a map. `entries` is `(key_string_offset, value_offset)`.
    /// Keys are sorted by their string content for canonical ordering.
    pub fn write_map(&mut self, entries: &mut [(u32, u32)]) -> u32 {
        // Sort entries by key string content
        entries.sort_by(|a, b| {
            let ka = self.read_string_at(a.0);
            let kb = self.read_string_at(b.0);
            ka.cmp(kb)
        });
        let off = self.buf.len() as u32;
        self.buf.push(TAG_MAP);
        self.buf
            .extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for &(k, v) in entries.iter() {
            self.buf.extend_from_slice(&k.to_le_bytes());
            self.buf.extend_from_slice(&v.to_le_bytes());
        }
        off
    }

    fn read_string_at(&self, off: u32) -> &str {
        let off = off as usize;
        debug_assert_eq!(self.buf[off], TAG_STRING);
        let len = u32::from_le_bytes(self.buf[off + 1..off + 5].try_into().unwrap()) as usize;
        std::str::from_utf8(&self.buf[off + 5..off + 5 + len]).unwrap()
    }

    /// Finalize the heap, writing the root offset into the header.
    pub fn finish(mut self, root: u32) -> Vec<u8> {
        self.buf[0..4].copy_from_slice(&root.to_le_bytes());
        self.buf
    }

    /// Recursively convert a serde_json::Value into the heap, returning its offset.
    pub fn write_json_value(&mut self, v: &serde_json::Value) -> u32 {
        match v {
            serde_json::Value::Null => self.write_null(),
            serde_json::Value::Bool(b) => self.write_bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    self.write_i64(i)
                } else if let Some(f) = n.as_f64() {
                    self.write_f64(f)
                } else {
                    panic!("unsupported number: {}", n)
                }
            }
            serde_json::Value::String(s) => self.write_string(s),
            serde_json::Value::Array(arr) => {
                let offsets: Vec<u32> = arr.iter().map(|v| self.write_json_value(v)).collect();
                self.write_array(&offsets)
            }
            serde_json::Value::Object(map) => {
                let mut entries: Vec<(u32, u32)> = map
                    .iter()
                    .map(|(k, v)| {
                        let ko = self.write_string(k);
                        let vo = self.write_json_value(v);
                        (ko, vo)
                    })
                    .collect();
                self.write_map(&mut entries)
            }
        }
    }
}

/// Zero-copy reader wrapping a rawheap byte slice.
pub struct RawHeap<'a> {
    data: &'a [u8],
    root_offset: u32,
}

impl<'a> RawHeap<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let root_offset = u32::from_le_bytes(data[0..4].try_into().unwrap());
        RawHeap { data, root_offset }
    }

    pub fn root(&self) -> HeapRef<'a> {
        HeapRef {
            data: self.data,
            offset: self.root_offset as usize,
        }
    }
}

/// A typed view into the heap at a given offset.
#[derive(Clone, Copy)]
pub struct HeapRef<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> HeapRef<'a> {
    pub fn tag(&self) -> u8 {
        self.data[self.offset]
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.tag() {
            TAG_TRUE => Some(true),
            TAG_FALSE => Some(false),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        if self.tag() != TAG_I64 {
            return None;
        }
        let start = self.offset + 1;
        Some(i64::from_le_bytes(
            self.data[start..start + 8].try_into().unwrap(),
        ))
    }

    pub fn as_f64(&self) -> Option<f64> {
        if self.tag() != TAG_F64 {
            return None;
        }
        let start = self.offset + 1;
        Some(f64::from_le_bytes(
            self.data[start..start + 8].try_into().unwrap(),
        ))
    }

    pub fn as_str(&self) -> Option<&'a str> {
        if self.tag() != TAG_STRING {
            return None;
        }
        let len_start = self.offset + 1;
        let len =
            u32::from_le_bytes(self.data[len_start..len_start + 4].try_into().unwrap()) as usize;
        let str_start = len_start + 4;
        Some(std::str::from_utf8(&self.data[str_start..str_start + len]).unwrap())
    }

    pub fn array_len(&self) -> Option<usize> {
        if self.tag() != TAG_ARRAY {
            return None;
        }
        let start = self.offset + 1;
        Some(u32::from_le_bytes(self.data[start..start + 4].try_into().unwrap()) as usize)
    }

    pub fn array_get(&self, i: usize) -> Option<HeapRef<'a>> {
        if self.tag() != TAG_ARRAY {
            return None;
        }
        let count_start = self.offset + 1;
        let count =
            u32::from_le_bytes(self.data[count_start..count_start + 4].try_into().unwrap())
                as usize;
        if i >= count {
            return None;
        }
        let off_start = count_start + 4 + i * 4;
        let child_off =
            u32::from_le_bytes(self.data[off_start..off_start + 4].try_into().unwrap()) as usize;
        Some(HeapRef {
            data: self.data,
            offset: child_off,
        })
    }

    pub fn map_len(&self) -> Option<usize> {
        if self.tag() != TAG_MAP {
            return None;
        }
        let start = self.offset + 1;
        Some(u32::from_le_bytes(self.data[start..start + 4].try_into().unwrap()) as usize)
    }

    /// Look up a key in the map by linear scan of sorted keys.
    pub fn map_get(&self, key: &str) -> Option<HeapRef<'a>> {
        if self.tag() != TAG_MAP {
            return None;
        }
        let count_start = self.offset + 1;
        let count =
            u32::from_le_bytes(self.data[count_start..count_start + 4].try_into().unwrap())
                as usize;
        let entries_start = count_start + 4;
        for i in 0..count {
            let entry_off = entries_start + i * 8;
            let key_off =
                u32::from_le_bytes(self.data[entry_off..entry_off + 4].try_into().unwrap())
                    as usize;
            let key_ref = HeapRef {
                data: self.data,
                offset: key_off,
            };
            match key_ref.as_str() {
                Some(k) if k == key => {
                    let val_off = u32::from_le_bytes(
                        self.data[entry_off + 4..entry_off + 8].try_into().unwrap(),
                    ) as usize;
                    return Some(HeapRef {
                        data: self.data,
                        offset: val_off,
                    });
                }
                Some(k) if k > key => return None, // sorted — no point continuing
                _ => {}
            }
        }
        None
    }

    /// Recursively walk all values to prove accessibility (used in benchmarks).
    pub fn visit(&self) {
        match self.tag() {
            TAG_NULL => {
                black_box(());
            }
            TAG_TRUE | TAG_FALSE => {
                black_box(self.as_bool());
            }
            TAG_I64 => {
                black_box(self.as_i64());
            }
            TAG_F64 => {
                black_box(self.as_f64());
            }
            TAG_STRING => {
                black_box(self.as_str());
            }
            TAG_ARRAY => {
                let count = self.array_len().unwrap();
                for i in 0..count {
                    self.array_get(i).unwrap().visit();
                }
            }
            TAG_MAP => {
                let count_start = self.offset + 1;
                let count = u32::from_le_bytes(
                    self.data[count_start..count_start + 4].try_into().unwrap(),
                ) as usize;
                let entries_start = count_start + 4;
                for i in 0..count {
                    let entry_off = entries_start + i * 8;
                    let key_off = u32::from_le_bytes(
                        self.data[entry_off..entry_off + 4].try_into().unwrap(),
                    ) as usize;
                    let val_off = u32::from_le_bytes(
                        self.data[entry_off + 4..entry_off + 8].try_into().unwrap(),
                    ) as usize;
                    HeapRef {
                        data: self.data,
                        offset: key_off,
                    }
                    .visit();
                    HeapRef {
                        data: self.data,
                        offset: val_off,
                    }
                    .visit();
                }
            }
            other => panic!("unknown tag: 0x{:02x}", other),
        }
    }
}
