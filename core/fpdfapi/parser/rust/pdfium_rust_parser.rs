//! Narrow parser primitives that preserve the retained C++ parser's behavior.

fn read_big_endian_var_int(input: &[u8]) -> u32 {
    input.iter().fold(0_u32, |value, byte| value.wrapping_mul(256).wrapping_add(u32::from(*byte)))
}

fn cross_ref_object_type(type_code: u32) -> Option<u8> {
    match type_code {
        0..=2 => Some(type_code as u8),
        _ => None,
    }
}

fn cross_ref_entry_type(has_type_field: bool, type_code: u32) -> Option<u8> {
    if has_type_field {
        cross_ref_object_type(type_code)
    } else {
        Some(1)
    }
}

fn cross_ref_entry_action(
    type_code: u8,
    normal_offset_fits: bool,
    generation: u32,
    archive_object_valid: bool,
) -> Option<u8> {
    const SKIP: u8 = 0;
    const FREE: u8 = 1;
    const NORMAL: u8 = 2;
    const COMPRESSED: u8 = 3;
    match type_code {
        0 => Some(if generation <= u16::MAX as u32 { FREE } else { SKIP }),
        1 => Some(if normal_offset_fits && generation <= u16::MAX as u32 { NORMAL } else { SKIP }),
        2 => Some(if archive_object_valid { COMPRESSED } else { SKIP }),
        _ => None,
    }
}

fn cross_ref_index_pair(start: i32, count: i32) -> Option<(u32, u32)> {
    if start < 0 || count <= 0 {
        return None;
    }
    Some((start as u32, count as u32))
}

/// Reads a variable-width big-endian cross-reference field.
///
/// # Safety
///
/// When `len` is non-zero, `data` must point to `len` readable bytes. `output`
/// must point to one writable `u32` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_read_big_endian_var_int(
    data: *const u8,
    len: usize,
    output: *mut u32,
) -> bool {
    if output.is_null() || (len != 0 && data.is_null()) {
        return false;
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    // SAFETY: The caller guarantees a readable input span whenever non-empty;
    // empty spans use a non-null dangling pointer and are never dereferenced.
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    // SAFETY: The checked output pointer refers to one writable result.
    unsafe {
        *output = read_big_endian_var_int(input);
    }
    true
}

/// Validates a cross-reference stream object type code.
///
/// # Safety
///
/// `output` must point to one writable `u8` value. Invalid codes leave it
/// unchanged and return `false`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_object_type(
    type_code: u32,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(result) = cross_ref_object_type(type_code) else {
        return false;
    };
    // SAFETY: The checked output pointer refers to one writable type code.
    unsafe {
        *output = result;
    }
    true
}

/// Selects the effective cross-reference entry type, including the ISO default.
///
/// # Safety
///
/// `output` must point to one writable `u8` value. Invalid explicit codes
/// leave it unchanged and return `false`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_entry_type(
    has_type_field: bool,
    type_code: u32,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(result) = cross_ref_entry_type(has_type_field, type_code) else {
        return false;
    };
    // SAFETY: The checked output pointer refers to one writable type code.
    unsafe {
        *output = result;
    }
    true
}

/// Plans the observable action for one decoded cross-reference entry.
///
/// # Safety
///
/// `output` must point to one writable `u8` value. Unsupported type codes
/// leave it unchanged and return `false`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_entry_action(
    type_code: u8,
    normal_offset_fits: bool,
    generation: u32,
    archive_object_valid: bool,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(action) =
        cross_ref_entry_action(type_code, normal_offset_fits, generation, archive_object_valid)
    else {
        return false;
    };
    // SAFETY: The checked output pointer refers to one writable action byte.
    unsafe {
        *output = action;
    }
    true
}

/// Validates and normalizes one `/Index` start/count pair.
///
/// # Safety
///
/// Both outputs must point to writable `u32` values. Invalid pairs leave both
/// values unchanged and return `false`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_index_pair(
    start: i32,
    count: i32,
    output_start: *mut u32,
    output_count: *mut u32,
) -> bool {
    if output_start.is_null() || output_count.is_null() {
        return false;
    }
    let Some((planned_start, planned_count)) = cross_ref_index_pair(start, count) else {
        return false;
    };
    // SAFETY: Both checked output pointers refer to one writable scalar.
    unsafe {
        *output_start = planned_start;
        *output_count = planned_count;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_endian_var_int_should_preserve_all_widths_and_wraparound() {
        assert_eq!(0, read_big_endian_var_int(&[]));
        assert_eq!(0x7f, read_big_endian_var_int(&[0x7f]));
        assert_eq!(0x1234, read_big_endian_var_int(&[0x12, 0x34]));
        assert_eq!(0x1234_5678, read_big_endian_var_int(&[0x12, 0x34, 0x56, 0x78]));
        assert_eq!(0x3456_789a, read_big_endian_var_int(&[0x12, 0x34, 0x56, 0x78, 0x9a]));
    }

    #[test]
    fn big_endian_var_int_ffi_should_reject_null_input_without_output_mutation() {
        let mut output = 0xfeed_beef_u32;
        // SAFETY: The nonzero length with a null input is deliberately
        // rejected before the valid output is written.
        assert!(!unsafe { pdfium_rust_read_big_endian_var_int(core::ptr::null(), 1, &mut output) });
        assert_eq!(0xfeed_beef, output);
    }

    #[test]
    fn cross_ref_object_type_should_accept_only_defined_codes() {
        assert_eq!(Some(0), cross_ref_object_type(0));
        assert_eq!(Some(1), cross_ref_object_type(1));
        assert_eq!(Some(2), cross_ref_object_type(2));
        assert_eq!(None, cross_ref_object_type(3));
        assert_eq!(None, cross_ref_object_type(u32::MAX));
    }

    #[test]
    fn cross_ref_object_type_ffi_should_reject_invalid_codes_without_mutation() {
        let mut output = 0xa5_u8;
        // SAFETY: The output is live; an invalid type code is rejected before
        // it is written.
        assert!(!unsafe { pdfium_rust_cross_ref_object_type(3, &mut output) });
        assert_eq!(0xa5, output);
    }

    #[test]
    fn cross_ref_entry_type_should_apply_the_iso_default_only_when_missing() {
        assert_eq!(Some(1), cross_ref_entry_type(false, u32::MAX));
        assert_eq!(Some(0), cross_ref_entry_type(true, 0));
        assert_eq!(None, cross_ref_entry_type(true, 3));
    }

    #[test]
    fn cross_ref_entry_action_should_preserve_field_range_rejections() {
        assert_eq!(Some(1), cross_ref_entry_action(0, true, u16::MAX as u32, false));
        assert_eq!(Some(0), cross_ref_entry_action(0, true, u16::MAX as u32 + 1, false));
        assert_eq!(Some(2), cross_ref_entry_action(1, true, 0, false));
        assert_eq!(Some(0), cross_ref_entry_action(1, false, 0, false));
        assert_eq!(Some(3), cross_ref_entry_action(2, false, 0, true));
        assert_eq!(Some(0), cross_ref_entry_action(2, false, 0, false));
        assert_eq!(None, cross_ref_entry_action(3, true, 0, true));
    }

    #[test]
    fn cross_ref_index_pair_should_reject_negative_and_empty_pairs() {
        assert_eq!(Some((0, 1)), cross_ref_index_pair(0, 1));
        assert_eq!(
            Some((i32::MAX as u32, i32::MAX as u32)),
            cross_ref_index_pair(i32::MAX, i32::MAX)
        );
        assert_eq!(None, cross_ref_index_pair(-1, 1));
        assert_eq!(None, cross_ref_index_pair(0, 0));
    }
}
