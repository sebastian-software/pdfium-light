// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Internal, allocation-owning implementations of PDF byte filters.
//!
//! The C++ adapter owns the ABI contract: it passes borrowed byte spans and
//! copies the returned allocation before asking Rust to free it. The codec
//! algorithms intentionally retain PDFium's historical edge-case behavior.

use std::mem::ManuallyDrop;
use std::slice;

const INVALID_OFFSET: u32 = u32::MAX;
// Keep in sync with `kMaxStreamSize` in
// core/fpdfapi/parser/fpdf_parser_decode.cpp.
const MAX_STREAM_SIZE: u32 = 20 * 1024 * 1024;

#[repr(C)]
pub struct RustCodecResult {
    data: *mut u8,
    len: usize,
    capacity: usize,
    bytes_consumed: u32,
}

impl RustCodecResult {
    fn from_bytes(bytes: Vec<u8>, bytes_consumed: u32) -> Self {
        if bytes.is_empty() {
            return Self { data: std::ptr::null_mut(), len: 0, capacity: 0, bytes_consumed };
        }

        let mut bytes = ManuallyDrop::new(bytes);
        Self {
            data: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
            bytes_consumed,
        }
    }

    fn failure() -> Self {
        Self::from_bytes(Vec::new(), INVALID_OFFSET)
    }
}

fn input_from_ffi<'a>(data: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        return &[];
    }

    // SAFETY: The C++ adapter passes a span's data pointer with its exact
    // length. It never calls this function with a null non-empty span.
    unsafe { slice::from_raw_parts(data, len) }
}

fn is_a85_whitespace(byte: u8) -> bool {
    matches!(byte, b'\r' | b'\n' | b' ' | b'\t')
}

fn a85_encode(input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(input.len() + input.len() / 4 + 8);
    let mut pos = 0;
    let mut line_length = 0;
    while pos + 3 < input.len() {
        let value =
            u32::from_be_bytes([input[pos], input[pos + 1], input[pos + 2], input[pos + 3]]);
        pos += 4;
        if value == 0 {
            output.push(b'z');
            line_length += 1;
        } else {
            let mut encoded = [0; 5];
            let mut remaining = value;
            for index in (0..5).rev() {
                encoded[index] = (remaining % 85) as u8 + 33;
                remaining /= 85;
            }
            output.extend_from_slice(&encoded);
            line_length += 5;
        }
        if line_length >= 75 {
            output.extend_from_slice(b"\r\n");
            line_length = 0;
        }
    }

    if pos < input.len() {
        let count = input.len() - pos;
        let mut value = 0;
        for (index, byte) in input[pos..].iter().enumerate() {
            value |= u32::from(*byte) << (8 * (3 - index));
        }
        let mut encoded = [0; 5];
        for index in (0..5).rev() {
            encoded[index] = (value % 85) as u8 + 33;
            value /= 85;
        }
        output.extend_from_slice(&encoded[..count + 1]);
    }

    output.extend_from_slice(b"~>");
    output
}

fn run_length_encode(input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }
    if input.len() == 1 {
        return vec![0, input[0], 128];
    }

    let mut output = Vec::with_capacity(input.len() + input.len() / 3 + 1);
    let mut run_start = 0;
    let mut run_end = 1;
    let mut x = input[run_start];
    let mut y = input[run_end];
    while run_end < input.len() {
        let max_len = usize::min(128, input.len() - run_start);
        while x == y && run_end - run_start < max_len - 1 {
            run_end += 1;
            y = input[run_end];
        }

        if x == y {
            run_end += 1;
            if run_end < input.len() {
                y = input[run_end];
            }
        }
        if run_end - run_start > 1 {
            output.push((257 - (run_end - run_start)) as u8);
            output.push(x);
            x = y;
            run_start = run_end;
            run_end += 1;
            if run_end < input.len() {
                y = input[run_end];
            }
            continue;
        }

        let mut literal = Vec::new();
        while x != y && run_end <= run_start + max_len {
            literal.push(x);
            x = y;
            run_end += 1;
            if run_end == input.len() {
                if run_end <= run_start + max_len {
                    literal.push(x);
                    run_end += 1;
                }
                break;
            }
            y = input[run_end];
        }
        output.push((run_end - run_start - 2) as u8);
        output.extend_from_slice(&literal);
        run_start = run_end - 1;
    }
    if run_start < input.len() {
        output.extend_from_slice(&[0, x]);
    }
    output.push(128);
    output
}

fn a85_decode(input: &[u8]) -> Result<(Vec<u8>, u32), ()> {
    if input.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let mut zcount = 0_u32;
    let mut legal_count = 0_usize;
    while legal_count < input.len() {
        let byte = input[legal_count];
        if byte == b'z' {
            zcount = zcount.checked_add(1).ok_or(())?;
        } else if !(b'!'..=b'u').contains(&byte) && !is_a85_whitespace(byte) {
            break;
        }
        legal_count += 1;
    }
    if legal_count == 0 {
        return Ok((Vec::new(), 0));
    }

    let non_zero_capacity = ((legal_count - zcount as usize) / 5)
        .checked_mul(4)
        .and_then(|value| value.checked_add(4))
        .ok_or(())?;
    let capacity = zcount
        .checked_mul(4)
        .and_then(|value| value.checked_add(u32::try_from(non_zero_capacity).ok()?))
        .ok_or(())?;

    let mut output = Vec::with_capacity(capacity as usize);
    let mut state = 0_usize;
    let mut value = 0_u32;
    let mut pos = 0_usize;
    while pos < input.len() {
        let byte = input[pos];
        pos += 1;
        if is_a85_whitespace(byte) {
            continue;
        }
        if byte == b'z' {
            output.extend_from_slice(&[0, 0, 0, 0]);
            state = 0;
            value = 0;
            continue;
        }
        if !(b'!'..=b'u').contains(&byte) {
            break;
        }

        value = value.wrapping_mul(85).wrapping_add(u32::from(byte - 33));
        if state < 4 {
            state += 1;
            continue;
        }
        output.extend_from_slice(&value.to_be_bytes());
        state = 0;
        value = 0;
    }
    if state != 0 {
        for _ in state..5 {
            value = value.wrapping_mul(85).wrapping_add(84);
        }
        output.extend_from_slice(&value.to_be_bytes()[..state - 1]);
    }
    if pos < input.len() && input[pos] == b'>' {
        pos += 1;
    }
    Ok((output, u32::try_from(pos).map_err(|_| ())?))
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn hex_decode(input: &[u8]) -> (Vec<u8>, u32) {
    if input.is_empty() {
        return (Vec::new(), 0);
    }

    let terminator = input.iter().position(|&byte| byte == b'>').unwrap_or(input.len());
    let mut output = Vec::with_capacity(terminator / 2 + 1);
    let mut first_nibble = None;
    for (index, &byte) in input.iter().enumerate() {
        if matches!(byte, b'\r' | b'\n' | b' ' | b'\t') {
            continue;
        }
        if byte == b'>' {
            if let Some(nibble) = first_nibble {
                output.push(nibble << 4);
            }
            return (output, (index + 1).min(u32::MAX as usize) as u32);
        }
        let Some(nibble) = hex_digit(byte) else {
            continue;
        };
        if let Some(first) = first_nibble {
            output.push((first << 4) + nibble);
            first_nibble = None;
        } else {
            first_nibble = Some(nibble);
        }
    }
    if let Some(nibble) = first_nibble {
        output.push(nibble << 4);
    }
    (output, input.len().min(u32::MAX as usize) as u32)
}

fn lzw_add_code(
    codes: &mut [u32; 5021],
    current_code: &mut u32,
    code_length: &mut u8,
    early_change: bool,
    prefix_code: u32,
    append_char: u8,
) {
    let early_change = u32::from(early_change);
    if *current_code + early_change == 4094 {
        return;
    }

    codes[*current_code as usize] = (prefix_code << 16) | u32::from(append_char);
    *current_code += 1;
    match *current_code + early_change {
        254 => *code_length = 10,
        766 => *code_length = 11,
        1790 => *code_length = 12,
        _ => {}
    }
}

fn lzw_decode_string(
    codes: &[u32; 5021],
    current_code: u32,
    mut code: u32,
    decode_stack: &mut [u8],
) -> usize {
    let mut stack_len = 0;
    while code >= 258 {
        let index = (code - 258) as usize;
        if index >= current_code as usize || stack_len >= decode_stack.len() {
            return stack_len;
        }
        let data = codes[index];
        decode_stack[stack_len] = data as u8;
        stack_len += 1;
        code = data >> 16;
    }
    if stack_len < decode_stack.len() {
        decode_stack[stack_len] = code as u8;
        stack_len += 1;
    }
    stack_len
}

fn lzw_read_code(input: &[u8], bit_pos: u32, code_length: u8) -> u32 {
    let mut byte_pos = (bit_pos / 8) as usize;
    let bit_pos = bit_pos % 8;
    let mut bit_left = code_length;
    let mut code = 0;
    if bit_pos != 0 {
        bit_left -= 8 - bit_pos as u8;
        code = (u32::from(input[byte_pos]) & ((1 << (8 - bit_pos)) - 1)) << bit_left;
        byte_pos += 1;
    }
    if bit_left < 8 {
        code |= u32::from(input[byte_pos] >> (8 - bit_left));
    } else {
        bit_left -= 8;
        code |= u32::from(input[byte_pos]) << bit_left;
        byte_pos += 1;
        if bit_left != 0 {
            code |= u32::from(input[byte_pos] >> (8 - bit_left));
        }
    }
    code
}

fn lzw_decode(input: &[u8], early_change: bool) -> Result<(Vec<u8>, u32), ()> {
    let mut output = Vec::with_capacity(512);
    let mut src_bit_pos = 0_u32;
    let mut codes = [0; 5021];
    let mut code_length = 9_u8;
    let mut current_code = 0_u32;
    let mut old_code = u32::MAX;
    let mut last_char = 0_u8;
    let input_bits = input.len().saturating_mul(8);

    while (src_bit_pos as usize).saturating_add(usize::from(code_length)) <= input_bits {
        let code = lzw_read_code(input, src_bit_pos, code_length);
        src_bit_pos += u32::from(code_length);

        if code < 256 {
            if output.len() >= u32::MAX as usize {
                return Err(());
            }
            output.push(code as u8);
            last_char = code as u8;
            if old_code != u32::MAX {
                lzw_add_code(
                    &mut codes,
                    &mut current_code,
                    &mut code_length,
                    early_change,
                    old_code,
                    last_char,
                );
            }
            old_code = code;
            continue;
        }
        if code == 256 {
            code_length = 9;
            current_code = 0;
            old_code = u32::MAX;
            continue;
        }
        if code == 257 {
            break;
        }
        if old_code == u32::MAX {
            return Err(());
        }

        let mut decode_stack = [0; 4000];
        let stack_len = if code - 258 >= current_code {
            decode_stack[0] = last_char;
            1 + lzw_decode_string(&codes, current_code, old_code, &mut decode_stack[1..])
        } else {
            lzw_decode_string(&codes, current_code, code, &mut decode_stack)
        };
        let required_size = output.len().checked_add(stack_len).ok_or(())?;
        if required_size > u32::MAX as usize {
            return Err(());
        }
        output.extend(decode_stack[..stack_len].iter().rev());
        last_char = decode_stack[stack_len - 1];
        if old_code >= 258 && old_code - 258 >= current_code {
            break;
        }
        lzw_add_code(
            &mut codes,
            &mut current_code,
            &mut code_length,
            early_change,
            old_code,
            last_char,
        );
        old_code = code;
    }

    if output.is_empty() {
        return Err(());
    }
    Ok((output, src_bit_pos.saturating_add(7) / 8))
}

fn run_length_decode(input: &[u8]) -> Result<(Vec<u8>, u32), ()> {
    let mut dest_size = 0_u32;
    let mut pos = 0_usize;
    while pos < input.len() {
        let control = input[pos];
        if control == 128 {
            break;
        }
        let previous_size = dest_size;
        if control < 128 {
            dest_size = dest_size.wrapping_add(u32::from(control) + 1);
            if dest_size < previous_size {
                return Err(());
            }
            pos += usize::from(control) + 2;
        } else {
            dest_size = dest_size.wrapping_add(257 - u32::from(control));
            if dest_size < previous_size {
                return Err(());
            }
            pos += 2;
        }
    }
    if dest_size >= MAX_STREAM_SIZE {
        return Err(());
    }

    let mut output = vec![0; dest_size as usize];
    pos = 0;
    let mut dest_count = 0_usize;
    while pos < input.len() {
        let control = input[pos];
        if control == 128 {
            break;
        }
        if control < 128 {
            let requested = usize::from(control) + 1;
            let available = input.len() - pos - 1;
            let copied = requested.min(available);
            output[dest_count..dest_count + copied]
                .copy_from_slice(&input[pos + 1..pos + 1 + copied]);
            dest_count += requested;
            pos += usize::from(control) + 2;
        } else {
            let fill = input.get(pos + 1).copied().unwrap_or(0);
            let count = 257 - usize::from(control);
            output[dest_count..dest_count + count].fill(fill);
            dest_count += count;
            pos += 2;
        }
    }
    let consumed = usize::min(pos + 1, input.len());
    Ok((output, u32::try_from(consumed).map_err(|_| ())?))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_a85_encode(data: *const u8, len: usize) -> RustCodecResult {
    RustCodecResult::from_bytes(a85_encode(input_from_ffi(data, len)), 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_run_length_encode(data: *const u8, len: usize) -> RustCodecResult {
    RustCodecResult::from_bytes(run_length_encode(input_from_ffi(data, len)), 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_a85_decode(data: *const u8, len: usize) -> RustCodecResult {
    match a85_decode(input_from_ffi(data, len)) {
        Ok((bytes, consumed)) => RustCodecResult::from_bytes(bytes, consumed),
        Err(()) => RustCodecResult::failure(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_hex_decode(data: *const u8, len: usize) -> RustCodecResult {
    let (bytes, consumed) = hex_decode(input_from_ffi(data, len));
    RustCodecResult::from_bytes(bytes, consumed)
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_lzw_decode(
    data: *const u8,
    len: usize,
    early_change: bool,
) -> RustCodecResult {
    match lzw_decode(input_from_ffi(data, len), early_change) {
        Ok((bytes, consumed)) => RustCodecResult::from_bytes(bytes, consumed),
        Err(()) => RustCodecResult::failure(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_run_length_decode(data: *const u8, len: usize) -> RustCodecResult {
    match run_length_decode(input_from_ffi(data, len)) {
        Ok((bytes, consumed)) => RustCodecResult::from_bytes(bytes, consumed),
        Err(()) => RustCodecResult::failure(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_codec_result_free(data: *mut u8, len: usize, capacity: usize) {
    if capacity == 0 {
        return;
    }
    // SAFETY: Only `RustCodecResult::from_bytes()` creates allocations passed
    // here, and the C++ adapter calls this function exactly once per result.
    unsafe {
        drop(Vec::from_raw_parts(data, len, capacity));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a85_encode_matches_pdfium_reference_cases() {
        assert_eq!(
            a85_encode(&[1, 2, 3, 4, 255, 255, 255, 255]),
            vec![33, 60, 78, 63, 43, 115, 56, 87, 45, 33, 126, 62]
        );
        assert_eq!(a85_encode(&[0, 0, 0, 0]), b"z~>");
    }

    #[test]
    fn run_length_encode_matches_pdfium_reference_cases() {
        assert_eq!(run_length_encode(&[]), Vec::<u8>::new());
        assert_eq!(run_length_encode(&[1]), vec![0, 1, 128]);
        assert_eq!(run_length_encode(&[2, 2, 2, 2]), vec![253, 2, 128]);
    }

    #[test]
    fn a85_decode_preserves_terminator_and_consumed_byte_behavior() {
        assert_eq!(a85_decode(b"FCfN8~>FCfN8"), Ok((b"test".to_vec(), 7)));
        assert_eq!(a85_decode(b"FCfN8FCfN8vw"), Ok((b"testtest".to_vec(), 11)));
    }

    #[test]
    fn hex_decode_preserves_odd_nibbles_and_consumed_byte_behavior() {
        assert_eq!(hex_decode(b"4869>ignored"), (b"Hi".to_vec(), 5));
        assert_eq!(hex_decode(b"4f6"), (b"O`".to_vec(), 3));
        assert_eq!(hex_decode(b"4 gF\n6@"), (b"O`".to_vec(), 7));
    }

    #[test]
    fn lzw_decode_preserves_clear_and_end_of_data_behavior() {
        let input = [0x80, 0x10, 0x48, 0x44, 0x38, 0x08];
        assert_eq!(lzw_decode(&input, true), Ok((b"ABC".to_vec(), 6)));
    }

    #[test]
    fn run_length_decode_preserves_truncated_literal_behavior() {
        assert_eq!(run_length_decode(&[2, b'a']), Ok((vec![b'a', 0, 0], 2)));
        assert_eq!(run_length_decode(&[255]), Ok((vec![0, 0], 1)));
    }
}
