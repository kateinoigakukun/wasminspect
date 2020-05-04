#![feature(link_args)]

use std::ffi;

extern "C" {
    fn swift_demangle(
        mangledName: *const u8,
        mangledNameLength: usize,
        outputBuffer: *mut u8,
        outputBufferSize: *mut usize,
        flags: u32,
    ) -> *const i8;
}

#[derive(Debug)]
pub enum DemangleError {
    Utf8Error(std::str::Utf8Error),
    Null,
}

impl std::error::Error for DemangleError {}

impl std::fmt::Display for DemangleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DemangleError::Utf8Error(err) => {
                write!(f, "Error while interpolating C string: {:?}", err)
            }
            DemangleError::Null => write!(f, "swift_demangle returns null"),
        }
    }
}

pub fn demangle(symbol: &str) -> Result<&str, DemangleError> {
    unsafe {
        let demangled = swift_demangle(
            symbol.as_ptr(),
            symbol.len(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        );
        if demangled == std::ptr::null() {
            Err(DemangleError::Null)
        } else {
            ffi::CStr::from_ptr(demangled)
               .to_str()
               .map_err(|e| DemangleError::Utf8Error(e))
        }
    }
}

#[test]
fn test_demangle() {
    let input = "$sSi";
    assert_eq!(demangle(input).unwrap(), "Swift.Int");
}
