//! Narrow parser primitives that preserve the retained C++ parser's behavior.

fn read_big_endian_var_int(input: &[u8]) -> u32 {
    input.iter().fold(0_u32, |value, byte| value.wrapping_mul(256).wrapping_add(u32::from(*byte)))
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
}
