//! A thin layer over CoreFoundation. It owns what it creates and turns any value into a [`Value`], with limits on
//! depth and size so a hostile or corrupt tree cannot exhaust memory.

use crate::value::Value;
use std::collections::BTreeMap;
use std::ffi::{CString, c_char, c_void};

pub type CFTypeRef = *const c_void;
type CFIndex = isize;
type CFTypeID = usize;

const UTF8: u32 = 0x0800_0100;
const NUMBER_SINT64: isize = 4;
const NUMBER_FLOAT64: isize = 6;

/// Deepest nesting that is converted. Registry trees are a few levels deep.
const MAX_DEPTH: usize = 16;
/// Most items converted from one array or dictionary.
const MAX_ITEMS: usize = 100_000;
/// Longest string or data blob converted, in bytes.
const MAX_BYTES: usize = 16 << 20;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: CFTypeRef);
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;
    fn CFStringGetTypeID() -> CFTypeID;
    fn CFNumberGetTypeID() -> CFTypeID;
    fn CFBooleanGetTypeID() -> CFTypeID;
    fn CFDictionaryGetTypeID() -> CFTypeID;
    fn CFArrayGetTypeID() -> CFTypeID;
    fn CFDataGetTypeID() -> CFTypeID;
    fn CFStringGetLength(s: CFTypeRef) -> CFIndex;
    fn CFStringGetMaximumSizeForEncoding(length: CFIndex, encoding: u32) -> CFIndex;
    fn CFStringGetCString(s: CFTypeRef, buffer: *mut c_char, size: CFIndex, encoding: u32) -> u8;
    fn CFStringCreateWithCString(alloc: CFTypeRef, c: *const c_char, encoding: u32) -> CFTypeRef;
    fn CFNumberIsFloatType(n: CFTypeRef) -> u8;
    fn CFNumberGetValue(n: CFTypeRef, kind: isize, out: *mut c_void) -> u8;
    fn CFBooleanGetValue(b: CFTypeRef) -> u8;
    fn CFDictionaryGetCount(d: CFTypeRef) -> CFIndex;
    fn CFDictionaryGetKeysAndValues(d: CFTypeRef, keys: *mut CFTypeRef, values: *mut CFTypeRef);
    fn CFDictionaryGetValue(d: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn CFDictionaryCreateMutableCopy(alloc: CFTypeRef, capacity: CFIndex, d: CFTypeRef) -> CFTypeRef;
    fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        count: CFIndex,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> CFTypeRef;
    fn CFNumberCreate(alloc: CFTypeRef, kind: isize, value: *const c_void) -> CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    fn CFArrayGetCount(a: CFTypeRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(a: CFTypeRef, index: CFIndex) -> CFTypeRef;
    fn CFDataGetLength(d: CFTypeRef) -> CFIndex;
    fn CFDataGetBytePtr(d: CFTypeRef) -> *const u8;
}

/// A CoreFoundation object that this program owns and releases when dropped.
#[derive(Debug)]
pub struct Owned(CFTypeRef);

// SAFETY: CoreFoundation objects are reference counted with atomic operations, and the values this crate holds are
// immutable once created.
unsafe impl Send for Owned {}

impl Owned {
    /// Takes ownership of an object returned by a `Create` or `Copy` function. Returns `None` for null.
    ///
    /// # Safety
    /// `ptr` must be null or a valid CoreFoundation object whose retain the caller owns.
    pub unsafe fn from_create(ptr: CFTypeRef) -> Option<Self> {
        if ptr.is_null() { None } else { Some(Self(ptr)) }
    }

    /// Retains a borrowed object (one returned by a `Get` function) so it outlives its container.
    ///
    /// # Safety
    /// `ptr` must be null or a valid CoreFoundation object.
    pub unsafe fn retain(ptr: CFTypeRef) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees `ptr` is a valid object.
        Some(Self(unsafe { CFRetain(ptr) }))
    }

    /// A new CFString.
    pub fn string(text: &str) -> Option<Self> {
        let c = CString::new(text).ok()?;
        // SAFETY: `c` is a valid C string; a null allocator means the default one.
        unsafe { Self::from_create(CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), UTF8)) }
    }

    pub fn as_ptr(&self) -> CFTypeRef {
        self.0
    }

    /// Looks up a key of a dictionary and converts the entry.
    pub fn dict_value(&self, key: &str) -> Option<Value> {
        let key = Self::string(key)?;
        // SAFETY: `self` and `key` are valid objects; the returned entry is borrowed and converted before release.
        let entry = unsafe { CFDictionaryGetValue(self.0, key.0) };
        if entry.is_null() {
            return None;
        }
        // SAFETY: `entry` is a valid borrowed object that lives as long as `self`.
        Some(unsafe { to_value(entry, 0) })
    }

    /// A mutable copy of a dictionary, which some private APIs insist on.
    pub fn mutable_copy(&self) -> Option<Self> {
        // SAFETY: `self.0` is a valid dictionary; a null allocator means the default one.
        unsafe { Self::from_create(CFDictionaryCreateMutableCopy(std::ptr::null(), 0, self.0)) }
    }

    /// A dictionary whose keys are strings and whose values are integers.
    pub fn int_dictionary(entries: &[(&str, i64)]) -> Option<Self> {
        let keys: Vec<Self> = entries.iter().map(|(k, _)| Self::string(k)).collect::<Option<_>>()?;
        let values: Vec<Self> = entries
            .iter()
            .map(|(_, v)| {
                // SAFETY: `v` is a valid `i64` and the number type says so.
                unsafe { Self::from_create(CFNumberCreate(std::ptr::null(), NUMBER_SINT64, (v as *const i64).cast())) }
            })
            .collect::<Option<_>>()?;
        let key_ptrs: Vec<CFTypeRef> = keys.iter().map(Self::as_ptr).collect();
        let value_ptrs: Vec<CFTypeRef> = values.iter().map(Self::as_ptr).collect();
        // SAFETY: the arrays have `count` valid objects, which the dictionary retains; the callbacks are the
        // standard CoreFoundation ones.
        unsafe {
            Self::from_create(CFDictionaryCreate(
                std::ptr::null(),
                key_ptrs.as_ptr(),
                value_ptrs.as_ptr(),
                key_ptrs.len() as CFIndex,
                (&raw const kCFTypeDictionaryKeyCallBacks).cast(),
                (&raw const kCFTypeDictionaryValueCallBacks).cast(),
            ))
        }
    }

    /// The raw entry of a dictionary, borrowed from `self`. Null-safe: `None` when the key is absent.
    pub fn dict_ptr(&self, key: &str) -> Option<CFTypeRef> {
        let key = Self::string(key)?;
        // SAFETY: both objects are valid; the entry is borrowed from the dictionary and the caller keeps `self`.
        let entry = unsafe { CFDictionaryGetValue(self.0, key.0) };
        (!entry.is_null()).then_some(entry)
    }

    /// The items of an array, as borrowed pointers that stay valid while `self` is alive. Empty for a non-array.
    pub fn array_items(&self) -> Vec<CFTypeRef> {
        // SAFETY: `self.0` is a valid object, and the array calls only run after the type check.
        unsafe {
            if CFGetTypeID(self.0) != CFArrayGetTypeID() {
                return Vec::new();
            }
            let count = usize::try_from(CFArrayGetCount(self.0)).unwrap_or(0).min(MAX_ITEMS);
            (0..count).map(|i| CFArrayGetValueAtIndex(self.0, i as CFIndex)).collect()
        }
    }

    /// Converts the whole tree.
    pub fn to_value(&self) -> Value {
        // SAFETY: `self.0` is a valid owned object.
        unsafe { to_value(self.0, 0) }
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: this value owns one retain of a valid object.
        unsafe { CFRelease(self.0) }
    }
}

/// # Safety
/// `cf` must be a valid CoreFoundation object for the duration of the call.
unsafe fn to_value(cf: CFTypeRef, depth: usize) -> Value {
    if cf.is_null() || depth > MAX_DEPTH {
        return Value::Null;
    }
    // SAFETY: `cf` is valid per this function's contract; every call below receives an object of the checked type.
    unsafe {
        let id = CFGetTypeID(cf);
        if id == CFStringGetTypeID() {
            string_value(cf).map(Value::Str).unwrap_or_default()
        } else if id == CFNumberGetTypeID() {
            if CFNumberIsFloatType(cf) != 0 {
                let mut out = 0f64;
                CFNumberGetValue(cf, NUMBER_FLOAT64, (&mut out as *mut f64).cast());
                Value::Float(out)
            } else {
                let mut out = 0i64;
                CFNumberGetValue(cf, NUMBER_SINT64, (&mut out as *mut i64).cast());
                Value::Int(out)
            }
        } else if id == CFBooleanGetTypeID() {
            Value::Bool(CFBooleanGetValue(cf) != 0)
        } else if id == CFDataGetTypeID() {
            let len = usize::try_from(CFDataGetLength(cf)).unwrap_or(0).min(MAX_BYTES);
            let ptr = CFDataGetBytePtr(cf);
            if ptr.is_null() {
                Value::Data(Vec::new())
            } else {
                Value::Data(std::slice::from_raw_parts(ptr, len).to_vec())
            }
        } else if id == CFArrayGetTypeID() {
            let count = usize::try_from(CFArrayGetCount(cf)).unwrap_or(0).min(MAX_ITEMS);
            Value::Array((0..count).map(|i| to_value(CFArrayGetValueAtIndex(cf, i as CFIndex), depth + 1)).collect())
        } else if id == CFDictionaryGetTypeID() {
            let count = usize::try_from(CFDictionaryGetCount(cf)).unwrap_or(0).min(MAX_ITEMS);
            let mut keys = vec![std::ptr::null(); count];
            let mut values = vec![std::ptr::null(); count];
            CFDictionaryGetKeysAndValues(cf, keys.as_mut_ptr(), values.as_mut_ptr());
            let mut map = BTreeMap::new();
            for (key, value) in keys.into_iter().zip(values) {
                if !key.is_null()
                    && CFGetTypeID(key) == CFStringGetTypeID()
                    && let Some(name) = string_value(key)
                {
                    map.insert(name, to_value(value, depth + 1));
                }
            }
            Value::Dict(map)
        } else {
            Value::Null
        }
    }
}

/// Reads a CFString that someone else owns, for example one returned by a `Get` function.
///
/// # Safety
/// `s` must be null or a valid CFString for the duration of the call.
pub unsafe fn string_from(s: CFTypeRef) -> Option<String> {
    // SAFETY: the caller guarantees `s` is null or a valid CFString, and the type is checked before it is read.
    unsafe { if s.is_null() || CFGetTypeID(s) != CFStringGetTypeID() { None } else { string_value(s) } }
}

/// # Safety
/// `s` must be a valid CFString.
unsafe fn string_value(s: CFTypeRef) -> Option<String> {
    // SAFETY: `s` is a valid CFString per this function's contract.
    unsafe {
        let len = CFStringGetLength(s);
        let max = CFStringGetMaximumSizeForEncoding(len, UTF8);
        let max = usize::try_from(max).ok()?;
        if max > MAX_BYTES {
            return None;
        }
        let mut buf = vec![0u8; max + 1];
        if CFStringGetCString(s, buf.as_mut_ptr().cast(), buf.len() as CFIndex, UTF8) == 0 {
            return None;
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        Some(String::from_utf8_lossy(&buf[..end]).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_round_trips() {
        let s = Owned::string("héllo 🌡").unwrap();
        assert_eq!(s.to_value(), Value::Str("héllo 🌡".into()));
    }

    #[test]
    fn a_null_pointer_is_not_owned() {
        // SAFETY: null is explicitly allowed.
        assert!(unsafe { Owned::from_create(std::ptr::null()) }.is_none());
        // SAFETY: null is explicitly allowed.
        assert!(unsafe { Owned::retain(std::ptr::null()) }.is_none());
    }

    #[test]
    fn an_interior_nul_is_rejected() {
        assert!(Owned::string("a\0b").is_none());
    }
}
