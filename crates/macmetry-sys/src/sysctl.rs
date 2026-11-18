//! Safe reads of `sysctl` values.

use crate::error::{Result, SysError};
use std::ffi::CString;

fn name_c(name: &str) -> Result<CString> {
    CString::new(name).map_err(|_| SysError::parse("a sysctl name", "it contains a NUL byte"))
}

/// The raw bytes of a value.
pub fn raw(name: &str) -> Result<Vec<u8>> {
    let c = name_c(name)?;
    let mut size: libc::size_t = 0;
    // SAFETY: `c` is a valid C string; a null output buffer asks only for the size.
    let rc = unsafe { libc::sysctlbyname(c.as_ptr(), std::ptr::null_mut(), &mut size, std::ptr::null_mut(), 0) };
    if rc != 0 {
        return Err(SysError::os("sysctlbyname"));
    }
    // The value can grow between the two calls, so leave a little room and use the length that comes back.
    let mut buf = vec![0u8; size + 16];
    let mut len: libc::size_t = buf.len();
    // SAFETY: `buf` has `len` writable bytes and `c` is a valid C string.
    let rc = unsafe { libc::sysctlbyname(c.as_ptr(), buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) };
    if rc != 0 {
        return Err(SysError::os("sysctlbyname"));
    }
    buf.truncate(len);
    Ok(buf)
}

/// A text value, without the trailing NUL.
pub fn string(name: &str) -> Result<String> {
    let bytes = raw(name)?;
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

/// A number stored as 4 or 8 bytes.
pub fn number(name: &str) -> Result<u64> {
    let bytes = raw(name)?;
    match bytes.len() {
        4 => Ok(u64::from(u32::from_ne_bytes(bytes[..4].try_into().expect("length checked")))),
        8 => Ok(u64::from_ne_bytes(bytes[..8].try_into().expect("length checked"))),
        n => Err(SysError::parse("a sysctl number", format!("`{name}` has {n} bytes"))),
    }
}

/// A fixed-size structure, such as `xsw_usage` or `timeval`.
pub fn structure<T: Copy>(name: &str) -> Result<T> {
    let bytes = raw(name)?;
    if bytes.len() != std::mem::size_of::<T>() {
        return Err(SysError::parse(
            "a sysctl structure",
            format!("`{name}` has {} bytes, expected {}", bytes.len(), std::mem::size_of::<T>()),
        ));
    }
    // SAFETY: the length matches `T`, `T` is `Copy` (plain data for every use in this crate) and the read is
    // unaligned-safe.
    Ok(unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<T>()) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_text_and_numbers() {
        assert!(!string("kern.osversion").unwrap().is_empty());
        assert!(number("hw.memsize").unwrap() > 1 << 28);
        assert!(number("hw.ncpu").unwrap() >= 1);
    }

    #[test]
    fn an_unknown_name_is_an_error_not_a_panic() {
        assert!(raw("macmetry.does.not.exist").is_err());
        assert!(raw("bad\0name").is_err());
    }

    #[test]
    fn a_structure_must_have_the_expected_size() {
        #[derive(Clone, Copy)]
        #[allow(dead_code)]
        struct Tiny(u8);
        assert!(structure::<Tiny>("hw.memsize").is_err());
        let boot: libc::timeval = structure("kern.boottime").unwrap();
        assert!(boot.tv_sec > 0);
    }
}
