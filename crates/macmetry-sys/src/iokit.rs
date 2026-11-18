//! Reading the IORegistry and talking to IOKit user clients.

use crate::cf::{CFTypeRef, Owned};
use crate::error::{Result, SysError};
use crate::value::Value;
use std::ffi::{CString, c_char, c_void};

type MachPort = u32;
type IoObject = u32;
type KernReturn = i32;

const KERN_SUCCESS: KernReturn = 0;
/// `MACH_PORT_NULL` selects the default main port.
const DEFAULT_PORT: MachPort = 0;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> CFTypeRef;
    fn IOServiceGetMatchingServices(main_port: MachPort, matching: CFTypeRef, existing: *mut IoObject) -> KernReturn;
    fn IOIteratorNext(iterator: IoObject) -> IoObject;
    fn IOObjectRelease(object: IoObject) -> KernReturn;
    fn IORegistryEntryCreateCFProperties(
        entry: IoObject,
        properties: *mut CFTypeRef,
        allocator: CFTypeRef,
        options: u32,
    ) -> KernReturn;
    fn IORegistryEntryGetName(entry: IoObject, name: *mut c_char) -> KernReturn;
    fn IORegistryEntryGetChildIterator(entry: IoObject, plane: *const c_char, iterator: *mut IoObject) -> KernReturn;
    fn IORegistryEntryGetParentEntry(entry: IoObject, plane: *const c_char, parent: *mut IoObject) -> KernReturn;
    fn IORegistryEntryGetRegistryEntryID(entry: IoObject, id: *mut u64) -> KernReturn;
    fn IOServiceOpen(service: IoObject, owning_task: MachPort, kind: u32, connect: *mut IoObject) -> KernReturn;
    fn IOServiceClose(connect: IoObject) -> KernReturn;
    fn IOConnectCallStructMethod(
        connect: IoObject,
        selector: u32,
        input: *const c_void,
        input_size: usize,
        output: *mut c_void,
        output_size: *mut usize,
    ) -> KernReturn;
}

/// A registry object, released when dropped.
#[derive(Debug)]
pub struct Service(IoObject);

impl Service {
    /// The entry's name in the registry.
    pub fn name(&self) -> String {
        let mut buf = [0 as c_char; 128];
        // SAFETY: `buf` is the 128 bytes of an `io_name_t`, which is what the call writes.
        let rc = unsafe { IORegistryEntryGetName(self.0, buf.as_mut_ptr()) };
        if rc != KERN_SUCCESS {
            return String::new();
        }
        let bytes: Vec<u8> = buf.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// All properties of the entry as a dictionary.
    pub fn properties(&self) -> Result<Value> {
        let mut props: CFTypeRef = std::ptr::null();
        // SAFETY: `props` is a valid out pointer; a null allocator means the default one.
        let rc = unsafe { IORegistryEntryCreateCFProperties(self.0, &mut props, std::ptr::null(), 0) };
        if rc != KERN_SUCCESS {
            return Err(SysError::kern("IORegistryEntryCreateCFProperties", rc));
        }
        // SAFETY: on success `props` is a new dictionary the caller owns.
        let owned = unsafe { Owned::from_create(props) }
            .ok_or_else(|| SysError::parse("registry properties", "the entry returned no properties"))?;
        Ok(owned.to_value())
    }

    /// Entries below this one in the service plane.
    pub fn children(&self) -> Vec<Service> {
        let plane = c"IOService";
        let mut iter: IoObject = 0;
        // SAFETY: `plane` is a valid C string and `iter` a valid out pointer.
        let rc = unsafe { IORegistryEntryGetChildIterator(self.0, plane.as_ptr(), &mut iter) };
        if rc != KERN_SUCCESS {
            return Vec::new();
        }
        drain(Iterator_(iter))
    }

    /// The entry that owns this one in the service plane.
    pub fn parent(&self) -> Option<Service> {
        let mut parent: IoObject = 0;
        // SAFETY: the plane is a valid C string and `parent` a valid out pointer.
        let rc = unsafe { IORegistryEntryGetParentEntry(self.0, c"IOService".as_ptr(), &mut parent) };
        (rc == KERN_SUCCESS && parent != 0).then_some(Service(parent))
    }

    /// The number that identifies this entry for as long as the system runs.
    pub fn entry_id(&self) -> Option<u64> {
        let mut id = 0u64;
        // SAFETY: `id` is a valid out pointer.
        let rc = unsafe { IORegistryEntryGetRegistryEntryID(self.0, &mut id) };
        (rc == KERN_SUCCESS).then_some(id)
    }

    /// Opens a user client connection, for example to the SMC.
    pub fn open(&self, kind: u32) -> Result<Connection> {
        let mut connect: IoObject = 0;
        // SAFETY: `connect` is a valid out pointer and the task port is valid for the life of the process.
        let rc = unsafe { IOServiceOpen(self.0, crate::mach::task_self(), kind, &mut connect) };
        if rc != KERN_SUCCESS {
            return Err(SysError::kern("IOServiceOpen", rc));
        }
        Ok(Connection(connect))
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        // SAFETY: this value owns one reference to the object.
        unsafe { IOObjectRelease(self.0) };
    }
}

/// An iterator object, released when dropped.
struct Iterator_(IoObject);

impl Drop for Iterator_ {
    fn drop(&mut self) {
        // SAFETY: this value owns one reference to the iterator.
        unsafe { IOObjectRelease(self.0) };
    }
}

fn drain(iter: Iterator_) -> Vec<Service> {
    let mut out = Vec::new();
    loop {
        // SAFETY: `iter.0` is a valid iterator.
        let next = unsafe { IOIteratorNext(iter.0) };
        if next == 0 {
            break;
        }
        out.push(Service(next));
        if out.len() >= 4096 {
            break;
        }
    }
    out
}

/// Every registry entry of a class (or a subclass of it), for example `IOAccelerator` or `AppleSmartBattery`.
pub fn services(class: &str) -> Result<Vec<Service>> {
    let name = CString::new(class).map_err(|_| SysError::parse("a registry class", "it contains a NUL byte"))?;
    // SAFETY: `name` is a valid C string. The returned dictionary is consumed by the call below.
    let matching = unsafe { IOServiceMatching(name.as_ptr()) };
    if matching.is_null() {
        return Err(SysError::parse("a registry class", "the matching dictionary could not be created"));
    }
    let mut iter: IoObject = 0;
    // SAFETY: `matching` is a valid dictionary and is released by this call; `iter` is a valid out pointer.
    let rc = unsafe { IOServiceGetMatchingServices(DEFAULT_PORT, matching, &mut iter) };
    if rc != KERN_SUCCESS {
        return Err(SysError::kern("IOServiceGetMatchingServices", rc));
    }
    Ok(drain(Iterator_(iter)))
}

/// The properties of every entry of a class, with the entry's name.
pub fn properties_of(class: &str) -> Result<Vec<(String, Value)>> {
    services(class)?.into_iter().map(|s| s.properties().map(|p| (s.name(), p))).collect()
}

/// An open user client connection, closed when dropped.
#[derive(Debug)]
pub struct Connection(IoObject);

impl Connection {
    /// Calls a struct method: sends `input` and fills `output`. Returns the number of bytes written to `output`.
    pub fn call_struct(&self, selector: u32, input: &[u8], output: &mut [u8]) -> Result<usize> {
        let mut out_len = output.len();
        // SAFETY: both buffers are valid for the sizes passed, and the connection is open.
        let rc = unsafe {
            IOConnectCallStructMethod(
                self.0,
                selector,
                input.as_ptr().cast(),
                input.len(),
                output.as_mut_ptr().cast(),
                &mut out_len,
            )
        };
        if rc != KERN_SUCCESS {
            return Err(SysError::kern("IOConnectCallStructMethod", rc));
        }
        Ok(out_len)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // SAFETY: this value owns the open connection.
        unsafe { IOServiceClose(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_platform_expert_always_exists() {
        let found = properties_of("IOPlatformExpertDevice").unwrap();
        assert_eq!(found.len(), 1);
        let (_, props) = &found[0];
        assert!(props.get("model").is_some(), "the platform expert has a model");
    }

    #[test]
    fn an_unknown_class_has_no_entries() {
        assert!(services("MacmetryNoSuchClass").unwrap().is_empty());
    }

    #[test]
    fn a_class_with_a_nul_byte_is_rejected() {
        assert!(services("a\0b").is_err());
    }

    #[test]
    fn entries_have_names_and_children_are_listable() {
        let root = services("IOPlatformExpertDevice").unwrap().remove(0);
        assert!(!root.name().is_empty());
        let _ = root.children();
    }

    #[test]
    fn a_child_knows_its_parent() {
        let root = services("IOPlatformExpertDevice").unwrap().remove(0);
        let root_id = root.entry_id().expect("entries have ids");
        let child = root.children().into_iter().next().expect("the platform expert has children");
        assert_eq!(child.parent().and_then(|p| p.entry_id()), Some(root_id));
    }
}
