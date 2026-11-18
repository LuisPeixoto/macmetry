//! The System Management Controller: temperatures, fans and power.
//!
//! The SMC is read through the `AppleSMC` user client with a fixed 80-byte request. **macmetry only ever sends the
//! three read commands** (key info, read key, key at index); the write command does not exist in this module, so no
//! code path can change what the hardware is doing.

use crate::error::{Result, SysError};
use crate::iokit::{self, Connection};

/// `kSMCHandleYPCEvent`, the one selector the user client understands.
const SELECTOR: u32 = 2;
const RESULT_OK: u8 = 0;

/// The commands this module can send. There is deliberately no write command.
#[derive(Clone, Copy)]
enum Command {
    ReadKey = 5,
    ReadIndex = 8,
    GetKeyInfo = 9,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Version {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PLimit {
    version: u16,
    length: u16,
    cpu: u32,
    gpu: u32,
    mem: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct KeyInfo {
    size: u32,
    data_type: u32,
    attributes: u8,
}

/// The request and reply structure shared with the SMC driver.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct KeyData {
    key: u32,
    version: Version,
    p_limit: PLimit,
    info: KeyInfo,
    result: u8,
    status: u8,
    command: u8,
    index: u32,
    bytes: [u8; 32],
}

const _: () = assert!(std::mem::size_of::<KeyData>() == 80, "the SMC request is 80 bytes");

/// A four-character key as the number the SMC uses.
pub fn key_code(key: &str) -> Option<u32> {
    let b = key.as_bytes();
    (b.len() == 4).then(|| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn key_name(code: u32) -> String {
    String::from_utf8_lossy(&code.to_be_bytes()).into_owned()
}

/// The kind and size of a key's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyType {
    pub size: u32,
    pub data_type: [u8; 4],
}

impl KeyType {
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.data_type).into_owned()
    }
}

/// Decodes a value into a number. Handles the unsigned and signed integers, `flt ` and the family of fixed-point
/// types named `spXY` (signed) and `fpXY` (unsigned), where `Y` is the number of fractional bits in hex.
pub fn decode(data_type: &[u8; 4], bytes: &[u8]) -> Option<f64> {
    match data_type {
        b"ui8 " => bytes.first().map(|&b| f64::from(b)),
        b"si8 " => bytes.first().map(|&b| f64::from(b as i8)),
        b"ui16" => bytes.get(..2).map(|b| f64::from(u16::from_be_bytes([b[0], b[1]]))),
        b"si16" => bytes.get(..2).map(|b| f64::from(i16::from_be_bytes([b[0], b[1]]))),
        b"ui32" => bytes.get(..4).map(|b| f64::from(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))),
        b"si32" => bytes.get(..4).map(|b| f64::from(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))),
        // The SMC is little-endian for floats on every Mac that has them.
        b"flt " => {
            bytes.get(..4).map(|b| f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))).filter(|f| f.is_finite())
        }
        [b's', b'p', int, frac] | [b'f', b'p', int, frac] if int.is_ascii_hexdigit() && frac.is_ascii_hexdigit() => {
            let fraction_bits = (*frac as char).to_digit(16)?;
            let raw = bytes.get(..2).map(|b| u16::from_be_bytes([b[0], b[1]]))?;
            let value = if data_type[0] == b's' { f64::from(raw as i16) } else { f64::from(raw) };
            Some(value / f64::from(1u32 << fraction_bits))
        }
        _ => None,
    }
}

/// An open connection to the SMC.
pub struct Smc {
    connection: Connection,
}

impl Smc {
    /// Opens the SMC. [`SysError::Unsupported`] when the Mac (or a virtual machine) has none.
    pub fn open() -> Result<Self> {
        let service = iokit::services("AppleSMC")?.into_iter().next().ok_or(SysError::Unsupported("The SMC"))?;
        Ok(Self { connection: service.open(0)? })
    }

    fn call(&self, request: KeyData) -> Result<KeyData> {
        // SAFETY: `KeyData` is `repr(C)` plain integers, so viewing it as bytes is valid.
        let input = unsafe {
            std::slice::from_raw_parts((&request as *const KeyData).cast::<u8>(), std::mem::size_of::<KeyData>())
        };
        let mut reply = KeyData::default();
        // SAFETY: as above, and every bit pattern is a valid `KeyData`.
        let output = unsafe {
            std::slice::from_raw_parts_mut((&mut reply as *mut KeyData).cast::<u8>(), std::mem::size_of::<KeyData>())
        };
        let written = self.connection.call_struct(SELECTOR, input, output)?;
        if written != std::mem::size_of::<KeyData>() {
            return Err(SysError::parse("an SMC reply", format!("{written} bytes instead of 80")));
        }
        if reply.result != RESULT_OK {
            return Err(SysError::parse("an SMC reply", format!("the SMC answered {:#x}", reply.result)));
        }
        Ok(reply)
    }

    /// The type and size of a key.
    pub fn key_type(&self, key: &str) -> Result<KeyType> {
        let code = key_code(key).ok_or_else(|| SysError::parse("an SMC key", "it is not four characters"))?;
        let reply = self.call(KeyData { key: code, command: Command::GetKeyInfo as u8, ..Default::default() })?;
        Ok(KeyType { size: reply.info.size, data_type: reply.info.data_type.to_be_bytes() })
    }

    /// The raw bytes of a key, with its type.
    pub fn read_bytes(&self, key: &str) -> Result<(KeyType, Vec<u8>)> {
        let kind = self.key_type(key)?;
        let code = key_code(key).ok_or_else(|| SysError::parse("an SMC key", "it is not four characters"))?;
        let size = kind.size.min(32);
        let reply = self.call(KeyData {
            key: code,
            info: KeyInfo { size, ..Default::default() },
            command: Command::ReadKey as u8,
            ..Default::default()
        })?;
        Ok((kind, reply.bytes[..size as usize].to_vec()))
    }

    /// A key decoded as a number, when its type is one `decode` understands.
    pub fn read_number(&self, key: &str) -> Result<f64> {
        let (kind, bytes) = self.read_bytes(key)?;
        decode(&kind.data_type, &bytes)
            .ok_or_else(|| SysError::parse("an SMC value", format!("type `{}`", kind.name())))
    }

    /// How many keys the SMC has (`#KEY`).
    pub fn key_count(&self) -> Result<u32> {
        self.read_number("#KEY").map(|n| n as u32)
    }

    /// The name of the key at an index.
    pub fn key_at(&self, index: u32) -> Result<String> {
        let reply = self.call(KeyData { command: Command::ReadIndex as u8, index, ..Default::default() })?;
        Ok(key_name(reply.key))
    }

    /// Every key name. Bounded, because a broken driver could claim billions.
    pub fn keys(&self) -> Result<Vec<String>> {
        let count = self.key_count()?.min(10_000);
        Ok((0..count).filter_map(|i| self.key_at(i).ok()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_four_characters() {
        assert_eq!(key_code("TC0P"), Some(0x5443_3050));
        assert_eq!(key_name(0x5443_3050), "TC0P");
        assert_eq!(key_code("TC0"), None);
        assert_eq!(key_code("TC0PP"), None);
    }

    #[test]
    fn decodes_integers() {
        assert_eq!(decode(b"ui8 ", &[200]), Some(200.0));
        assert_eq!(decode(b"si8 ", &[0xff]), Some(-1.0));
        assert_eq!(decode(b"ui16", &[0x01, 0x02]), Some(258.0));
        assert_eq!(decode(b"ui32", &[0, 0, 1, 0]), Some(256.0));
        assert_eq!(decode(b"si16", &[0xff, 0xfe]), Some(-2.0));
    }

    #[test]
    fn decodes_fixed_point() {
        // 0x2d80 / 256 = 45.5 degrees.
        assert_eq!(decode(b"sp78", &[0x2d, 0x80]), Some(45.5));
        // Negative values are two's complement: -1.0.
        assert_eq!(decode(b"sp78", &[0xff, 0x00]), Some(-1.0));
        // 683 rpm * 4 = 2732 = 0x0aac.
        assert_eq!(decode(b"fpe2", &[0x0a, 0xac]), Some(683.0));
        assert_eq!(decode(b"fp88", &[0x01, 0x80]), Some(1.5));
        assert_eq!(decode(b"sp1e", &[0x40, 0x00]), Some(1.0));
    }

    #[test]
    fn decodes_floats_and_refuses_the_rest() {
        assert_eq!(decode(b"flt ", &48.5f32.to_le_bytes()), Some(48.5));
        assert_eq!(decode(b"flt ", &f32::NAN.to_le_bytes()), None, "a NaN is not a reading");
        assert_eq!(decode(b"ch8*", b"abcd"), None);
        assert_eq!(decode(b"{fds", &[0; 16]), None);
        assert_eq!(decode(b"ui16", &[1]), None, "too few bytes");
        assert_eq!(decode(b"spzz", &[0, 0]), None, "not a fixed-point name");
    }

    #[test]
    fn this_mac_has_an_smc_or_says_so() {
        match Smc::open() {
            Ok(smc) => {
                let count = smc.key_count().unwrap();
                assert!(count > 10, "an SMC has many keys, found {count}");
            }
            Err(e) => assert!(e.is_unsupported(), "unexpected error: {e}"),
        }
    }
}
