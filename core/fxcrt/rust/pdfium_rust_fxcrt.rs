// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::collections::BTreeMap;

#[derive(Default)]
pub struct ByteStringPoolIndex {
    handles: BTreeMap<Vec<u8>, usize>,
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_bytestring_pool_new() -> *mut ByteStringPoolIndex {
    Box::into_raw(Box::new(ByteStringPoolIndex::default()))
}

/// # Safety
/// `state` must be null or uniquely owned from
/// `pdfium_rust_bytestring_pool_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_bytestring_pool_destroy(state: *mut ByteStringPoolIndex) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

fn input_bytes<'a>(data: *const u8, len: usize) -> Option<&'a [u8]> {
    if len != 0 && data.is_null() {
        return None;
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    // SAFETY: The caller guarantees a readable span for nonempty input.
    Some(unsafe { core::slice::from_raw_parts(data, len) })
}

/// # Safety
/// Nonempty input must identify readable bytes; `output` must remain writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_bytestring_pool_get(
    state: *const ByteStringPoolIndex,
    data: *const u8,
    len: usize,
    output: *mut usize,
) -> bool {
    let (Some(state), Some(input), Some(output)) =
        (unsafe { state.as_ref() }, input_bytes(data, len), unsafe { output.as_mut() })
    else {
        return false;
    };
    let Some(handle) = state.handles.get(input) else {
        return false;
    };
    *output = *handle;
    true
}

/// # Safety
/// Nonempty input must identify readable bytes and `handle` must be nonzero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_bytestring_pool_insert(
    state: *mut ByteStringPoolIndex,
    data: *const u8,
    len: usize,
    handle: usize,
) -> bool {
    let (Some(state), Some(input)) = (unsafe { state.as_mut() }, input_bytes(data, len)) else {
        return false;
    };
    if handle == 0 || state.handles.contains_key(input) {
        return false;
    }
    state.handles.insert(input.to_vec(), handle);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_index_should_preserve_binary_keys_and_existing_handles() {
        let mut state = ByteStringPoolIndex::default();
        for (key, handle) in [(b"a".as_slice(), 1), (b"a\0b".as_slice(), 2)] {
            assert!(unsafe {
                pdfium_rust_bytestring_pool_insert(&mut state, key.as_ptr(), key.len(), handle)
            });
            let mut output = 0;
            assert!(unsafe {
                pdfium_rust_bytestring_pool_get(&state, key.as_ptr(), key.len(), &mut output)
            });
            assert_eq!(handle, output);
            assert!(!unsafe {
                pdfium_rust_bytestring_pool_insert(&mut state, key.as_ptr(), key.len(), handle + 10)
            });
        }
    }
}
