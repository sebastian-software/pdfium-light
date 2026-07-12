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

fn cross_ref_segment_range(
    segment_index: u32,
    object_count: u32,
    entry_width: u32,
    data_len: u64,
) -> Option<(u64, u64)> {
    let offset = u64::from(segment_index).checked_mul(u64::from(entry_width))?;
    let len = u64::from(object_count).checked_mul(u64::from(entry_width))?;
    let end = offset.checked_add(len)?;
    (end <= data_len).then_some((offset, len))
}

fn cross_ref_field_width(value: i32) -> u32 {
    value as u32
}

fn read_cross_ref_entry(input: &[u8], field_widths: [u32; 3]) -> Option<[u32; 3]> {
    let mut offset = 0_usize;
    let mut fields = [0_u32; 3];
    for (field, width) in fields.iter_mut().zip(field_widths) {
        let width = usize::try_from(width).ok()?;
        let end = offset.checked_add(width)?;
        *field = read_big_endian_var_int(input.get(offset..end)?);
        offset = end;
    }
    Some(fields)
}

fn is_pdf_whitespace(byte: u8) -> bool {
    matches!(byte, 0 | b'\t' | b'\n' | 0x0c | b'\r' | b' ' | 0x80 | 0xff)
}

fn is_pdf_delimiter(byte: u8) -> bool {
    matches!(byte, b'%' | b'(' | b')' | b'/' | b'<' | b'>' | b'[' | b']' | b'{' | b'}')
}

fn skip_pdf_spaces_and_comments(input: &[u8], mut position: usize) -> (usize, Option<u8>) {
    loop {
        let Some(&first) = input.get(position) else {
            return (position, None);
        };
        let mut current = first;
        position += 1;

        while is_pdf_whitespace(current) {
            let Some(&next) = input.get(position) else {
                return (position, None);
            };
            current = next;
            position += 1;
        }

        if current != b'%' {
            return (position, Some(current));
        }

        loop {
            let Some(&comment_byte) = input.get(position) else {
                return (position, None);
            };
            position += 1;
            if matches!(comment_byte, b'\r' | b'\n') {
                break;
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct PdfTokenScan {
    next_position: usize,
    token_range: Option<(usize, usize)>,
}

fn scan_pdf_token(input: &[u8], position: usize) -> PdfTokenScan {
    let (mut next_position, first) = skip_pdf_spaces_and_comments(input, position);
    let Some(first) = first else {
        return PdfTokenScan { next_position, token_range: None };
    };
    let start = next_position - 1;

    if !is_pdf_delimiter(first) {
        while let Some(&byte) = input.get(next_position) {
            if is_pdf_delimiter(byte) || is_pdf_whitespace(byte) {
                break;
            }
            next_position += 1;
        }
        return PdfTokenScan { next_position, token_range: Some((start, next_position - start)) };
    }

    match first {
        b'/' => {
            while let Some(&byte) = input.get(next_position) {
                if is_pdf_delimiter(byte) || is_pdf_whitespace(byte) {
                    return PdfTokenScan {
                        next_position,
                        token_range: Some((start, next_position - start)),
                    };
                }
                next_position += 1;
            }
            PdfTokenScan { next_position, token_range: None }
        }
        b'<' => {
            let Some(&mut_current) = input.get(next_position) else {
                return PdfTokenScan {
                    next_position,
                    token_range: Some((start, next_position - start)),
                };
            };
            let mut current = mut_current;
            next_position += 1;
            if current != b'<' {
                while next_position < input.len() && current != b'>' {
                    current = input[next_position];
                    next_position += 1;
                }
            }
            PdfTokenScan { next_position, token_range: Some((start, next_position - start)) }
        }
        b'>' => {
            if input.get(next_position) == Some(&b'>') {
                next_position += 1;
            }
            PdfTokenScan { next_position, token_range: Some((start, next_position - start)) }
        }
        b'(' => {
            let mut level = 1_i32;
            while next_position < input.len() && level > 0 {
                match input[next_position] {
                    b'(' => level += 1,
                    b')' => level -= 1,
                    _ => {}
                }
                next_position += 1;
            }
            PdfTokenScan { next_position, token_range: Some((start, next_position - start)) }
        }
        _ => PdfTokenScan { next_position, token_range: Some((start, next_position - start)) },
    }
}

type CrossRefSegmentCallback = unsafe extern "C" fn(*mut core::ffi::c_void, u32) -> bool;
type CrossRefMutationCallback = unsafe extern "C" fn(*mut core::ffi::c_void, u8) -> bool;

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

/// Selects and invokes one cross-reference table mutation action.
///
/// Skip actions complete without invoking C++. Free, normal, and compressed
/// actions invoke the callback exactly once.
///
/// # Safety
///
/// `context` and `callback` must remain valid for the synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_run_cross_ref_entry_mutation(
    type_code: u8,
    normal_offset_fits: bool,
    generation: u32,
    archive_object_valid: bool,
    context: *mut core::ffi::c_void,
    callback: Option<CrossRefMutationCallback>,
) -> bool {
    if context.is_null() {
        return false;
    }
    let Some(action) =
        cross_ref_entry_action(type_code, normal_offset_fits, generation, archive_object_valid)
    else {
        return false;
    };
    if action == 0 {
        return true;
    }
    let Some(callback) = callback else {
        return false;
    };
    // SAFETY: The caller guarantees a live context and synchronous callback
    // for this single selected mutation.
    unsafe { callback(context, action) }
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

/// Plans the checked byte range for one cross-reference stream segment.
///
/// # Safety
///
/// Both outputs must point to writable `u64` values. Overflow or an
/// out-of-bounds range leaves both unchanged and returns `false`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_segment_range(
    segment_index: u32,
    object_count: u32,
    entry_width: u32,
    data_len: u64,
    output_offset: *mut u64,
    output_len: *mut u64,
) -> bool {
    if output_offset.is_null() || output_len.is_null() {
        return false;
    }
    let Some((offset, len)) =
        cross_ref_segment_range(segment_index, object_count, entry_width, data_len)
    else {
        return false;
    };
    // SAFETY: Both checked pointers refer to one writable scalar.
    unsafe {
        *output_offset = offset;
        *output_len = len;
    }
    true
}

/// Runs cross-reference entries in ascending order until C++ requests a stop.
///
/// # Safety
///
/// `context` and `callback` must remain valid for every invoked index below
/// `entry_count`. A `true` callback result requests an orderly stop.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_run_cross_ref_segment_entries(
    entry_count: u32,
    context: *mut core::ffi::c_void,
    callback: Option<CrossRefSegmentCallback>,
) -> bool {
    if context.is_null() {
        return false;
    }
    let Some(callback) = callback else {
        return false;
    };
    for entry_index in 0..entry_count {
        // SAFETY: The caller guarantees the synchronous callback/context
        // contract for the complete bounded iteration.
        if unsafe { callback(context, entry_index) } {
            break;
        }
    }
    true
}

/// Converts a signed `/W` array value using PDFium's unsigned representation.
///
/// # Safety
///
/// `output` must point to one writable `u32` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_field_width(value: i32, output: *mut u32) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller guarantees one writable scalar output.
    unsafe {
        *output = cross_ref_field_width(value);
    }
    true
}

/// Decodes the three variable-width fields of one cross-reference entry.
///
/// # Safety
///
/// When `len` is non-zero, `data` must point to `len` readable bytes. All
/// three output pointers must point to writable `u32` values. The sum of the
/// widths must fit within the borrowed input; otherwise no output is written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_read_cross_ref_entry(
    data: *const u8,
    len: usize,
    first_width: u32,
    second_width: u32,
    third_width: u32,
    output_first: *mut u32,
    output_second: *mut u32,
    output_third: *mut u32,
) -> bool {
    if output_first.is_null()
        || output_second.is_null()
        || output_third.is_null()
        || (len != 0 && data.is_null())
    {
        return false;
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    // SAFETY: The caller guarantees a readable input span whenever non-empty;
    // empty spans use a non-null dangling pointer and are never dereferenced.
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    let Some([first, second, third]) =
        read_cross_ref_entry(input, [first_width, second_width, third_width])
    else {
        return false;
    };
    // SAFETY: All checked output pointers refer to writable scalar results.
    unsafe {
        *output_first = first;
        *output_second = second;
        *output_third = third;
    }
    true
}

/// Skips PDF whitespace and complete comment lines from a borrowed input.
///
/// The returned position always reflects bytes consumed, including when no
/// parseable byte remains. A successful return additionally writes that byte.
///
/// # Safety
///
/// When `len` is non-zero, `data` must point to `len` readable bytes.
/// `output_position` must point to one writable `u32`; `output_byte` must
/// point to one writable `u8` when a parseable byte is available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_skip_pdf_spaces_and_comments(
    data: *const u8,
    len: usize,
    position: u32,
    output_position: *mut u32,
    output_byte: *mut u8,
) -> bool {
    if output_position.is_null() || (len != 0 && data.is_null()) {
        return false;
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    // SAFETY: The caller guarantees a readable input span whenever non-empty;
    // empty spans use a non-null dangling pointer and are never dereferenced.
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    let (next_position, byte) =
        skip_pdf_spaces_and_comments(input, usize::try_from(position).unwrap_or(usize::MAX));
    let Ok(next_position) = u32::try_from(next_position) else {
        return false;
    };
    // SAFETY: The checked output pointer refers to one writable position.
    unsafe {
        *output_position = next_position;
    }
    let Some(byte) = byte else {
        return false;
    };
    if output_byte.is_null() {
        return false;
    }
    // SAFETY: A successful call requires one writable byte output.
    unsafe {
        *output_byte = byte;
    }
    true
}

/// Scans one complete token using `CPDF_SimpleParser` semantics.
///
/// A successful boundary call always writes the consumed position. `has_word`
/// distinguishes an empty result from a token range. Name tokens that reach
/// end-of-input intentionally return no word, matching the retained parser.
///
/// # Safety
///
/// When `len` is non-zero, `data` must point to `len` readable bytes. Every
/// output pointer must point to one writable scalar.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_scan_pdf_token(
    data: *const u8,
    len: usize,
    position: u32,
    output_position: *mut u32,
    output_has_word: *mut bool,
    output_start: *mut u32,
    output_len: *mut u32,
) -> bool {
    if output_position.is_null()
        || output_has_word.is_null()
        || output_start.is_null()
        || output_len.is_null()
        || (len != 0 && data.is_null())
    {
        return false;
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    // SAFETY: The caller guarantees a readable input span whenever non-empty;
    // empty spans use a non-null dangling pointer and are never dereferenced.
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    let result = scan_pdf_token(input, position as usize);
    let Ok(next_position) = u32::try_from(result.next_position) else {
        return false;
    };
    let (has_word, start, token_len) = match result.token_range {
        Some((start, token_len)) => {
            let (Ok(start), Ok(token_len)) = (u32::try_from(start), u32::try_from(token_len))
            else {
                return false;
            };
            (true, start, token_len)
        }
        None => (false, 0, 0),
    };
    // SAFETY: All checked output pointers refer to writable scalar results.
    unsafe {
        *output_position = next_position;
        *output_has_word = has_word;
        *output_start = start;
        *output_len = token_len;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EntryLoopState {
        visited: [u32; 4],
        count: usize,
        stop_at: Option<u32>,
    }

    struct MutationState {
        action: u8,
        calls: usize,
    }

    unsafe extern "C" fn record_mutation(context: *mut core::ffi::c_void, action: u8) -> bool {
        // SAFETY: The test passes one live `MutationState` for the call.
        let state = unsafe { &mut *context.cast::<MutationState>() };
        state.action = action;
        state.calls += 1;
        true
    }

    unsafe extern "C" fn record_entry(context: *mut core::ffi::c_void, entry_index: u32) -> bool {
        // SAFETY: The test passes one live `EntryLoopState` for the call.
        let state = unsafe { &mut *context.cast::<EntryLoopState>() };
        state.visited[state.count] = entry_index;
        state.count += 1;
        state.stop_at == Some(entry_index)
    }

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
    fn cross_ref_mutation_should_invoke_selected_actions_and_skip_rejections() {
        let mut state = MutationState { action: 0, calls: 0 };
        // SAFETY: The context remains live for the synchronous callback.
        assert!(unsafe {
            pdfium_rust_run_cross_ref_entry_mutation(
                1,
                true,
                7,
                false,
                (&mut state as *mut MutationState).cast(),
                Some(record_mutation),
            )
        });
        assert_eq!(2, state.action);
        assert_eq!(1, state.calls);

        // SAFETY: The valid context is not read because the rejected
        // generation maps to a skip action with no callback.
        assert!(unsafe {
            pdfium_rust_run_cross_ref_entry_mutation(
                0,
                true,
                u16::MAX as u32 + 1,
                false,
                (&mut state as *mut MutationState).cast(),
                None,
            )
        });
        assert_eq!(1, state.calls);
    }

    #[test]
    fn cross_ref_mutation_should_reject_invalid_boundaries() {
        let mut state = MutationState { action: 0, calls: 0 };
        // SAFETY: An unknown type is rejected before the valid context and
        // callback are used.
        assert!(!unsafe {
            pdfium_rust_run_cross_ref_entry_mutation(
                3,
                true,
                0,
                true,
                (&mut state as *mut MutationState).cast(),
                Some(record_mutation),
            )
        });
        assert_eq!(0, state.calls);
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

    #[test]
    fn cross_ref_segment_range_should_bound_multiplication_and_data_length() {
        assert_eq!(Some((12, 8)), cross_ref_segment_range(3, 2, 4, 20));
        assert_eq!(
            Some((u64::from(u32::MAX) * 4, 4)),
            cross_ref_segment_range(u32::MAX, 1, 4, u64::MAX)
        );
        assert_eq!(None, cross_ref_segment_range(3, 2, 4, 19));
        assert_eq!(None, cross_ref_segment_range(u32::MAX, u32::MAX, u32::MAX, 1));
    }

    #[test]
    fn cross_ref_segment_loop_should_visit_in_order_and_stop() {
        let mut state = EntryLoopState { visited: [u32::MAX; 4], count: 0, stop_at: Some(1) };
        // SAFETY: The context remains live and the callback accepts every
        // generated index until it requests the expected stop.
        assert!(unsafe {
            pdfium_rust_run_cross_ref_segment_entries(
                4,
                (&mut state as *mut EntryLoopState).cast(),
                Some(record_entry),
            )
        });
        assert_eq!(2, state.count);
        assert_eq!([0, 1], state.visited[..2]);
    }

    #[test]
    fn cross_ref_field_width_should_preserve_signed_cast_behavior() {
        assert_eq!(0, cross_ref_field_width(0));
        assert_eq!(17, cross_ref_field_width(17));
        assert_eq!(u32::MAX, cross_ref_field_width(-1));
        assert_eq!(0x8000_0000, cross_ref_field_width(i32::MIN));
    }

    #[test]
    fn cross_ref_entry_should_decode_zero_width_and_wrapping_fields() {
        assert_eq!(
            Some([0, 0x1234, 0x3456_789a]),
            read_cross_ref_entry(&[0x12, 0x34, 0x12, 0x34, 0x56, 0x78, 0x9a], [0, 2, 5])
        );
        assert_eq!(None, read_cross_ref_entry(&[0x12], [1, 1, 0]));
    }

    #[test]
    fn cross_ref_entry_ffi_should_reject_short_input_without_mutation() {
        let mut first = 0xa1a1_a1a1_u32;
        let mut second = 0xb2b2_b2b2_u32;
        let mut third = 0xc3c3_c3c3_u32;
        // SAFETY: The test supplies readable input and live writable outputs.
        assert!(!unsafe {
            pdfium_rust_read_cross_ref_entry(
                [0x12].as_ptr(),
                1,
                1,
                1,
                0,
                &mut first,
                &mut second,
                &mut third,
            )
        });
        assert_eq!(0xa1a1_a1a1, first);
        assert_eq!(0xb2b2_b2b2, second);
        assert_eq!(0xc3c3_c3c3, third);
    }

    #[test]
    fn skip_spaces_and_comments_should_match_pdfium_whitespace_and_comment_rules() {
        assert_eq!((4, Some(b'a')), skip_pdf_spaces_and_comments(b" \t\0a", 0));
        assert_eq!((3, Some(b'a')), skip_pdf_spaces_and_comments(&[0x80, 0xff, b'a'], 0));
        assert_eq!((8, Some(b'b')), skip_pdf_spaces_and_comments(b"%skip\r\nb", 0));
        assert_eq!((13, Some(b'c')), skip_pdf_spaces_and_comments(b" \n%one\n%two\nc", 0));
        assert_eq!((4, None), skip_pdf_spaces_and_comments(b" \t\0\n", 0));
        assert_eq!((8, None), skip_pdf_spaces_and_comments(b"%comment", 0));
    }

    #[test]
    fn skip_spaces_and_comments_ffi_should_record_consumed_position_on_eof() {
        let input = b"%comment";
        let mut output_position = 0_u32;
        let mut output_byte = 0xa5_u8;
        // SAFETY: The test supplies readable input and writable scalar outputs.
        assert!(!unsafe {
            pdfium_rust_skip_pdf_spaces_and_comments(
                input.as_ptr(),
                input.len(),
                0,
                &mut output_position,
                &mut output_byte,
            )
        });
        assert_eq!(input.len() as u32, output_position);
        assert_eq!(0xa5, output_byte);
    }

    #[test]
    fn token_scan_should_preserve_simple_parser_token_shapes() {
        assert_eq!(PdfTokenScan { next_position: 0, token_range: None }, scan_pdf_token(b"", 0));
        assert_eq!(PdfTokenScan { next_position: 3, token_range: None }, scan_pdf_token(b"/99", 0));
        assert_eq!(
            PdfTokenScan { next_position: 3, token_range: Some((0, 3)) },
            scan_pdf_token(b"/99}", 0)
        );
        assert_eq!(
            PdfTokenScan { next_position: 13, token_range: Some((1, 12)) },
            scan_pdf_token(b" (nice (day))!", 0)
        );
        assert_eq!(
            PdfTokenScan { next_position: 2, token_range: Some((0, 2)) },
            scan_pdf_token(b"<< /Name", 0)
        );
        assert_eq!(
            PdfTokenScan { next_position: 6, token_range: Some((1, 5)) },
            scan_pdf_token(b" apple pear", 0)
        );
    }

    #[test]
    fn token_scan_ffi_should_reject_null_input_without_output_mutation() {
        let mut position = 0xa1a1_a1a1_u32;
        let mut has_word = true;
        let mut start = 0xb2b2_b2b2_u32;
        let mut len = 0xc3c3_c3c3_u32;
        // SAFETY: The nonzero length with a null input is rejected before the
        // live outputs are written.
        assert!(!unsafe {
            pdfium_rust_scan_pdf_token(
                core::ptr::null(),
                1,
                0,
                &mut position,
                &mut has_word,
                &mut start,
                &mut len,
            )
        });
        assert_eq!(0xa1a1_a1a1, position);
        assert!(has_word);
        assert_eq!(0xb2b2_b2b2, start);
        assert_eq!(0xc3c3_c3c3, len);
    }
}
