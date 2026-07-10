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

// The instruction tables are a byte-for-byte port of the retained C++ Group
// 4 run decoders. Keeping their compact form avoids changing prefix matching.
const FAX_BLACK_RUN_INS: &[u8] = &[
    0, 2, 2, 3, 0, 3, 2, 0, 2, 2, 1, 0, 3, 4, 0, 2, 2, 6, 0, 3, 5, 0, 1, 3, 7, 0, 2, 4, 9, 0, 5, 8,
    0, 3, 4, 10, 0, 5, 11, 0, 7, 12, 0, 2, 4, 13, 0, 7, 14, 0, 1, 24, 15, 0, 5, 8, 18, 0, 15, 64,
    0, 23, 16, 0, 24, 17, 0, 55, 0, 0, 10, 8, 0, 7, 12, 64, 7, 13, 128, 7, 23, 24, 0, 24, 25, 0,
    40, 23, 0, 55, 22, 0, 103, 19, 0, 104, 20, 0, 108, 21, 0, 54, 18, 192, 7, 19, 0, 8, 20, 64, 8,
    21, 128, 8, 22, 192, 8, 23, 0, 9, 28, 64, 9, 29, 128, 9, 30, 192, 9, 31, 0, 10, 36, 52, 0, 39,
    55, 0, 40, 56, 0, 43, 59, 0, 44, 60, 0, 51, 64, 1, 52, 128, 1, 53, 192, 1, 55, 53, 0, 56, 54,
    0, 82, 50, 0, 83, 51, 0, 84, 44, 0, 85, 45, 0, 86, 46, 0, 87, 47, 0, 88, 57, 0, 89, 58, 0, 90,
    61, 0, 91, 0, 1, 100, 48, 0, 101, 49, 0, 102, 62, 0, 103, 63, 0, 104, 30, 0, 105, 31, 0, 106,
    32, 0, 107, 33, 0, 108, 40, 0, 109, 41, 0, 200, 128, 0, 201, 192, 0, 202, 26, 0, 203, 27, 0,
    204, 28, 0, 205, 29, 0, 210, 34, 0, 211, 35, 0, 212, 36, 0, 213, 37, 0, 214, 38, 0, 215, 39, 0,
    218, 42, 0, 219, 43, 0, 20, 74, 128, 2, 75, 192, 2, 76, 0, 3, 77, 64, 3, 82, 0, 5, 83, 64, 5,
    84, 128, 5, 85, 192, 5, 90, 0, 6, 91, 64, 6, 100, 128, 6, 101, 192, 6, 108, 0, 2, 109, 64, 2,
    114, 128, 3, 115, 192, 3, 116, 0, 4, 117, 64, 4, 118, 128, 4, 119, 192, 4, 255,
];

const FAX_WHITE_RUN_INS: &[u8] = &[
    0, 0, 0, 6, 7, 2, 0, 8, 3, 0, 11, 4, 0, 12, 5, 0, 14, 6, 0, 15, 7, 0, 6, 7, 10, 0, 8, 11, 0,
    18, 128, 0, 19, 8, 0, 20, 9, 0, 27, 64, 0, 9, 3, 13, 0, 7, 1, 0, 8, 12, 0, 23, 192, 0, 24, 128,
    6, 42, 16, 0, 43, 17, 0, 52, 14, 0, 53, 15, 0, 12, 3, 22, 0, 4, 23, 0, 8, 20, 0, 12, 19, 0, 19,
    26, 0, 23, 21, 0, 24, 28, 0, 36, 27, 0, 39, 18, 0, 40, 24, 0, 43, 25, 0, 55, 0, 1, 42, 2, 29,
    0, 3, 30, 0, 4, 45, 0, 5, 46, 0, 10, 47, 0, 11, 48, 0, 18, 33, 0, 19, 34, 0, 20, 35, 0, 21, 36,
    0, 22, 37, 0, 23, 38, 0, 26, 31, 0, 27, 32, 0, 36, 53, 0, 37, 54, 0, 40, 39, 0, 41, 40, 0, 42,
    41, 0, 43, 42, 0, 44, 43, 0, 45, 44, 0, 50, 61, 0, 51, 62, 0, 52, 63, 0, 53, 0, 0, 54, 64, 1,
    55, 128, 1, 74, 59, 0, 75, 60, 0, 82, 49, 0, 83, 50, 0, 84, 51, 0, 85, 52, 0, 88, 55, 0, 89,
    56, 0, 90, 57, 0, 91, 58, 0, 100, 192, 1, 101, 0, 2, 103, 128, 2, 104, 64, 2, 16, 152, 192, 5,
    153, 0, 6, 154, 64, 6, 155, 192, 6, 204, 192, 2, 205, 0, 3, 210, 64, 3, 211, 128, 3, 212, 192,
    3, 213, 0, 4, 214, 64, 4, 215, 128, 4, 216, 192, 4, 217, 0, 5, 218, 64, 5, 219, 128, 5, 0, 3,
    8, 0, 7, 12, 64, 7, 13, 128, 7, 10, 18, 192, 7, 19, 0, 8, 20, 64, 8, 21, 128, 8, 22, 192, 8,
    23, 0, 9, 28, 64, 9, 29, 128, 9, 30, 192, 9, 31, 0, 10, 255,
];

#[repr(C)]
pub struct RustCodecResult {
    data: *mut u8,
    len: usize,
    capacity: usize,
    bytes_consumed: u32,
}

#[repr(C)]
pub struct RustFaxScanlineResult {
    data: RustCodecResult,
    offsets: *mut u32,
    offsets_len: usize,
    offsets_capacity: usize,
}

impl RustFaxScanlineResult {
    fn from_lines(data: Vec<u8>, offsets: Vec<u32>) -> Self {
        let mut offsets = ManuallyDrop::new(offsets);
        Self {
            data: RustCodecResult::from_bytes(data, 0),
            offsets: offsets.as_mut_ptr(),
            offsets_len: offsets.len(),
            offsets_capacity: offsets.capacity(),
        }
    }

    fn failure() -> Self {
        Self::from_lines(Vec::new(), Vec::new())
    }
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

fn calculate_pitch8(bits_per_component: i32, colors: i32, columns: i32) -> Option<usize> {
    let bits = i64::from(bits_per_component)
        .checked_mul(i64::from(colors))?
        .checked_mul(i64::from(columns))?;
    if bits <= 0 {
        return None;
    }
    usize::try_from((bits + 7) / 8).ok()
}

fn paeth_predictor(left: u8, up: u8, upper_left: u8) -> u8 {
    let prediction = i32::from(left) + i32::from(up) - i32::from(upper_left);
    let left_distance = (prediction - i32::from(left)).unsigned_abs();
    let up_distance = (prediction - i32::from(up)).unsigned_abs();
    let upper_left_distance = (prediction - i32::from(upper_left)).unsigned_abs();
    if left_distance <= up_distance && left_distance <= upper_left_distance {
        left
    } else if up_distance <= upper_left_distance {
        up
    } else {
        upper_left
    }
}

fn png_predictor(
    colors: i32,
    bits_per_component: i32,
    columns: i32,
    input: &[u8],
) -> Option<Vec<u8>> {
    let row_size = calculate_pitch8(bits_per_component, colors, columns)?;
    let source_row_size = row_size.checked_add(1)?;
    let row_count = input.len().checked_add(row_size)? / source_row_size;
    if row_count == 0 {
        return None;
    }
    let last_row_size = input.len() % source_row_size;
    let mut destination_size = row_size.checked_mul(row_count)?;
    if last_row_size != 0 {
        destination_size = destination_size.checked_sub(source_row_size - last_row_size)?;
    }
    let mut output = vec![0; destination_size];
    let bytes_per_pixel =
        usize::try_from((i64::from(colors).checked_mul(i64::from(bits_per_component))? + 7) / 8)
            .ok()?;
    let mut source_pos: usize = 0;
    let mut destination_pos: usize = 0;
    let mut previous_row_pos: usize = 0;
    for _ in 0..row_count {
        let row_data_pos = source_pos.checked_add(1)?;
        let remaining_row_size = row_size.min(input.len().checked_sub(row_data_pos)?);
        let tag = input[source_pos];
        for index in 0..remaining_row_size {
            let source = input[row_data_pos + index];
            let left: u8 = if index >= bytes_per_pixel {
                output[destination_pos + index - bytes_per_pixel]
            } else {
                0
            };
            let up: u8 = if destination_pos == 0 { 0 } else { output[previous_row_pos + index] };
            let upper_left: u8 = if destination_pos != 0 && index >= bytes_per_pixel {
                output[previous_row_pos + index - bytes_per_pixel]
            } else {
                0
            };
            output[destination_pos + index] = match tag {
                1 => source.wrapping_add(left),
                2 => source.wrapping_add(up),
                3 => source.wrapping_add((up.wrapping_add(left)) / 2),
                4 => source.wrapping_add(paeth_predictor(left, up, upper_left)),
                _ => source,
            };
        }
        source_pos = source_pos.checked_add(remaining_row_size + 1)?;
        previous_row_pos = destination_pos;
        destination_pos += remaining_row_size;
    }
    Some(output)
}

fn tiff_predictor(
    colors: i32,
    bits_per_component: i32,
    columns: i32,
    mut output: Vec<u8>,
) -> Option<Vec<u8>> {
    let row_size = calculate_pitch8(bits_per_component, colors, columns)?;
    let mut row_start = 0;
    while row_start < output.len() {
        let row_end = (row_start + row_size).min(output.len());
        let row = &mut output[row_start..row_end];
        if bits_per_component == 1 {
            let row_bits = i64::from(bits_per_component)
                .checked_mul(i64::from(colors))?
                .checked_mul(i64::from(columns))?
                .min(i64::try_from(row.len().checked_mul(8)?).ok()?);
            let mut previous_index = 0;
            let mut previous_column = 0;
            for bit in 1..row_bits {
                let column = bit % 8;
                let index = usize::try_from(bit / 8).ok()?;
                let current = (row[index] >> (7 - column)) & 1;
                let previous = (row[previous_index] >> (7 - previous_column)) & 1;
                if current ^ previous != 0 {
                    row[index] |= 1 << (7 - column);
                } else {
                    row[index] &= !(1 << (7 - column));
                }
                previous_index = index;
                previous_column = column;
            }
        } else {
            let bytes_per_pixel =
                usize::try_from(i64::from(bits_per_component).checked_mul(i64::from(colors))? / 8)
                    .ok()?;
            if bits_per_component == 16 {
                let mut index = bytes_per_pixel;
                while index + 1 < row.len() {
                    let pixel = u16::from_be_bytes([
                        row[index - bytes_per_pixel],
                        row[index - bytes_per_pixel + 1],
                    ])
                    .wrapping_add(u16::from_be_bytes([row[index], row[index + 1]]));
                    let [high, low] = pixel.to_be_bytes();
                    row[index] = high;
                    row[index + 1] = low;
                    index += 2;
                }
            } else {
                for index in bytes_per_pixel..row.len() {
                    row[index] = row[index].wrapping_add(row[index - bytes_per_pixel]);
                }
            }
        }
        row_start = row_end;
    }
    Some(output)
}

fn fax_next_bit(input: &[u8], bit_pos: &mut u32) -> bool {
    let pos = *bit_pos;
    *bit_pos = bit_pos.wrapping_add(1);
    input[(pos / 8) as usize] & (1 << (7 - pos % 8)) != 0
}

fn fax_find_bit(data: &[u8], max_pos: i32, mut start_pos: i32, bit: bool) -> i32 {
    while start_pos < max_pos {
        let current = data[(start_pos / 8) as usize] & (1 << (7 - start_pos % 8)) != 0;
        if current == bit {
            return start_pos;
        }
        start_pos += 1;
    }
    max_pos
}

fn fax_g4_find_b1_b2(reference: &[u8], columns: i32, a0: i32, a0_color: bool) -> (i32, i32) {
    let mut first_bit = a0 < 0 || reference[(a0 / 8) as usize] & (1 << (7 - a0 % 8)) != 0;
    let mut b1 = fax_find_bit(reference, columns, a0 + 1, !first_bit);
    if b1 >= columns {
        return (columns, columns);
    }
    if first_bit == !a0_color {
        b1 = fax_find_bit(reference, columns, b1 + 1, first_bit);
        first_bit = !first_bit;
    }
    if b1 >= columns {
        return (columns, columns);
    }
    (b1, fax_find_bit(reference, columns, b1 + 1, first_bit))
}

fn fax_fill_bits(destination: &mut [u8], columns: i32, start: i32, end: i32) {
    let start = start.max(0);
    let end = end.clamp(0, columns);
    for position in start..end {
        let byte = &mut destination[(position / 8) as usize];
        *byte = byte.wrapping_sub(1 << (7 - position % 8));
    }
}

fn fax_get_run(table: &[u8], input: &[u8], bit_pos: &mut u32) -> i32 {
    let bit_size = input.len().saturating_mul(8) as u32;
    let mut code = 0_u8;
    let mut instruction_offset = 0_usize;
    loop {
        let instruction = table[instruction_offset];
        instruction_offset += 1;
        if instruction == 0xff || *bit_pos >= bit_size {
            return -1;
        }
        code <<= 1;
        if fax_next_bit(input, bit_pos) {
            code += 1;
        }
        let next_offset = instruction_offset + usize::from(instruction) * 3;
        while instruction_offset < next_offset {
            if table[instruction_offset] == code {
                return i32::from(table[instruction_offset + 1])
                    + i32::from(table[instruction_offset + 2]) * 256;
            }
            instruction_offset += 3;
        }
    }
}

fn fax_g4_get_row(
    input: &[u8],
    bit_pos: &mut u32,
    destination: &mut [u8],
    reference: &[u8],
    columns: i32,
) {
    let bit_size = input.len().saturating_mul(8) as u32;
    let mut a0 = -1;
    let mut a0_color = true;
    loop {
        if *bit_pos >= bit_size {
            return;
        }
        let (b1, b2) = fax_g4_find_b1_b2(reference, columns, a0, a0_color);
        let mut vertical_delta = 0;
        if !fax_next_bit(input, bit_pos) {
            if *bit_pos >= bit_size {
                return;
            }
            let bit1 = fax_next_bit(input, bit_pos);
            if *bit_pos >= bit_size {
                return;
            }
            let bit2 = fax_next_bit(input, bit_pos);
            if bit1 {
                vertical_delta = if bit2 { 1 } else { -1 };
            } else if bit2 {
                let mut run1 = 0;
                loop {
                    let run = fax_get_run(
                        if a0_color { FAX_WHITE_RUN_INS } else { FAX_BLACK_RUN_INS },
                        input,
                        bit_pos,
                    );
                    run1 += run;
                    if run < 64 {
                        break;
                    }
                }
                if a0 < 0 {
                    run1 += 1;
                }
                if run1 < 0 {
                    return;
                }
                let a1 = a0 + run1;
                if !a0_color {
                    fax_fill_bits(destination, columns, a0, a1);
                }
                let mut run2 = 0;
                loop {
                    let run = fax_get_run(
                        if a0_color { FAX_BLACK_RUN_INS } else { FAX_WHITE_RUN_INS },
                        input,
                        bit_pos,
                    );
                    run2 += run;
                    if run < 64 {
                        break;
                    }
                }
                if run2 < 0 {
                    return;
                }
                let a2 = a1 + run2;
                if a0_color {
                    fax_fill_bits(destination, columns, a1, a2);
                }
                a0 = a2;
                if a0 < columns {
                    continue;
                }
                return;
            } else {
                if *bit_pos >= bit_size {
                    return;
                }
                if fax_next_bit(input, bit_pos) {
                    if !a0_color {
                        fax_fill_bits(destination, columns, a0, b2);
                    }
                    if b2 >= columns {
                        return;
                    }
                    a0 = b2;
                    continue;
                }
                if *bit_pos >= bit_size {
                    return;
                }
                let next_bit1 = fax_next_bit(input, bit_pos);
                if *bit_pos >= bit_size {
                    return;
                }
                let next_bit2 = fax_next_bit(input, bit_pos);
                if next_bit1 {
                    vertical_delta = if next_bit2 { 2 } else { -2 };
                } else if next_bit2 {
                    if *bit_pos >= bit_size {
                        return;
                    }
                    vertical_delta = if fax_next_bit(input, bit_pos) { 3 } else { -3 };
                } else {
                    if *bit_pos >= bit_size {
                        return;
                    }
                    if fax_next_bit(input, bit_pos) {
                        *bit_pos = bit_pos.wrapping_add(3);
                        continue;
                    }
                    *bit_pos = bit_pos.wrapping_add(5);
                    return;
                }
            }
        }
        let a1 = b1 + vertical_delta;
        if !a0_color {
            fax_fill_bits(destination, columns, a0, a1);
        }
        if a1 >= columns || a0 >= a1 {
            return;
        }
        a0 = a1;
        a0_color = !a0_color;
    }
}

fn fax_g4_decode(
    input: &[u8],
    starting_bit_pos: u32,
    width: i32,
    height: i32,
    pitch: i32,
) -> Option<(Vec<u8>, u32)> {
    let pitch = usize::try_from(pitch).ok()?;
    let height = usize::try_from(height).ok()?;
    if pitch == 0 || width <= 0 {
        return None;
    }
    let mut output = vec![0; pitch.checked_mul(height)?];
    let mut reference = vec![0xff; pitch];
    let mut bit_pos = starting_bit_pos;
    for line in output.chunks_exact_mut(pitch) {
        line.fill(0xff);
        fax_g4_get_row(input, &mut bit_pos, line, &reference, width);
        reference.copy_from_slice(line);
    }
    Some((output, bit_pos))
}

fn fax_skip_eol(input: &[u8], bit_pos: &mut u32) {
    let bit_size = input.len().saturating_mul(8) as u32;
    let start_bit = *bit_pos;
    while *bit_pos < bit_size {
        if !fax_next_bit(input, bit_pos) {
            continue;
        }
        if bit_pos.wrapping_sub(start_bit) <= 11 {
            *bit_pos = start_bit;
        }
        return;
    }
}

fn fax_get_1d_line(input: &[u8], bit_pos: &mut u32, destination: &mut [u8], columns: i32) {
    let bit_size = input.len().saturating_mul(8) as u32;
    let mut color = true;
    let mut start_pos = 0;
    loop {
        if *bit_pos >= bit_size {
            return;
        }
        let mut run_length = 0;
        loop {
            let run = fax_get_run(
                if color { FAX_WHITE_RUN_INS } else { FAX_BLACK_RUN_INS },
                input,
                bit_pos,
            );
            if run < 0 {
                while *bit_pos < bit_size {
                    if fax_next_bit(input, bit_pos) {
                        return;
                    }
                }
                return;
            }
            run_length += run;
            if run < 64 {
                break;
            }
        }
        if !color {
            fax_fill_bits(destination, columns, start_pos, start_pos + run_length);
        }
        start_pos += run_length;
        if start_pos >= columns {
            return;
        }
        color = !color;
    }
}

fn fax_scanline_decode(
    input: &[u8],
    width: i32,
    height: i32,
    encoding: i32,
    end_of_line: bool,
    byte_align: bool,
    black_is_1: bool,
    pitch: i32,
) -> Option<(Vec<u8>, Vec<u32>)> {
    let pitch = usize::try_from(pitch).ok()?;
    let height = usize::try_from(height).ok()?;
    if width <= 0 || pitch == 0 {
        return None;
    }
    let bit_size = input.len().saturating_mul(8) as u32;
    let mut data = Vec::with_capacity(pitch.checked_mul(height)?);
    let mut offsets = Vec::with_capacity(height);
    let mut reference = vec![0xff; pitch];
    let mut bit_pos = 0_u32;
    let mut byte_align = byte_align;
    for _ in 0..height {
        fax_skip_eol(input, &mut bit_pos);
        if bit_pos >= bit_size {
            break;
        }
        let mut line = vec![0xff; pitch];
        if encoding < 0 {
            fax_g4_get_row(input, &mut bit_pos, &mut line, &reference, width);
            reference.copy_from_slice(&line);
        } else if encoding == 0 {
            fax_get_1d_line(input, &mut bit_pos, &mut line, width);
        } else {
            if fax_next_bit(input, &mut bit_pos) {
                fax_get_1d_line(input, &mut bit_pos, &mut line, width);
            } else {
                fax_g4_get_row(input, &mut bit_pos, &mut line, &reference, width);
            }
            reference.copy_from_slice(&line);
        }
        if end_of_line {
            fax_skip_eol(input, &mut bit_pos);
        }
        if byte_align && bit_pos < bit_size {
            let mut bit_pos0 = bit_pos;
            let bit_pos1 = (bit_pos + 7) & !7;
            while byte_align && bit_pos0 < bit_pos1 {
                if input[(bit_pos0 / 8) as usize] & (1 << (7 - bit_pos0 % 8)) != 0 {
                    byte_align = false;
                } else {
                    bit_pos0 += 1;
                }
            }
            if byte_align {
                bit_pos = bit_pos1;
            }
        }
        if black_is_1 {
            for value in &mut line {
                *value = !*value;
            }
        }
        data.extend_from_slice(&line);
        offsets.push(((bit_pos + 7) / 8).min(input.len() as u32));
    }
    Some((data, offsets))
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
pub extern "C" fn pdfium_rust_png_predictor(
    data: *const u8,
    len: usize,
    colors: i32,
    bits_per_component: i32,
    columns: i32,
) -> RustCodecResult {
    match png_predictor(colors, bits_per_component, columns, input_from_ffi(data, len)) {
        Some(bytes) => RustCodecResult::from_bytes(bytes, 0),
        None => RustCodecResult::failure(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_tiff_predictor(
    data: *const u8,
    len: usize,
    colors: i32,
    bits_per_component: i32,
    columns: i32,
) -> RustCodecResult {
    let input = input_from_ffi(data, len).to_vec();
    match tiff_predictor(colors, bits_per_component, columns, input) {
        Some(bytes) => RustCodecResult::from_bytes(bytes, 0),
        None => RustCodecResult::failure(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_fax_g4_decode(
    data: *const u8,
    len: usize,
    starting_bit_pos: u32,
    width: i32,
    height: i32,
    pitch: i32,
) -> RustCodecResult {
    match fax_g4_decode(input_from_ffi(data, len), starting_bit_pos, width, height, pitch) {
        Some((bytes, bit_pos)) => RustCodecResult::from_bytes(bytes, bit_pos),
        None => RustCodecResult::failure(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_fax_scanline_decode(
    data: *const u8,
    len: usize,
    width: i32,
    height: i32,
    encoding: i32,
    end_of_line: bool,
    byte_align: bool,
    black_is_1: bool,
    pitch: i32,
) -> RustFaxScanlineResult {
    match fax_scanline_decode(
        input_from_ffi(data, len),
        width,
        height,
        encoding,
        end_of_line,
        byte_align,
        black_is_1,
        pitch,
    ) {
        Some((bytes, offsets)) => RustFaxScanlineResult::from_lines(bytes, offsets),
        None => RustFaxScanlineResult::failure(),
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_fax_scanline_result_free(result: RustFaxScanlineResult) {
    // SAFETY: `result` was created by `pdfium_rust_fax_scanline_decode()` and
    // the C++ adapter invokes this exactly once after copying both vectors.
    unsafe {
        pdfium_rust_codec_result_free(result.data.data, result.data.len, result.data.capacity);
        if result.offsets_capacity != 0 {
            drop(Vec::from_raw_parts(result.offsets, result.offsets_len, result.offsets_capacity));
        }
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
    fn predictors_preserve_png_and_tiff_reference_cases() {
        assert_eq!(png_predictor(1, 8, 3, &[1, 1, 1, 1, 2, 4, 4, 4]), Some(vec![1, 2, 3, 5, 6, 7]));
        assert_eq!(tiff_predictor(1, 8, 4, vec![1, 1, 1, 1]), Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn fax_g4_decode_preserves_horizontal_and_truncated_rows() {
        assert_eq!(
            fax_g4_decode(&[0x26, 0xa2, 0x80], 0, 8, 1, 4),
            Some((vec![0, 0xff, 0xff, 0xff], 17))
        );
        assert_eq!(fax_g4_decode(&[0], 0, 8, 1, 4), Some((vec![0xff, 0xff, 0xff, 0xff], 12)));
    }

    #[test]
    fn fax_scanline_decode_preserves_group3_rows_and_offsets() {
        assert_eq!(
            fax_scanline_decode(&[0x35, 0x14], 8, 1, 0, false, false, false, 4),
            Some((vec![0, 0xff, 0xff, 0xff], vec![2]))
        );
    }

    #[test]
    fn run_length_decode_preserves_truncated_literal_behavior() {
        assert_eq!(run_length_decode(&[2, b'a']), Ok((vec![b'a', 0, 0], 2)));
        assert_eq!(run_length_decode(&[255]), Ok((vec![0, 0], 1)));
    }
}
