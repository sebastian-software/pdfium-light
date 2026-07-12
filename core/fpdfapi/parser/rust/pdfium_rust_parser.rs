//! Narrow parser primitives that preserve the retained C++ parser's behavior.

use std::collections::BTreeMap;

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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CrossRefObjectInfo {
    object_type: u8,
    is_object_stream: bool,
    generation: u16,
    position: i64,
    archive_object_number: u32,
    archive_object_index: u32,
}

#[derive(Default)]
pub struct CrossRefTableState {
    objects: BTreeMap<u32, CrossRefObjectInfo>,
}

impl CrossRefTableState {
    fn add_compressed(
        &mut self,
        object_number: u32,
        archive_object_number: u32,
        archive_object_index: u32,
    ) {
        let info = self.objects.entry(object_number).or_default();
        if info.generation > 0 || info.is_object_stream {
            return;
        }
        info.object_type = 2;
        info.archive_object_number = archive_object_number;
        info.archive_object_index = archive_object_index;
        info.generation = 0;
        self.objects.entry(archive_object_number).or_default().is_object_stream = true;
    }

    fn add_normal(
        &mut self,
        object_number: u32,
        generation: u16,
        is_object_stream: bool,
        position: i64,
    ) {
        let info = self.objects.entry(object_number).or_default();
        if info.generation > generation {
            return;
        }
        info.object_type = 1;
        info.is_object_stream |= is_object_stream;
        info.generation = generation;
        info.position = position;
    }

    fn set_free(&mut self, object_number: u32, generation: u16) {
        let info = self.objects.entry(object_number).or_default();
        info.object_type = 0;
        info.generation = generation;
        info.position = 0;
    }

    fn set_size(&mut self, size: u32) {
        if size == 0 {
            self.objects.clear();
            return;
        }
        drop(self.objects.split_off(&size));
        self.objects.entry(size - 1).or_default().position = 0;
    }

    fn overlay(&mut self, top: &mut Self) {
        if top.objects.is_empty() {
            return;
        }
        if self.objects.is_empty() {
            self.objects = std::mem::take(&mut top.objects);
            return;
        }
        for (&object_number, &current) in &self.objects {
            match top.objects.entry(object_number) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(current);
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    let new = entry.get_mut();
                    if new.object_type == 1 && current.object_type == 1 && current.is_object_stream
                    {
                        new.is_object_stream = true;
                    }
                }
            }
        }
        self.objects = std::mem::take(&mut top.objects);
    }
}

#[derive(Default)]
pub struct IndirectObjectIndexState {
    last_object_number: u32,
    objects: BTreeMap<u32, Option<usize>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PdfNumberValue {
    Unsigned(u32),
    Signed(i32),
    Float(f32),
}

pub struct PdfNumberState {
    value: PdfNumberValue,
}

pub struct PdfBooleanState {
    value: bool,
}

impl Default for PdfNumberState {
    fn default() -> Self {
        Self { value: PdfNumberValue::Unsigned(0) }
    }
}

impl PdfNumberState {
    fn from_bytes(input: &[u8]) -> Self {
        if input.is_empty() {
            return Self::default();
        }
        if input.contains(&b'.') {
            return Self { value: PdfNumberValue::Float(parse_pdf_float(input)) };
        }

        let (signed, negative, digits) = match input[0] {
            b'+' => (true, false, &input[1..]),
            b'-' => (true, true, &input[1..]),
            _ => (false, false, input),
        };
        let mut value = Some(0_u32);
        for &byte in digits {
            if !byte.is_ascii_digit() {
                break;
            }
            value = value
                .and_then(|value| value.checked_mul(10))
                .and_then(|value| value.checked_add(u32::from(byte - b'0')));
        }
        let mut value = value.unwrap_or(0);
        if !signed {
            return Self { value: PdfNumberValue::Unsigned(value) };
        }
        let limit = i32::MAX as u32 + u32::from(negative);
        if value > limit {
            value = 0;
        }
        let signed_value = if negative {
            if value == i32::MIN as u32 {
                i32::MIN
            } else {
                -(value as i32)
            }
        } else {
            value as i32
        };
        Self { value: PdfNumberValue::Signed(signed_value) }
    }

    fn is_integer(&self) -> bool {
        !matches!(self.value, PdfNumberValue::Float(_))
    }

    fn get_signed(&self) -> i32 {
        match self.value {
            PdfNumberValue::Unsigned(value) => value as i32,
            PdfNumberValue::Signed(value) => value,
            PdfNumberValue::Float(value) => value as i32,
        }
    }

    fn get_float(&self) -> f32 {
        match self.value {
            PdfNumberValue::Unsigned(value) => value as f32,
            PdfNumberValue::Signed(value) => value as f32,
            PdfNumberValue::Float(value) => value,
        }
    }
}

fn parse_pdf_float(input: &[u8]) -> f32 {
    let mut start = 0;
    while input.get(start).is_some_and(|byte| matches!(byte, b' ' | b'+' | b'-')) {
        start += 1;
    }
    if start > 0 && input[start - 1] == b'-' {
        start -= 1;
    }
    let input = &input[start..];
    let mut end = usize::from(input.first().is_some_and(|byte| matches!(byte, b'+' | b'-')));
    let mut digits = 0;
    while input.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
        digits += 1;
    }
    if input.get(end) == Some(&b'.') {
        end += 1;
        while input.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    if matches!(input.get(end), Some(b'e' | b'E')) {
        let exponent_start = end;
        end += 1;
        if matches!(input.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let exponent_digits = end;
        while input.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == exponent_digits {
            end = exponent_start;
        }
    }
    std::str::from_utf8(&input[..end])
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or_else(|| {
            if input.first() == Some(&b'-') {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            }
        })
}

impl IndirectObjectIndexState {
    fn lookup(&self, object_number: u32) -> (u8, usize) {
        match self.objects.get(&object_number) {
            None => (0, 0),
            Some(None) => (1, 0),
            Some(Some(handle)) => (2, *handle),
        }
    }

    fn reserve_parse(&mut self, object_number: u32) -> (u8, usize) {
        match self.objects.entry(object_number) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(None);
                (0, 0)
            }
            std::collections::btree_map::Entry::Occupied(entry) => match entry.get() {
                None => (1, 0),
                Some(handle) => (2, *handle),
            },
        }
    }

    fn finish_parse(&mut self, object_number: u32, handle: usize) -> bool {
        let Some(slot) = self.objects.get_mut(&object_number) else {
            return false;
        };
        if slot.is_some() || handle == 0 {
            return false;
        }
        *slot = Some(handle);
        self.last_object_number = self.last_object_number.max(object_number);
        true
    }

    fn cancel_parse(&mut self, object_number: u32) -> bool {
        if self.objects.get(&object_number) != Some(&None) {
            return false;
        }
        self.objects.remove(&object_number);
        true
    }

    fn add(&mut self, handle: usize) -> Option<(u32, Option<usize>)> {
        if handle == 0 {
            return None;
        }
        self.last_object_number = self.last_object_number.wrapping_add(1);
        let old_handle = self.objects.insert(self.last_object_number, Some(handle)).flatten();
        Some((self.last_object_number, old_handle))
    }

    fn replace(
        &mut self,
        object_number: u32,
        handle: usize,
        new_generation: u32,
        old_generation: Option<u32>,
    ) -> Option<Option<usize>> {
        if handle == 0 {
            return None;
        }
        if old_generation.is_some_and(|old| new_generation <= old) {
            return None;
        }
        let old_handle = self.objects.insert(object_number, Some(handle)).flatten();
        self.last_object_number = self.last_object_number.max(object_number);
        Some(old_handle)
    }

    fn delete(&mut self, object_number: u32) -> Option<usize> {
        self.objects.get(&object_number).copied().flatten()?;
        self.objects.remove(&object_number).flatten()
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

type CrossRefSnapshotCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, u32, u8, bool, u16, i64, u32, u32) -> bool;

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_cross_ref_table_new() -> *mut CrossRefTableState {
    Box::into_raw(Box::new(CrossRefTableState::default()))
}

/// # Safety
///
/// `state` must be null or a pointer returned by
/// `pdfium_rust_cross_ref_table_new` that has not previously been destroyed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_table_destroy(state: *mut CrossRefTableState) {
    if !state.is_null() {
        // SAFETY: The caller transfers the unique allocation back to Rust.
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
///
/// `state` must point to a live Rust cross-reference table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_table_add_compressed(
    state: *mut CrossRefTableState,
    object_number: u32,
    archive_object_number: u32,
    archive_object_index: u32,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    state.add_compressed(object_number, archive_object_number, archive_object_index);
    true
}

/// # Safety
///
/// `state` must point to a live Rust cross-reference table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_table_add_normal(
    state: *mut CrossRefTableState,
    object_number: u32,
    generation: u16,
    is_object_stream: bool,
    position: i64,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    state.add_normal(object_number, generation, is_object_stream, position);
    true
}

/// # Safety
///
/// `state` must point to a live Rust cross-reference table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_table_set_free(
    state: *mut CrossRefTableState,
    object_number: u32,
    generation: u16,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    state.set_free(object_number, generation);
    true
}

/// # Safety
///
/// `state` must point to a live Rust cross-reference table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_table_set_size(
    state: *mut CrossRefTableState,
    size: u32,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    state.set_size(size);
    true
}

/// Moves the top table's object entries over the current table.
///
/// # Safety
///
/// Both pointers must identify distinct live Rust cross-reference tables.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_table_overlay(
    current: *mut CrossRefTableState,
    top: *mut CrossRefTableState,
) -> bool {
    if current.is_null() || top.is_null() || current == top {
        return false;
    }
    // SAFETY: The caller guarantees two distinct live allocations.
    let (current, top) = unsafe { (&mut *current, &mut *top) };
    current.overlay(top);
    true
}

/// # Safety
///
/// `state` must point to a live table. A present entry requires every output
/// pointer to refer to one writable scalar.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_table_get(
    state: *const CrossRefTableState,
    object_number: u32,
    output_type: *mut u8,
    output_is_object_stream: *mut bool,
    output_generation: *mut u16,
    output_position: *mut i64,
    output_archive_object_number: *mut u32,
    output_archive_object_index: *mut u32,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    let Some(info) = state.objects.get(&object_number) else {
        return false;
    };
    if output_type.is_null()
        || output_is_object_stream.is_null()
        || output_generation.is_null()
        || output_position.is_null()
        || output_archive_object_number.is_null()
        || output_archive_object_index.is_null()
    {
        return false;
    }
    // SAFETY: Every output pointer was checked and the caller guarantees it is
    // writable.
    unsafe {
        *output_type = info.object_type;
        *output_is_object_stream = info.is_object_stream;
        *output_generation = info.generation;
        *output_position = info.position;
        *output_archive_object_number = info.archive_object_number;
        *output_archive_object_index = info.archive_object_index;
    }
    true
}

/// Visits the complete table in ascending object-number order.
///
/// # Safety
///
/// `state`, `context`, and `callback` must remain valid for the synchronous
/// walk.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_cross_ref_table_snapshot(
    state: *const CrossRefTableState,
    context: *mut core::ffi::c_void,
    callback: Option<CrossRefSnapshotCallback>,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if context.is_null() {
        return false;
    }
    let Some(callback) = callback else {
        return false;
    };
    for (&object_number, info) in &state.objects {
        // SAFETY: The caller guarantees a synchronous live callback boundary.
        if !unsafe {
            callback(
                context,
                object_number,
                info.object_type,
                info.is_object_stream,
                info.generation,
                info.position,
                info.archive_object_number,
                info.archive_object_index,
            )
        } {
            return false;
        }
    }
    true
}

type IndirectObjectSnapshotCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, u32, usize) -> bool;

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_indirect_object_index_new() -> *mut IndirectObjectIndexState {
    Box::into_raw(Box::new(IndirectObjectIndexState::default()))
}

/// # Safety
///
/// `state` must be null or a uniquely owned pointer returned by
/// `pdfium_rust_indirect_object_index_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_destroy(
    state: *mut IndirectObjectIndexState,
) {
    if !state.is_null() {
        // SAFETY: The caller transfers the unique allocation back to Rust.
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
///
/// `state` and both output pointers must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_lookup(
    state: *const IndirectObjectIndexState,
    object_number: u32,
    output_status: *mut u8,
    output_handle: *mut usize,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if output_status.is_null() || output_handle.is_null() {
        return false;
    }
    let (status, handle) = state.lookup(object_number);
    // SAFETY: Both checked outputs point to writable scalars.
    unsafe {
        *output_status = status;
        *output_handle = handle;
    }
    true
}

/// # Safety
///
/// `state` and both output pointers must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_reserve_parse(
    state: *mut IndirectObjectIndexState,
    object_number: u32,
    output_status: *mut u8,
    output_handle: *mut usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if output_status.is_null() || output_handle.is_null() {
        return false;
    }
    let (status, handle) = state.reserve_parse(object_number);
    // SAFETY: Both checked outputs point to writable scalars.
    unsafe {
        *output_status = status;
        *output_handle = handle;
    }
    true
}

/// # Safety
///
/// `state` must point to a live index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_finish_parse(
    state: *mut IndirectObjectIndexState,
    object_number: u32,
    handle: usize,
) -> bool {
    unsafe { state.as_mut() }.is_some_and(|state| state.finish_parse(object_number, handle))
}

/// # Safety
///
/// `state` must point to a live index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_cancel_parse(
    state: *mut IndirectObjectIndexState,
    object_number: u32,
) -> bool {
    unsafe { state.as_mut() }.is_some_and(|state| state.cancel_parse(object_number))
}

/// # Safety
///
/// `state` and `output_object_number` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_add(
    state: *mut IndirectObjectIndexState,
    handle: usize,
    output_object_number: *mut u32,
    output_had_old_handle: *mut bool,
    output_old_handle: *mut usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if output_object_number.is_null()
        || output_had_old_handle.is_null()
        || output_old_handle.is_null()
    {
        return false;
    }
    let Some((object_number, old_handle)) = state.add(handle) else {
        return false;
    };
    // SAFETY: Every checked output points to one writable scalar.
    unsafe {
        *output_object_number = object_number;
        *output_had_old_handle = old_handle.is_some();
        *output_old_handle = old_handle.unwrap_or(0);
    }
    true
}

/// # Safety
///
/// `state` and every output pointer must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_replace(
    state: *mut IndirectObjectIndexState,
    object_number: u32,
    handle: usize,
    new_generation: u32,
    has_old_generation: bool,
    old_generation: u32,
    output_applied: *mut bool,
    output_had_old_handle: *mut bool,
    output_old_handle: *mut usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if output_applied.is_null() || output_had_old_handle.is_null() || output_old_handle.is_null() {
        return false;
    }
    let result = state.replace(
        object_number,
        handle,
        new_generation,
        has_old_generation.then_some(old_generation),
    );
    let (applied, old_handle) = match result {
        None => (false, None),
        Some(old_handle) => (true, old_handle),
    };
    // SAFETY: Every checked output points to one writable scalar.
    unsafe {
        *output_applied = applied;
        *output_had_old_handle = old_handle.is_some();
        *output_old_handle = old_handle.unwrap_or(0);
    }
    true
}

/// # Safety
///
/// `state` and both output pointers must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_delete(
    state: *mut IndirectObjectIndexState,
    object_number: u32,
    output_deleted: *mut bool,
    output_handle: *mut usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if output_deleted.is_null() || output_handle.is_null() {
        return false;
    }
    let handle = state.delete(object_number);
    // SAFETY: Both checked outputs point to writable scalars.
    unsafe {
        *output_deleted = handle.is_some();
        *output_handle = handle.unwrap_or(0);
    }
    true
}

/// # Safety
///
/// `state` and `output` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_get_last(
    state: *const IndirectObjectIndexState,
    output: *mut u32,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if output.is_null() {
        return false;
    }
    // SAFETY: The checked output points to one writable scalar.
    unsafe { *output = state.last_object_number };
    true
}

/// # Safety
///
/// `state` must point to a live index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_set_last(
    state: *mut IndirectObjectIndexState,
    object_number: u32,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    state.last_object_number = object_number;
    true
}

/// Visits every slot in object-number order. A zero handle denotes the
/// recursive-parse placeholder.
///
/// # Safety
///
/// `state`, `context`, and `callback` must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_snapshot(
    state: *const IndirectObjectIndexState,
    context: *mut core::ffi::c_void,
    callback: Option<IndirectObjectSnapshotCallback>,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if context.is_null() {
        return false;
    }
    let Some(callback) = callback else {
        return false;
    };
    for (&object_number, handle) in &state.objects {
        // SAFETY: The caller guarantees a live synchronous callback boundary.
        if !unsafe { callback(context, object_number, handle.unwrap_or(0)) } {
            return false;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_pdf_number_new_default() -> *mut PdfNumberState {
    Box::into_raw(Box::new(PdfNumberState::default()))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_pdf_number_new_signed(value: i32) -> *mut PdfNumberState {
    Box::into_raw(Box::new(PdfNumberState { value: PdfNumberValue::Signed(value) }))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_pdf_number_new_float(value: f32) -> *mut PdfNumberState {
    Box::into_raw(Box::new(PdfNumberState { value: PdfNumberValue::Float(value) }))
}

/// # Safety
///
/// A nonempty input must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_number_new_string(
    data: *const u8,
    len: usize,
) -> *mut PdfNumberState {
    if len != 0 && data.is_null() {
        return core::ptr::null_mut();
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    // SAFETY: The caller guarantees a readable span when nonempty.
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    Box::into_raw(Box::new(PdfNumberState::from_bytes(input)))
}

/// # Safety
///
/// `state` must be null or a uniquely owned pointer returned by a number
/// constructor in this module.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_number_destroy(state: *mut PdfNumberState) {
    if !state.is_null() {
        // SAFETY: The caller transfers the unique allocation back to Rust.
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
///
/// `state` and `output` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_number_is_integer(
    state: *const PdfNumberState,
    output: *mut bool,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if output.is_null() {
        return false;
    }
    // SAFETY: The checked output points to one writable scalar.
    unsafe { *output = state.is_integer() };
    true
}

/// # Safety
///
/// `state` and `output` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_number_get_signed(
    state: *const PdfNumberState,
    output: *mut i32,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if output.is_null() {
        return false;
    }
    // SAFETY: The checked output points to one writable scalar.
    unsafe { *output = state.get_signed() };
    true
}

/// # Safety
///
/// `state` and `output` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_number_get_float(
    state: *const PdfNumberState,
    output: *mut f32,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if output.is_null() {
        return false;
    }
    // SAFETY: The checked output points to one writable scalar.
    unsafe { *output = state.get_float() };
    true
}

/// # Safety
///
/// `state` must point to a live number and a nonempty input must point to
/// `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_number_set_string(
    state: *mut PdfNumberState,
    data: *const u8,
    len: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if len != 0 && data.is_null() {
        return false;
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    // SAFETY: The caller guarantees a readable span when nonempty.
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    *state = PdfNumberState::from_bytes(input);
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_pdf_boolean_new(value: bool) -> *mut PdfBooleanState {
    Box::into_raw(Box::new(PdfBooleanState { value }))
}

/// # Safety
///
/// `state` must be null or a uniquely owned pointer returned by
/// `pdfium_rust_pdf_boolean_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_boolean_destroy(state: *mut PdfBooleanState) {
    if !state.is_null() {
        // SAFETY: The caller transfers the unique allocation back to Rust.
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
///
/// `state` and `output` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_boolean_get(
    state: *const PdfBooleanState,
    output: *mut bool,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if output.is_null() {
        return false;
    }
    // SAFETY: The checked output points to one writable bool.
    unsafe { *output = state.value };
    true
}

/// # Safety
///
/// `state` must point to a live value and nonempty input must identify `len`
/// readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_boolean_set_string(
    state: *mut PdfBooleanState,
    data: *const u8,
    len: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if len != 0 && data.is_null() {
        return false;
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    // SAFETY: The caller guarantees a readable span when nonempty.
    let input = unsafe { core::slice::from_raw_parts(data, len) };
    state.value = input == b"true";
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

    struct SnapshotState {
        object_numbers: [u32; 4],
        count: usize,
    }

    unsafe extern "C" fn record_mutation(context: *mut core::ffi::c_void, action: u8) -> bool {
        // SAFETY: The test passes one live `MutationState` for the call.
        let state = unsafe { &mut *context.cast::<MutationState>() };
        state.action = action;
        state.calls += 1;
        true
    }

    unsafe extern "C" fn record_snapshot(
        context: *mut core::ffi::c_void,
        object_number: u32,
        _object_type: u8,
        _is_object_stream: bool,
        _generation: u16,
        _position: i64,
        _archive_object_number: u32,
        _archive_object_index: u32,
    ) -> bool {
        // SAFETY: The test passes one live `SnapshotState` for the walk.
        let state = unsafe { &mut *context.cast::<SnapshotState>() };
        state.object_numbers[state.count] = object_number;
        state.count += 1;
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
    fn cross_ref_table_state_should_own_mutation_size_and_overlay_semantics() {
        let mut current = CrossRefTableState::default();
        current.add_normal(3, 4, true, 90);
        current.add_normal(3, 3, false, 12);
        assert_eq!(
            Some(&CrossRefObjectInfo {
                object_type: 1,
                is_object_stream: true,
                generation: 4,
                position: 90,
                ..CrossRefObjectInfo::default()
            }),
            current.objects.get(&3)
        );
        current.set_free(3, 0);
        assert!(current.objects[&3].is_object_stream);
        current.add_compressed(5, 9, 2);
        assert_eq!(2, current.objects[&5].object_type);
        assert!(current.objects[&9].is_object_stream);
        current.add_normal(10, 0, true, 200);

        let mut top = CrossRefTableState::default();
        top.add_normal(3, 0, false, 120);
        top.add_normal(7, 0, false, 140);
        top.add_normal(10, 0, false, 220);
        current.overlay(&mut top);
        assert_eq!(1, current.objects[&3].object_type);
        assert!(!current.objects[&3].is_object_stream);
        assert!(current.objects.contains_key(&5));
        assert!(current.objects.contains_key(&7));
        assert!(current.objects[&10].is_object_stream);
        assert!(top.objects.is_empty());

        current.set_size(7);
        assert!(current.objects.contains_key(&6));
        assert!(!current.objects.contains_key(&7));
        current.set_size(0);
        assert!(current.objects.is_empty());
    }

    #[test]
    fn cross_ref_table_ffi_should_round_trip_and_snapshot_order() {
        let table = pdfium_rust_cross_ref_table_new();
        assert!(!table.is_null());
        // SAFETY: `table` remains live until the final destroy call.
        unsafe {
            assert!(pdfium_rust_cross_ref_table_add_normal(table, 8, 2, false, 88));
            assert!(pdfium_rust_cross_ref_table_add_compressed(table, 4, 8, 1));
        }

        let (mut object_type, mut is_stream, mut generation, mut position) = (0, false, 0, 0);
        let (mut archive_number, mut archive_index) = (0, 0);
        // SAFETY: The table and every output remain live for the call.
        assert!(unsafe {
            pdfium_rust_cross_ref_table_get(
                table,
                4,
                &mut object_type,
                &mut is_stream,
                &mut generation,
                &mut position,
                &mut archive_number,
                &mut archive_index,
            )
        });
        assert_eq!(
            (2, false, 0, 0, 8, 1),
            (object_type, is_stream, generation, position, archive_number, archive_index)
        );

        let mut snapshot = SnapshotState { object_numbers: [0; 4], count: 0 };
        // SAFETY: The table and callback context remain live synchronously.
        assert!(unsafe {
            pdfium_rust_cross_ref_table_snapshot(
                table,
                (&mut snapshot as *mut SnapshotState).cast(),
                Some(record_snapshot),
            )
        });
        assert_eq!(&[4, 8], &snapshot.object_numbers[..snapshot.count]);
        // SAFETY: This transfers the live allocation back to Rust exactly once.
        unsafe { pdfium_rust_cross_ref_table_destroy(table) };

        assert!(!unsafe {
            pdfium_rust_cross_ref_table_get(
                core::ptr::null(),
                0,
                &mut object_type,
                &mut is_stream,
                &mut generation,
                &mut position,
                &mut archive_number,
                &mut archive_index,
            )
        });
    }

    #[test]
    fn indirect_object_index_should_own_parse_replace_delete_and_numbering() {
        let mut index = IndirectObjectIndexState::default();
        assert_eq!((0, 0), index.reserve_parse(7));
        assert_eq!((1, 0), index.reserve_parse(7));
        assert!(!index.finish_parse(8, 100));
        assert!(!index.finish_parse(7, 0));
        assert!(index.finish_parse(7, 100));
        assert_eq!((2, 100), index.reserve_parse(7));
        assert_eq!(7, index.last_object_number);

        assert_eq!(Some((8, None)), index.add(200));
        assert_eq!(None, index.add(0));
        assert_eq!(None, index.replace(7, 300, 2, Some(2)));
        assert_eq!(Some(Some(100)), index.replace(7, 300, 3, Some(2)));
        assert_eq!(Some(None), index.replace(12, 400, 0, None));
        assert_eq!(12, index.last_object_number);
        assert_eq!(Some(300), index.delete(7));
        assert_eq!(None, index.delete(7));

        assert_eq!((0, 0), index.reserve_parse(5));
        assert!(index.cancel_parse(5));
        assert!(!index.cancel_parse(5));

        index.last_object_number = u32::MAX;
        assert_eq!(Some((0, None)), index.add(500));
        assert_eq!(Some(500), index.objects[&0]);
    }

    #[test]
    fn pdf_number_state_should_preserve_pdfium_scalar_semantics() {
        let cases = [
            (b"".as_slice(), true, 0, 0.0),
            (b"123x".as_slice(), true, 123, 123.0),
            (b"1e2".as_slice(), true, 1, 1.0),
            (b"4294967295".as_slice(), true, -1, 4_294_967_296.0),
            (b"4294967296".as_slice(), true, 0, 0.0),
            (b"+2147483647".as_slice(), true, i32::MAX, i32::MAX as f32),
            (b"-2147483648".as_slice(), true, i32::MIN, i32::MIN as f32),
            (b"+2147483648".as_slice(), true, 0, 0.0),
            (b"-2147483649".as_slice(), true, 0, 0.0),
            (b"38.895285".as_slice(), false, 38, 38.89528656005859375),
            (b"+-100.0".as_slice(), false, -100, -100.0),
            (b"++100.0".as_slice(), false, 100, 100.0),
            (b"invalid.".as_slice(), false, 0, 0.0),
        ];
        for (input, is_integer, signed, float) in cases {
            let number = PdfNumberState::from_bytes(input);
            assert_eq!(is_integer, number.is_integer(), "{input:?}");
            assert_eq!(signed, number.get_signed(), "{input:?}");
            assert_eq!(float, number.get_float(), "{input:?}");
        }

        let positive = PdfNumberState { value: PdfNumberValue::Float(f32::INFINITY) };
        let negative = PdfNumberState { value: PdfNumberValue::Float(f32::NEG_INFINITY) };
        let nan = PdfNumberState { value: PdfNumberValue::Float(f32::NAN) };
        assert_eq!(i32::MAX, positive.get_signed());
        assert_eq!(i32::MIN, negative.get_signed());
        assert_eq!(0, nan.get_signed());
    }

    #[test]
    fn pdf_boolean_state_should_accept_only_the_true_keyword() {
        for (input, expected) in [
            (b"true".as_slice(), true),
            (b"false".as_slice(), false),
            (b"True".as_slice(), false),
            (b"true\0".as_slice(), false),
            (b"".as_slice(), false),
        ] {
            let mut state = PdfBooleanState { value: !expected };
            // SAFETY: The state and borrowed input remain live for the call.
            assert!(unsafe {
                pdfium_rust_pdf_boolean_set_string(&mut state, input.as_ptr(), input.len())
            });
            assert_eq!(expected, state.value);
        }
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
