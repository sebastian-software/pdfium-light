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

pub struct PdfReferenceState {
    object_number: u32,
}

#[derive(Default)]
pub struct PdfArrayState {
    objects: Vec<usize>,
}

#[derive(Default)]
pub struct PdfDictionaryState {
    objects: BTreeMap<Vec<u8>, usize>,
}

pub struct PdfStringState {
    bytes: Vec<u8>,
    output_is_hex: bool,
}

pub struct PdfStreamDataState {
    bytes: Vec<u8>,
}

#[derive(Default)]
pub struct DocumentPageIndexState {
    object_numbers: Vec<u32>,
}

struct DocumentPageTraversalEntry {
    handle: usize,
    child_index: usize,
}

#[derive(Default)]
pub struct DocumentPageTraversalState {
    stack: Vec<DocumentPageTraversalEntry>,
    reached_max_level: bool,
    next_page: i32,
}

type PdfDictionarySnapshotCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, *const u8, usize, usize) -> bool;

type DocumentPageDescribeCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, *mut i32, *mut usize) -> bool;
type DocumentPageChildCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    usize,
    *mut usize,
    *mut u8,
    *mut bool,
) -> bool;
type DocumentPageNormalizeCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, u8) -> bool;
type DocumentPageSetCountCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, i32) -> bool;
type DocumentPageFindDescribeCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    *mut bool,
    *mut i32,
    *mut u32,
    *mut usize,
) -> bool;
type DocumentPageFindChildCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, usize, *mut usize, *mut u32) -> bool;
type DocumentPageMutationDescribeCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, *mut usize) -> bool;
type DocumentPageMutationChildCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    usize,
    *mut usize,
    *mut u8,
    *mut i32,
) -> bool;
type DocumentPageTraversalRetainCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize) -> bool;
type DocumentPageTraversalReleaseCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize) -> bool;
type DocumentPageTraversalDescribeCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    *mut bool,
    *mut u8,
    *mut u32,
    *mut usize,
) -> bool;
type DocumentPageTraversalChildCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    usize,
    *mut usize,
    *mut bool,
    *mut u32,
) -> bool;
type DocumentPageTraversalCacheCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, i32, u32) -> bool;
type DocumentPageTraversalSelectCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize) -> bool;
type RedactionRectCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    *mut f32,
    *mut f32,
    *mut f32,
    *mut f32,
) -> bool;
type RedactionObjectCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    *mut bool,
    *mut u8,
    *mut f32,
    *mut f32,
    *mut f32,
    *mut f32,
) -> bool;
type PageObjectStreamCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, *mut i32) -> bool;
type PageObjectDescribeCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, *mut usize, *mut i32) -> bool;
type PageObjectActiveCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, *mut bool) -> bool;

#[derive(Clone, Copy)]
struct RedactionRect {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

pub struct RedactionPlanState {
    status: i32,
    removals: Vec<usize>,
}

fn redaction_plan(
    has_rects: bool,
    rect_count: usize,
    object_count: usize,
    mut get_rect: impl FnMut(usize) -> Option<RedactionRect>,
    mut get_object: impl FnMut(usize) -> Option<(bool, u8, RedactionRect)>,
) -> Option<RedactionPlanState> {
    if !has_rects || rect_count == 0 {
        return Some(RedactionPlanState { status: 2, removals: Vec::new() });
    }
    let mut rects = Vec::with_capacity(rect_count);
    for index in 0..rect_count {
        let rect = get_rect(index)?;
        if !(rect.left < rect.right && rect.bottom < rect.top) {
            return Some(RedactionPlanState { status: 2, removals: Vec::new() });
        }
        rects.push(rect);
    }

    let mut removals = Vec::new();
    for index in 0..object_count {
        let (active, object_type, object_rect) = get_object(index)?;
        if !active {
            continue;
        }
        let mut intersects = false;
        let mut fully_covered = false;
        for rect in &rects {
            if rect.left < object_rect.right
                && rect.right > object_rect.left
                && rect.bottom < object_rect.top
                && rect.top > object_rect.bottom
            {
                intersects = true;
                if rect.left <= object_rect.left
                    && rect.right >= object_rect.right
                    && rect.bottom <= object_rect.bottom
                    && rect.top >= object_rect.top
                {
                    fully_covered = true;
                    break;
                }
            }
        }
        if !intersects {
            continue;
        }
        if !fully_covered {
            return Some(RedactionPlanState { status: 5, removals: Vec::new() });
        }
        if !matches!(object_type, 1..=3) {
            return Some(RedactionPlanState { status: 4, removals: Vec::new() });
        }
        removals.push(index);
    }
    let status = if removals.is_empty() { 3 } else { 0 };
    Some(RedactionPlanState { status, removals })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PageObjectInsertPlan {
    allowed: bool,
    content_stream: i32,
    mark_dirty: bool,
}

fn page_object_insert_plan(
    index: usize,
    object_count: usize,
    content_stream: i32,
    mut get_neighbor_stream: impl FnMut(usize) -> Option<i32>,
) -> Option<PageObjectInsertPlan> {
    if index > object_count {
        return Some(PageObjectInsertPlan { allowed: false, content_stream, mark_dirty: false });
    }
    if index < object_count && content_stream == -1 {
        let neighbor_stream = get_neighbor_stream(index)?;
        if neighbor_stream != -1 {
            return Some(PageObjectInsertPlan {
                allowed: true,
                content_stream: neighbor_stream,
                mark_dirty: true,
            });
        }
    }
    Some(PageObjectInsertPlan { allowed: true, content_stream, mark_dirty: false })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PageObjectRemovePlan {
    found: bool,
    index: usize,
    content_stream: i32,
}

fn page_object_remove_plan(
    object_count: usize,
    target_handle: usize,
    mut describe: impl FnMut(usize) -> Option<(usize, i32)>,
) -> Option<PageObjectRemovePlan> {
    for index in 0..object_count {
        let (handle, content_stream) = describe(index)?;
        if handle == target_handle {
            return Some(PageObjectRemovePlan { found: true, index, content_stream });
        }
    }
    Some(PageObjectRemovePlan { found: false, index: 0, content_stream: -1 })
}

fn page_object_active_update(current: bool, requested: bool) -> (bool, bool) {
    (requested, current != requested)
}

fn page_object_active_count(
    object_count: usize,
    mut get_active: impl FnMut(usize) -> Option<bool>,
) -> Option<usize> {
    let mut active_count = 0usize;
    for index in 0..object_count {
        if get_active(index)? {
            active_count = active_count.checked_add(1)?;
        }
    }
    Some(active_count)
}

fn page_object_matrix_route(object_type: u8) -> Option<u8> {
    matches!(object_type, 1..=3 | 5).then_some(object_type)
}

fn page_object_matrix_dirty(
    object_type: u8,
    original: [f32; 6],
    replacement: [f32; 6],
) -> Option<bool> {
    page_object_matrix_route(object_type)?;
    Some(object_type != 3 || original != replacement)
}

fn page_object_rotated_bounds(
    object_type: u8,
    matrix: [f32; 6],
    bounds: [f32; 4],
) -> Option<[f32; 8]> {
    if !matches!(object_type, 1 | 3) {
        return None;
    }
    let transform = |x: f32, y: f32| {
        [matrix[0] * x + matrix[2] * y + matrix[4], matrix[1] * x + matrix[3] * y + matrix[5]]
    };
    let bottom_left = transform(bounds[0], bounds[1]);
    let bottom_right = transform(bounds[2], bounds[1]);
    let top_right = transform(bounds[2], bounds[3]);
    let top_left = transform(bounds[0], bounds[3]);
    Some([
        bottom_left[0],
        bottom_left[1],
        bottom_right[0],
        bottom_right[1],
        top_right[0],
        top_right[1],
        top_left[0],
        top_left[1],
    ])
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
    fn contains_handle(&self, handle: usize) -> bool {
        self.objects.values().any(|value| *value == Some(handle))
    }
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
/// `state` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_indirect_object_index_contains_handle(
    state: *const IndirectObjectIndexState,
    handle: usize,
) -> bool {
    unsafe { state.as_ref() }.is_some_and(|state| state.contains_handle(handle))
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
    let c_string = input.split(|byte| *byte == 0).next().unwrap_or_default();
    state.value = c_string == b"true";
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_pdf_reference_new(object_number: u32) -> *mut PdfReferenceState {
    Box::into_raw(Box::new(PdfReferenceState { object_number }))
}

/// # Safety
///
/// `state` must be null or a uniquely owned pointer returned by
/// `pdfium_rust_pdf_reference_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_reference_destroy(state: *mut PdfReferenceState) {
    if !state.is_null() {
        // SAFETY: The caller transfers the unique allocation back to Rust.
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
///
/// `state` and `output` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_reference_get(
    state: *const PdfReferenceState,
    output: *mut u32,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if output.is_null() {
        return false;
    }
    // SAFETY: The checked output points to one writable `u32`.
    unsafe { *output = state.object_number };
    true
}

/// # Safety
///
/// `state` must point to a live value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_reference_set(
    state: *mut PdfReferenceState,
    object_number: u32,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    state.object_number = object_number;
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_pdf_array_new() -> *mut PdfArrayState {
    Box::into_raw(Box::new(PdfArrayState::default()))
}

/// # Safety
///
/// `state` must be null or a uniquely owned pointer returned by
/// `pdfium_rust_pdf_array_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_destroy(state: *mut PdfArrayState) {
    if !state.is_null() {
        // SAFETY: The caller transfers the unique allocation back to Rust.
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
///
/// `state` and `output` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_len(
    state: *const PdfArrayState,
    output: *mut usize,
) -> bool {
    let (Some(state), Some(output)) = ((unsafe { state.as_ref() }), (unsafe { output.as_mut() }))
    else {
        return false;
    };
    *output = state.objects.len();
    true
}

/// # Safety
///
/// `state` and `output` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_get(
    state: *const PdfArrayState,
    index: usize,
    output: *mut usize,
) -> bool {
    let (Some(state), Some(output)) = ((unsafe { state.as_ref() }), (unsafe { output.as_mut() }))
    else {
        return false;
    };
    let Some(handle) = state.objects.get(index) else {
        return false;
    };
    *output = *handle;
    true
}

/// # Safety
///
/// `state` must point to a live value and `handle` must be nonzero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_append(
    state: *mut PdfArrayState,
    handle: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if handle == 0 {
        return false;
    }
    state.objects.push(handle);
    true
}

/// # Safety
///
/// `state` and `old_handle` must remain valid and `handle` must be nonzero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_set(
    state: *mut PdfArrayState,
    index: usize,
    handle: usize,
    old_handle: *mut usize,
) -> bool {
    let (Some(state), Some(old_handle)) =
        ((unsafe { state.as_mut() }), (unsafe { old_handle.as_mut() }))
    else {
        return false;
    };
    if handle == 0 || index >= state.objects.len() {
        return false;
    }
    *old_handle = std::mem::replace(&mut state.objects[index], handle);
    true
}

/// # Safety
///
/// `state` must point to a live value and `handle` must be nonzero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_insert(
    state: *mut PdfArrayState,
    index: usize,
    handle: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if handle == 0 || index > state.objects.len() {
        return false;
    }
    state.objects.insert(index, handle);
    true
}

/// # Safety
///
/// `state` and `old_handle` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_remove(
    state: *mut PdfArrayState,
    index: usize,
    old_handle: *mut usize,
) -> bool {
    let (Some(state), Some(old_handle)) =
        ((unsafe { state.as_mut() }), (unsafe { old_handle.as_mut() }))
    else {
        return false;
    };
    if index >= state.objects.len() {
        return false;
    }
    *old_handle = state.objects.remove(index);
    true
}

/// # Safety
///
/// `state` must point to a live value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_clear(state: *mut PdfArrayState) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    state.objects.clear();
    true
}

/// # Safety
/// `state` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_array_contains_handle(
    state: *const PdfArrayState,
    handle: usize,
) -> bool {
    unsafe { state.as_ref() }.is_some_and(|state| state.objects.contains(&handle))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_pdf_dictionary_new() -> *mut PdfDictionaryState {
    Box::into_raw(Box::new(PdfDictionaryState::default()))
}

/// # Safety
/// `state` must be null or uniquely owned from
/// `pdfium_rust_pdf_dictionary_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_dictionary_destroy(state: *mut PdfDictionaryState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` and `output` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_dictionary_len(
    state: *const PdfDictionaryState,
    output: *mut usize,
) -> bool {
    let (Some(state), Some(output)) = ((unsafe { state.as_ref() }), (unsafe { output.as_mut() }))
    else {
        return false;
    };
    *output = state.objects.len();
    true
}

/// # Safety
/// Nonempty keys must identify readable bytes; `output` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_dictionary_get(
    state: *const PdfDictionaryState,
    key: *const u8,
    key_len: usize,
    output: *mut usize,
) -> bool {
    let (Some(state), Some(output)) = ((unsafe { state.as_ref() }), (unsafe { output.as_mut() }))
    else {
        return false;
    };
    if key_len != 0 && key.is_null() {
        return false;
    }
    let key = if key_len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { key };
    let key = unsafe { core::slice::from_raw_parts(key, key_len) };
    let Some(handle) = state.objects.get(key) else {
        return false;
    };
    *output = *handle;
    true
}

/// # Safety
/// Nonempty keys must identify readable bytes; `old_handle` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_dictionary_set(
    state: *mut PdfDictionaryState,
    key: *const u8,
    key_len: usize,
    handle: usize,
    old_handle: *mut usize,
) -> bool {
    let (Some(state), Some(old_handle)) =
        ((unsafe { state.as_mut() }), (unsafe { old_handle.as_mut() }))
    else {
        return false;
    };
    if handle == 0 || (key_len != 0 && key.is_null()) {
        return false;
    }
    let key = if key_len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { key };
    let key = unsafe { core::slice::from_raw_parts(key, key_len) };
    *old_handle = state.objects.insert(key.to_vec(), handle).unwrap_or(0);
    true
}

/// # Safety
/// Nonempty keys must identify readable bytes; `old_handle` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_dictionary_remove(
    state: *mut PdfDictionaryState,
    key: *const u8,
    key_len: usize,
    old_handle: *mut usize,
) -> bool {
    let (Some(state), Some(old_handle)) =
        ((unsafe { state.as_mut() }), (unsafe { old_handle.as_mut() }))
    else {
        return false;
    };
    if key_len != 0 && key.is_null() {
        return false;
    }
    let key = if key_len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { key };
    let key = unsafe { core::slice::from_raw_parts(key, key_len) };
    let Some(handle) = state.objects.remove(key) else {
        return false;
    };
    *old_handle = handle;
    true
}

/// # Safety
/// `state`, `context`, and `callback` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_dictionary_snapshot(
    state: *const PdfDictionaryState,
    context: *mut core::ffi::c_void,
    callback: Option<PdfDictionarySnapshotCallback>,
) -> bool {
    let (Some(state), Some(callback)) = ((unsafe { state.as_ref() }), callback) else {
        return false;
    };
    for (key, handle) in &state.objects {
        if !unsafe { callback(context, key.as_ptr(), key.len(), *handle) } {
            return false;
        }
    }
    true
}

/// # Safety
/// `state` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_dictionary_contains_handle(
    state: *const PdfDictionaryState,
    handle: usize,
) -> bool {
    unsafe { state.as_ref() }
        .is_some_and(|state| state.objects.values().any(|value| *value == handle))
}

/// # Safety
/// Nonempty input must identify readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_string_new(
    data: *const u8,
    len: usize,
    output_is_hex: bool,
) -> *mut PdfStringState {
    if len != 0 && data.is_null() {
        return core::ptr::null_mut();
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    let bytes = unsafe { core::slice::from_raw_parts(data, len) }.to_vec();
    Box::into_raw(Box::new(PdfStringState { bytes, output_is_hex }))
}

/// # Safety
/// `state` must be null or uniquely owned from `pdfium_rust_pdf_string_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_string_destroy(state: *mut PdfStringState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` and `output` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_string_is_hex(
    state: *const PdfStringState,
    output: *mut bool,
) -> bool {
    let (Some(state), Some(output)) = ((unsafe { state.as_ref() }), (unsafe { output.as_mut() }))
    else {
        return false;
    };
    *output = state.output_is_hex;
    true
}

/// # Safety
/// Nonempty input must identify readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_string_equals(
    state: *const PdfStringState,
    data: *const u8,
    len: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if len != 0 && data.is_null() {
        return false;
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    state.bytes == unsafe { core::slice::from_raw_parts(data, len) }
}

/// # Safety
/// Nonempty input must identify readable bytes and `state` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_string_set(
    state: *mut PdfStringState,
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
    state.bytes = unsafe { core::slice::from_raw_parts(data, len) }.to_vec();
    true
}

/// # Safety
/// Nonempty input must identify readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_stream_data_new(
    data: *const u8,
    len: usize,
) -> *mut PdfStreamDataState {
    if len != 0 && data.is_null() {
        return core::ptr::null_mut();
    }
    let data = if len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { data };
    let bytes = unsafe { core::slice::from_raw_parts(data, len) }.to_vec();
    Box::into_raw(Box::new(PdfStreamDataState { bytes }))
}

/// # Safety
/// `state` must be null or uniquely owned from
/// `pdfium_rust_pdf_stream_data_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_stream_data_destroy(state: *mut PdfStreamDataState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state`, `data`, and `len` must remain valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_pdf_stream_data_span(
    state: *const PdfStreamDataState,
    data: *mut *const u8,
    len: *mut usize,
) -> bool {
    let (Some(state), Some(data), Some(len)) =
        (unsafe { state.as_ref() }, unsafe { data.as_mut() }, unsafe { len.as_mut() })
    else {
        return false;
    };
    *data = state.bytes.as_ptr();
    *len = state.bytes.len();
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_document_page_index_new() -> *mut DocumentPageIndexState {
    Box::into_raw(Box::new(DocumentPageIndexState::default()))
}

/// # Safety
/// `state` must be null or uniquely owned from
/// `pdfium_rust_document_page_index_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_index_destroy(
    state: *mut DocumentPageIndexState,
) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` and `output` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_index_len(
    state: *const DocumentPageIndexState,
    output: *mut usize,
) -> bool {
    let (Some(state), Some(output)) = (unsafe { state.as_ref() }, unsafe { output.as_mut() })
    else {
        return false;
    };
    *output = state.object_numbers.len();
    true
}

/// # Safety
/// `state` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_index_resize(
    state: *mut DocumentPageIndexState,
    len: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    state.object_numbers.resize(len, 0);
    true
}

/// # Safety
/// `state` and `output` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_index_get(
    state: *const DocumentPageIndexState,
    index: usize,
    output: *mut u32,
) -> bool {
    let (Some(state), Some(output)) = (unsafe { state.as_ref() }, unsafe { output.as_mut() })
    else {
        return false;
    };
    let Some(value) = state.object_numbers.get(index) else {
        return false;
    };
    *output = *value;
    true
}

/// # Safety
/// `state` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_index_set(
    state: *mut DocumentPageIndexState,
    index: usize,
    object_number: u32,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    let Some(value) = state.object_numbers.get_mut(index) else {
        return false;
    };
    *value = object_number;
    true
}

/// # Safety
/// `state` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_index_insert(
    state: *mut DocumentPageIndexState,
    index: usize,
    object_number: u32,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if index > state.object_numbers.len() {
        return false;
    }
    state.object_numbers.insert(index, object_number);
    true
}

/// # Safety
/// `state` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_index_remove(
    state: *mut DocumentPageIndexState,
    index: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    if index >= state.object_numbers.len() {
        return false;
    }
    state.object_numbers.remove(index);
    true
}

/// # Safety
/// `state` must remain valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_index_contains(
    state: *const DocumentPageIndexState,
    object_number: u32,
) -> bool {
    unsafe { state.as_ref() }.is_some_and(|state| state.object_numbers.contains(&object_number))
}

struct DocumentPageTraversalCallbacks {
    context: *mut core::ffi::c_void,
    describe: DocumentPageTraversalDescribeCallback,
    child: DocumentPageTraversalChildCallback,
    cache: DocumentPageTraversalCacheCallback,
    retain: DocumentPageTraversalRetainCallback,
    release: DocumentPageTraversalReleaseCallback,
}

impl DocumentPageTraversalState {
    fn clear(
        &mut self,
        context: *mut core::ffi::c_void,
        release: DocumentPageTraversalReleaseCallback,
    ) -> bool {
        let mut success = true;
        while let Some(entry) = self.stack.pop() {
            success &= unsafe { release(context, entry.handle) };
        }
        self.reached_max_level = false;
        self.next_page = 0;
        success
    }

    fn pop_level(&mut self, callbacks: &DocumentPageTraversalCallbacks) -> Result<(), ()> {
        let entry = self.stack.pop().ok_or(())?;
        if !unsafe { (callbacks.release)(callbacks.context, entry.handle) } {
            return Err(());
        }
        Ok(())
    }

    fn traverse_level(
        &mut self,
        page_index: i32,
        pages_to_go: &mut i32,
        level: usize,
        callbacks: &DocumentPageTraversalCallbacks,
    ) -> Result<usize, ()> {
        const LEVEL_MAX: usize = 1024;
        if *pages_to_go < 0 || self.reached_max_level {
            return Ok(0);
        }
        let handle = self.stack.get(level).ok_or(())?.handle;
        let mut has_kids_array = false;
        let mut node_type = 0;
        let mut object_number = 0;
        let mut child_count = 0;
        if !unsafe {
            (callbacks.describe)(
                callbacks.context,
                handle,
                &mut has_kids_array,
                &mut node_type,
                &mut object_number,
                &mut child_count,
            )
        } {
            return Err(());
        }
        if !has_kids_array {
            self.pop_level(callbacks)?;
            if *pages_to_go != 1 || node_type == 1 {
                return Ok(0);
            }
            if node_type != 2
                || !unsafe { (callbacks.cache)(callbacks.context, page_index, object_number) }
            {
                return Err(());
            }
            return Ok(handle);
        }
        if level >= LEVEL_MAX {
            self.pop_level(callbacks)?;
            self.reached_max_level = true;
            return Ok(0);
        }

        let mut page = 0;
        loop {
            let child_index = self.stack.get(level).ok_or(())?.child_index;
            if child_index >= child_count || *pages_to_go == 0 {
                break;
            }
            let mut child_handle = 0;
            let mut child_has_kids = false;
            let mut child_object_number = 0;
            if !unsafe {
                (callbacks.child)(
                    callbacks.context,
                    handle,
                    child_index,
                    &mut child_handle,
                    &mut child_has_kids,
                    &mut child_object_number,
                )
            } {
                return Err(());
            }
            if child_handle == 0 {
                *pages_to_go = (*pages_to_go).checked_sub(1).ok_or(())?;
                self.stack[level].child_index += 1;
                continue;
            }
            if child_handle == handle {
                self.stack[level].child_index += 1;
                continue;
            }
            if !child_has_kids {
                let cache_index =
                    page_index.checked_sub(*pages_to_go).and_then(|v| v.checked_add(1)).ok_or(())?;
                if !unsafe {
                    (callbacks.cache)(callbacks.context, cache_index, child_object_number)
                } {
                    return Err(());
                }
                *pages_to_go = (*pages_to_go).checked_sub(1).ok_or(())?;
                self.stack[level].child_index += 1;
                if *pages_to_go == 0 {
                    page = child_handle;
                    break;
                }
                continue;
            }

            if self.stack.len() == level + 1 {
                if !unsafe { (callbacks.retain)(callbacks.context, child_handle) } {
                    return Err(());
                }
                self.stack
                    .push(DocumentPageTraversalEntry { handle: child_handle, child_index: 0 });
            }
            let child_page = self.traverse_level(page_index, pages_to_go, level + 1, callbacks)?;
            if self.stack.len() == level + 1 {
                self.stack[level].child_index += 1;
            }
            if self.stack.len() != level + 1 || *pages_to_go == 0 || self.reached_max_level {
                page = child_page;
                break;
            }
        }
        if self.stack.get(level).is_some_and(|entry| entry.child_index == child_count) {
            self.pop_level(callbacks)?;
        }
        Ok(page)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_document_page_traversal_new() -> *mut DocumentPageTraversalState {
    Box::into_raw(Box::new(DocumentPageTraversalState::default()))
}

/// # Safety
/// `state` must be null or returned by the matching constructor and cleared.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_traversal_free(
    state: *mut DocumentPageTraversalState,
) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` and callbacks must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_traversal_clear(
    state: *mut DocumentPageTraversalState,
    context: *mut core::ffi::c_void,
    release: Option<DocumentPageTraversalReleaseCallback>,
) -> bool {
    let (Some(state), Some(release)) = (unsafe { state.as_mut() }, release) else {
        return false;
    };
    state.clear(context, release)
}

/// # Safety
/// `state` and callbacks must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_traversal_reset(
    state: *mut DocumentPageTraversalState,
    root_handle: usize,
    context: *mut core::ffi::c_void,
    retain: Option<DocumentPageTraversalRetainCallback>,
    release: Option<DocumentPageTraversalReleaseCallback>,
) -> bool {
    let (Some(state), Some(retain), Some(release)) = (unsafe { state.as_mut() }, retain, release)
    else {
        return false;
    };
    if !state.clear(context, release)
        || root_handle == 0
        || !unsafe { retain(context, root_handle) }
    {
        return false;
    }
    state.stack.push(DocumentPageTraversalEntry { handle: root_handle, child_index: 0 });
    true
}

/// # Safety
/// `state`, callbacks, and `found` must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_traversal_run(
    state: *mut DocumentPageTraversalState,
    page_index: i32,
    context: *mut core::ffi::c_void,
    describe: Option<DocumentPageTraversalDescribeCallback>,
    child: Option<DocumentPageTraversalChildCallback>,
    cache: Option<DocumentPageTraversalCacheCallback>,
    select: Option<DocumentPageTraversalSelectCallback>,
    retain: Option<DocumentPageTraversalRetainCallback>,
    release: Option<DocumentPageTraversalReleaseCallback>,
    found: *mut bool,
) -> bool {
    let (
        Some(state),
        Some(describe),
        Some(child),
        Some(cache),
        Some(select),
        Some(retain),
        Some(release),
        Some(found),
    ) = (unsafe { state.as_mut() }, describe, child, cache, select, retain, release, unsafe {
        found.as_mut()
    })
    else {
        return false;
    };
    let callbacks =
        DocumentPageTraversalCallbacks { context, describe, child, cache, retain, release };
    let Some(mut pages_to_go) =
        page_index.checked_sub(state.next_page).and_then(|v| v.checked_add(1))
    else {
        return false;
    };
    let page = if state.stack.is_empty() {
        0
    } else {
        match state.traverse_level(page_index, &mut pages_to_go, 0, &callbacks) {
            Ok(page) => page,
            Err(()) => return false,
        }
    };
    let Some(next_page) = page_index.checked_add(1) else {
        return false;
    };
    state.next_page = next_page;
    *found = page != 0;
    page == 0 || unsafe { select(context, page) }
}

/// Plans public page movement without touching document state.
///
/// # Safety
/// Nonempty `page_indices` and `deletion_order` must identify `len` readable
/// and writable values respectively.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_move_page_plan(
    page_indices: *const i32,
    len: usize,
    num_pages: usize,
    destination: i32,
    deletion_order: *mut i32,
) -> bool {
    if len == 0
        || len > num_pages
        || page_indices.is_null()
        || deletion_order.is_null()
        || destination < 0
        || destination as usize > num_pages - len
    {
        return false;
    }
    let indices = unsafe { core::slice::from_raw_parts(page_indices, len) };
    let mut unique = std::collections::BTreeSet::new();
    for &index in indices {
        if index < 0 || index as usize >= num_pages || !unique.insert(index) {
            return false;
        }
    }
    let output = unsafe { core::slice::from_raw_parts_mut(deletion_order, len) };
    for (target, index) in output.iter_mut().zip(unique.iter().rev()) {
        *target = *index;
    }
    true
}

struct DocumentPageCountCallbacks {
    context: *mut core::ffi::c_void,
    describe: DocumentPageDescribeCallback,
    child: DocumentPageChildCallback,
    normalize: DocumentPageNormalizeCallback,
    set_count: DocumentPageSetCountCallback,
}

fn count_document_pages(
    handle: usize,
    depth: usize,
    visited: &mut std::collections::BTreeSet<usize>,
    callbacks: &DocumentPageCountCallbacks,
) -> Option<i32> {
    const PAGE_MAX: i32 = 0xF_FFFF;
    const LEVEL_MAX: usize = 1024;
    if handle == 0 || depth >= LEVEL_MAX {
        return None;
    }
    let mut count_hint = 0;
    let mut child_count = 0;
    if !unsafe {
        (callbacks.describe)(callbacks.context, handle, &mut count_hint, &mut child_count)
    } {
        return None;
    }
    if count_hint > 0 && count_hint < PAGE_MAX {
        return Some(count_hint);
    }

    let mut count: i32 = 0;
    for index in 0..child_count {
        let mut child_handle = 0;
        let mut node_type = 0;
        let mut has_kids = false;
        if !unsafe {
            (callbacks.child)(
                callbacks.context,
                handle,
                index,
                &mut child_handle,
                &mut node_type,
                &mut has_kids,
            )
        } {
            return None;
        }
        if child_handle == 0 || visited.contains(&child_handle) {
            continue;
        }
        let effective_type = match node_type {
            1 => 1,
            2 => 2,
            _ => {
                let value = if has_kids { 1 } else { 2 };
                if !unsafe { (callbacks.normalize)(callbacks.context, child_handle, value) } {
                    return None;
                }
                value
            }
        };
        if effective_type == 1 {
            visited.insert(child_handle);
            let child_result = count_document_pages(child_handle, depth + 1, visited, callbacks);
            visited.remove(&child_handle);
            count = count.checked_add(child_result?)?;
        } else {
            count += 1;
        }
        if count >= PAGE_MAX {
            return None;
        }
    }
    if !unsafe { (callbacks.set_count)(callbacks.context, handle, count) } {
        return None;
    }
    Some(count)
}

/// # Safety
/// All callbacks and `output` must remain valid for the synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_count_pages(
    root_handle: usize,
    context: *mut core::ffi::c_void,
    describe: Option<DocumentPageDescribeCallback>,
    child: Option<DocumentPageChildCallback>,
    normalize: Option<DocumentPageNormalizeCallback>,
    set_count: Option<DocumentPageSetCountCallback>,
    output: *mut i32,
) -> bool {
    let (Some(describe), Some(child), Some(normalize), Some(set_count), Some(output)) =
        (describe, child, normalize, set_count, unsafe { output.as_mut() })
    else {
        return false;
    };
    let callbacks = DocumentPageCountCallbacks { context, describe, child, normalize, set_count };
    let mut visited = std::collections::BTreeSet::from([root_handle]);
    let Some(count) = count_document_pages(root_handle, 0, &mut visited, &callbacks) else {
        return false;
    };
    *output = count;
    true
}

struct DocumentPageFindCallbacks {
    context: *mut core::ffi::c_void,
    describe: DocumentPageFindDescribeCallback,
    child: DocumentPageFindChildCallback,
}

fn find_document_page_index(
    handle: usize,
    target_object_number: u32,
    skip_count: &mut u32,
    index: &mut i32,
    depth: usize,
    callbacks: &DocumentPageFindCallbacks,
) -> Option<i32> {
    const LEVEL_MAX: usize = 1024;
    let mut has_kids = false;
    let mut count_hint = 0;
    let mut object_number = 0;
    let mut child_count = 0;
    if handle == 0
        || !unsafe {
            (callbacks.describe)(
                callbacks.context,
                handle,
                &mut has_kids,
                &mut count_hint,
                &mut object_number,
                &mut child_count,
            )
        }
    {
        return None;
    }
    if !has_kids {
        if object_number == target_object_number {
            return Some(*index);
        }
        if *skip_count != 0 {
            *skip_count -= 1;
        }
        *index += 1;
        return Some(-1);
    }
    if depth >= LEVEL_MAX {
        return Some(-1);
    }

    let count = count_hint as usize;
    if count <= *skip_count as usize {
        *skip_count -= count as u32;
        *index += count as i32;
        return Some(-1);
    }
    if count != 0 && count == child_count {
        for child_index in 0..count {
            let mut child_handle = 0;
            let mut reference_object_number = 0;
            if !unsafe {
                (callbacks.child)(
                    callbacks.context,
                    handle,
                    child_index,
                    &mut child_handle,
                    &mut reference_object_number,
                )
            } {
                return None;
            }
            if reference_object_number == target_object_number {
                return Some(*index + child_index as i32);
            }
        }
    }
    for child_index in 0..child_count {
        let mut child_handle = 0;
        let mut reference_object_number = 0;
        if !unsafe {
            (callbacks.child)(
                callbacks.context,
                handle,
                child_index,
                &mut child_handle,
                &mut reference_object_number,
            )
        } {
            return None;
        }
        if child_handle == 0 || child_handle == handle {
            continue;
        }
        let found = find_document_page_index(
            child_handle,
            target_object_number,
            skip_count,
            index,
            depth + 1,
            callbacks,
        )?;
        if found >= 0 {
            return Some(found);
        }
    }
    Some(-1)
}

/// # Safety
/// All callbacks and `output` must remain valid for the synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_find_page_index(
    root_handle: usize,
    target_object_number: u32,
    initial_skip_count: u32,
    context: *mut core::ffi::c_void,
    describe: Option<DocumentPageFindDescribeCallback>,
    child: Option<DocumentPageFindChildCallback>,
    output: *mut i32,
) -> bool {
    let (Some(describe), Some(child), Some(output)) = (describe, child, unsafe { output.as_mut() })
    else {
        return false;
    };
    let callbacks = DocumentPageFindCallbacks { context, describe, child };
    let mut skip_count = initial_skip_count;
    let mut index = 0;
    let Some(found) = find_document_page_index(
        root_handle,
        target_object_number,
        &mut skip_count,
        &mut index,
        0,
        &callbacks,
    ) else {
        return false;
    };
    *output = found;
    true
}

fn parse_sdk_page_number(input: &[u8]) -> Option<u32> {
    if input.is_empty() {
        return Some(0);
    }
    let mut value: u32 = 0;
    for &byte in input {
        if !byte.is_ascii_digit() {
            return Some(0);
        }
        value = value.checked_mul(10)?.checked_add(u32::from(byte - b'0'))?;
        if value > i32::MAX as u32 {
            return None;
        }
    }
    Some(value)
}

fn parse_sdk_page_range(input: &[u8], page_count: u32) -> Option<Vec<u32>> {
    if input.iter().any(|byte| !matches!(*byte, b' ' | b'0'..=b'9' | b'-' | b',')) {
        return Some(Vec::new());
    }
    let stripped: Vec<u8> = input.iter().copied().filter(|byte| *byte != b' ').collect();
    let mut result = Vec::new();
    for entry in stripped.split(|byte| *byte == b',') {
        let mut args = entry.split(|byte| *byte == b'-');
        let first = args.next().unwrap_or_default();
        let second = args.next();
        if args.next().is_some() {
            return Some(Vec::new());
        }
        let first_number = parse_sdk_page_number(first)?;
        match second {
            None => {
                if first_number == 0 || first_number > page_count {
                    return Some(Vec::new());
                }
                result.push(first_number - 1);
            }
            Some(last) => {
                let last_number = parse_sdk_page_number(last)?;
                if first_number == 0
                    || last_number == 0
                    || first_number > last_number
                    || last_number > page_count
                {
                    return Some(Vec::new());
                }
                result.extend((first_number..=last_number).map(|page| page - 1));
            }
        }
    }
    Some(result)
}

/// Parses the SDK page-range grammar and copies the zero-based page indices.
///
/// A null output with zero capacity performs the sizing pass. Decimal overflow
/// rejects the boundary so C++ can preserve its platform `atoi()` oracle.
///
/// # Safety
/// `input` and non-null `output` must identify readable/writable spans.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_sdk_parse_page_range(
    input: *const u8,
    input_len: usize,
    page_count: u32,
    output: *mut u32,
    output_capacity: usize,
    output_len: *mut usize,
) -> bool {
    let Some(output_len) = (unsafe { output_len.as_mut() }) else {
        return false;
    };
    if input_len != 0 && input.is_null() {
        return false;
    }
    let input =
        if input_len == 0 { &[] } else { unsafe { core::slice::from_raw_parts(input, input_len) } };
    let Some(result) = parse_sdk_page_range(input, page_count) else {
        return false;
    };
    *output_len = result.len();
    if output_capacity == 0 {
        return output.is_null();
    }
    if output.is_null() || output_capacity < result.len() {
        return false;
    }
    unsafe { core::slice::from_raw_parts_mut(output, result.len()) }.copy_from_slice(&result);
    true
}

/// Computes a public SDK byte-string result length and conditionally copies a
/// trailing-NUL representation without partially modifying short outputs.
///
/// # Safety
/// Nonempty input/output spans must identify readable/writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_sdk_nul_terminate(
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    output_capacity: usize,
    required_len: *mut usize,
) -> bool {
    let Some(required_len) = (unsafe { required_len.as_mut() }) else {
        return false;
    };
    if input_len != 0 && input.is_null() {
        return false;
    }
    let Some(required) = input_len.checked_add(1) else {
        return false;
    };
    *required_len = required;
    if output_capacity < required {
        return output_capacity == 0 || !output.is_null();
    }
    if output.is_null() {
        return false;
    }
    let input =
        if input_len == 0 { &[] } else { unsafe { core::slice::from_raw_parts(input, input_len) } };
    let output = unsafe { core::slice::from_raw_parts_mut(output, required) };
    output[..input_len].copy_from_slice(input);
    output[input_len] = 0;
    true
}

/// Builds an atomic public redaction-removal plan.
///
/// # Safety
/// The callbacks and context must remain valid for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_redaction_plan_new(
    has_rects: bool,
    rect_count: usize,
    object_count: usize,
    context: *mut core::ffi::c_void,
    get_rect: RedactionRectCallback,
    get_object: RedactionObjectCallback,
) -> *mut RedactionPlanState {
    let plan = redaction_plan(
        has_rects,
        rect_count,
        object_count,
        |index| {
            let mut rect = RedactionRect { left: 0.0, bottom: 0.0, right: 0.0, top: 0.0 };
            // SAFETY: The caller guarantees synchronous callback validity.
            unsafe {
                get_rect(
                    context,
                    index,
                    &mut rect.left,
                    &mut rect.bottom,
                    &mut rect.right,
                    &mut rect.top,
                )
            }
            .then_some(rect)
        },
        |index| {
            let mut active = false;
            let mut object_type = 0;
            let mut rect = RedactionRect { left: 0.0, bottom: 0.0, right: 0.0, top: 0.0 };
            // SAFETY: The caller guarantees synchronous callback validity.
            unsafe {
                get_object(
                    context,
                    index,
                    &mut active,
                    &mut object_type,
                    &mut rect.left,
                    &mut rect.bottom,
                    &mut rect.right,
                    &mut rect.top,
                )
            }
            .then_some((active, object_type, rect))
        },
    );
    plan.map_or(core::ptr::null_mut(), |plan| Box::into_raw(Box::new(plan)))
}

/// # Safety
/// `state` must be null or returned by the matching constructor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_redaction_plan_free(state: *mut RedactionPlanState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_redaction_plan_status(
    state: *const RedactionPlanState,
    output: *mut i32,
) -> bool {
    let (Some(state), Some(output)) = (unsafe { state.as_ref() }, unsafe { output.as_mut() })
    else {
        return false;
    };
    *output = state.status;
    true
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_redaction_plan_count(
    state: *const RedactionPlanState,
) -> usize {
    unsafe { state.as_ref() }.map_or(0, |state| state.removals.len())
}

/// # Safety
/// State and output must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_redaction_plan_index(
    state: *const RedactionPlanState,
    index: usize,
    output: *mut usize,
) -> bool {
    let (Some(value), Some(output)) =
        (unsafe { state.as_ref() }.and_then(|state| state.removals.get(index)), unsafe {
            output.as_mut()
        })
    else {
        return false;
    };
    *output = *value;
    true
}

/// Plans indexed page-object insertion and content-stream inheritance.
///
/// # Safety
/// The callback, context, and outputs must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_page_object_insert_plan(
    index: usize,
    object_count: usize,
    content_stream: i32,
    context: *mut core::ffi::c_void,
    get_neighbor_stream: PageObjectStreamCallback,
    allowed: *mut bool,
    planned_content_stream: *mut i32,
    mark_dirty: *mut bool,
) -> bool {
    let (Some(allowed), Some(planned_content_stream), Some(mark_dirty)) =
        (unsafe { allowed.as_mut() }, unsafe { planned_content_stream.as_mut() }, unsafe {
            mark_dirty.as_mut()
        })
    else {
        return false;
    };
    let Some(plan) = page_object_insert_plan(index, object_count, content_stream, |neighbor| {
        let mut stream = -1;
        // SAFETY: The caller guarantees synchronous callback validity.
        unsafe { get_neighbor_stream(context, neighbor, &mut stream) }.then_some(stream)
    }) else {
        return false;
    };
    *allowed = plan.allowed;
    *planned_content_stream = plan.content_stream;
    *mark_dirty = plan.mark_dirty;
    true
}

/// Plans page-object lookup and dirty-stream selection before native removal.
///
/// # Safety
/// The callback, context, and outputs must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_page_object_remove_plan(
    object_count: usize,
    target_handle: usize,
    context: *mut core::ffi::c_void,
    describe: PageObjectDescribeCallback,
    found: *mut bool,
    index: *mut usize,
    content_stream: *mut i32,
) -> bool {
    let (Some(found), Some(index), Some(content_stream)) =
        (unsafe { found.as_mut() }, unsafe { index.as_mut() }, unsafe { content_stream.as_mut() })
    else {
        return false;
    };
    let Some(plan) = page_object_remove_plan(object_count, target_handle, |object_index| {
        let mut handle = 0;
        let mut stream = -1;
        // SAFETY: The caller guarantees synchronous callback validity.
        unsafe { describe(context, object_index, &mut handle, &mut stream) }
            .then_some((handle, stream))
    }) else {
        return false;
    };
    *found = plan.found;
    *index = plan.index;
    *content_stream = plan.content_stream;
    true
}

/// Plans active-state mutation and whether it dirties the page object.
///
/// # Safety
/// Outputs must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_page_object_active_update(
    current: bool,
    requested: bool,
    active: *mut bool,
    mark_dirty: *mut bool,
) -> bool {
    let (Some(active), Some(mark_dirty)) =
        (unsafe { active.as_mut() }, unsafe { mark_dirty.as_mut() })
    else {
        return false;
    };
    (*active, *mark_dirty) = page_object_active_update(current, requested);
    true
}

/// Counts active page objects through a synchronous borrowed-state callback.
///
/// # Safety
/// The callback, context, and output must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_page_object_active_count(
    object_count: usize,
    context: *mut core::ffi::c_void,
    get_active: PageObjectActiveCallback,
    output: *mut usize,
) -> bool {
    let Some(output) = (unsafe { output.as_mut() }) else {
        return false;
    };
    let Some(count) = page_object_active_count(object_count, |index| {
        let mut active = false;
        // SAFETY: The caller guarantees synchronous callback validity.
        unsafe { get_active(context, index, &mut active) }.then_some(active)
    }) else {
        return false;
    };
    *output = count;
    true
}

/// Selects the native matrix storage for a public page-object operation.
///
/// # Safety
/// `output` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_page_object_matrix_route(
    object_type: u8,
    output: *mut u8,
) -> bool {
    let (Some(route), Some(output)) =
        (page_object_matrix_route(object_type), unsafe { output.as_mut() })
    else {
        return false;
    };
    *output = route;
    true
}

/// Selects dirty-state behavior after a public matrix mutation.
///
/// # Safety
/// The inputs contain six readable floats and `output` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_page_object_matrix_dirty(
    object_type: u8,
    original: *const f32,
    replacement: *const f32,
    output: *mut bool,
) -> bool {
    if original.is_null() || replacement.is_null() {
        return false;
    }
    let (Some(dirty), Some(output)) = (
        page_object_matrix_dirty(object_type, unsafe { *(original as *const [f32; 6]) }, unsafe {
            *(replacement as *const [f32; 6])
        }),
        unsafe { output.as_mut() },
    ) else {
        return false;
    };
    *output = dirty;
    true
}

/// Computes public rotated bounds in PDF QuadPoints order.
///
/// # Safety
/// Inputs contain six and four readable floats; output contains eight writable
/// floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_page_object_rotated_bounds(
    object_type: u8,
    matrix: *const f32,
    bounds: *const f32,
    output: *mut f32,
) -> bool {
    if matrix.is_null() || bounds.is_null() || output.is_null() {
        return false;
    }
    let Some(quad) =
        page_object_rotated_bounds(object_type, unsafe { *(matrix as *const [f32; 6]) }, unsafe {
            *(bounds as *const [f32; 4])
        })
    else {
        return false;
    };
    unsafe { *(output as *mut [f32; 8]) = quad };
    true
}

struct DocumentPageMutationCallbacks {
    context: *mut core::ffi::c_void,
    describe: DocumentPageMutationDescribeCallback,
    child: DocumentPageMutationChildCallback,
}

fn plan_document_page_mutation(
    handle: usize,
    pages_to_go: &mut i32,
    active_path: &mut std::collections::BTreeSet<usize>,
    depth: usize,
    callbacks: &DocumentPageMutationCallbacks,
) -> Result<Option<Vec<usize>>, ()> {
    const LEVEL_MAX: usize = 1024;
    if handle == 0 || depth >= LEVEL_MAX {
        return Err(());
    }
    let mut child_count = 0;
    if !unsafe { (callbacks.describe)(callbacks.context, handle, &mut child_count) } {
        return Err(());
    }
    for index in 0..child_count {
        let mut child_handle = 0;
        let mut node_type = 0;
        let mut page_count = 0;
        if !unsafe {
            (callbacks.child)(
                callbacks.context,
                handle,
                index,
                &mut child_handle,
                &mut node_type,
                &mut page_count,
            )
        } {
            return Err(());
        }
        if node_type == 2 {
            if *pages_to_go != 0 {
                *pages_to_go = (*pages_to_go).checked_sub(1).ok_or(())?;
                continue;
            }
            return Ok(Some(vec![index]));
        }
        if node_type != 1 {
            return Err(());
        }
        if *pages_to_go >= page_count {
            *pages_to_go = (*pages_to_go).checked_sub(page_count).ok_or(())?;
            continue;
        }
        if child_handle == 0 || !active_path.insert(child_handle) {
            return Ok(None);
        }
        let child_result = plan_document_page_mutation(
            child_handle,
            pages_to_go,
            active_path,
            depth + 1,
            callbacks,
        );
        active_path.remove(&child_handle);
        let Some(mut path) = child_result? else {
            return Ok(None);
        };
        path.insert(0, index);
        return Ok(Some(path));
    }
    Ok(None)
}

/// Plans the child-index path to an insertion/deletion leaf.
///
/// An empty path is a valid "no leaf" result. A null output performs the
/// sizing pass. Depth rejection falls back to the retained C++ traversal.
///
/// # Safety
/// All callbacks and non-null spans must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_document_page_mutation_path(
    root_handle: usize,
    pages_to_go: i32,
    context: *mut core::ffi::c_void,
    describe: Option<DocumentPageMutationDescribeCallback>,
    child: Option<DocumentPageMutationChildCallback>,
    output: *mut usize,
    output_capacity: usize,
    output_len: *mut usize,
) -> bool {
    let (Some(describe), Some(child), Some(output_len)) =
        (describe, child, unsafe { output_len.as_mut() })
    else {
        return false;
    };
    let callbacks = DocumentPageMutationCallbacks { context, describe, child };
    let mut pages_to_go = pages_to_go;
    let mut active_path = std::collections::BTreeSet::from([root_handle]);
    let Ok(path) =
        plan_document_page_mutation(root_handle, &mut pages_to_go, &mut active_path, 0, &callbacks)
    else {
        return false;
    };
    let path = path.unwrap_or_default();
    *output_len = path.len();
    if output_capacity == 0 {
        return output.is_null();
    }
    if output.is_null() || output_capacity < path.len() {
        return false;
    }
    unsafe { core::slice::from_raw_parts_mut(output, path.len()) }.copy_from_slice(&path);
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

    struct PageCountState {
        normalized: [(usize, u8); 4],
        normalized_count: usize,
        counts: [(usize, i32); 4],
        count_count: usize,
    }

    #[derive(Default)]
    struct PageTraversalTestState {
        retained: Vec<usize>,
        cached: Vec<(i32, u32)>,
        selected: usize,
    }

    unsafe extern "C" fn describe_page_node(
        _context: *mut core::ffi::c_void,
        handle: usize,
        count_hint: *mut i32,
        child_count: *mut usize,
    ) -> bool {
        if count_hint.is_null() || child_count.is_null() {
            return false;
        }
        unsafe {
            *count_hint = 0;
            *child_count = match handle {
                1 => 2,
                2 => 1,
                _ => return false,
            };
        }
        true
    }

    unsafe extern "C" fn describe_page_child(
        _context: *mut core::ffi::c_void,
        handle: usize,
        index: usize,
        child_handle: *mut usize,
        node_type: *mut u8,
        has_kids: *mut bool,
    ) -> bool {
        if child_handle.is_null() || node_type.is_null() || has_kids.is_null() {
            return false;
        }
        let value = match (handle, index) {
            (1, 0) => (2, 0, true),
            (1, 1) => (3, 2, false),
            (2, 0) => (4, 0, false),
            _ => return false,
        };
        unsafe {
            *child_handle = value.0;
            *node_type = value.1;
            *has_kids = value.2;
        }
        true
    }

    unsafe extern "C" fn describe_shared_page_child(
        _context: *mut core::ffi::c_void,
        handle: usize,
        index: usize,
        child_handle: *mut usize,
        node_type: *mut u8,
        has_kids: *mut bool,
    ) -> bool {
        if child_handle.is_null() || node_type.is_null() || has_kids.is_null() {
            return false;
        }
        let value = match (handle, index) {
            (1, 0 | 1) => (2, 1, true),
            (2, 0) => (3, 2, false),
            _ => return false,
        };
        unsafe {
            *child_handle = value.0;
            *node_type = value.1;
            *has_kids = value.2;
        }
        true
    }

    unsafe extern "C" fn normalize_page_node(
        context: *mut core::ffi::c_void,
        handle: usize,
        node_type: u8,
    ) -> bool {
        let state = unsafe { &mut *context.cast::<PageCountState>() };
        state.normalized[state.normalized_count] = (handle, node_type);
        state.normalized_count += 1;
        true
    }

    unsafe extern "C" fn set_page_count(
        context: *mut core::ffi::c_void,
        handle: usize,
        count: i32,
    ) -> bool {
        let state = unsafe { &mut *context.cast::<PageCountState>() };
        state.counts[state.count_count] = (handle, count);
        state.count_count += 1;
        true
    }

    unsafe extern "C" fn describe_page_index_node(
        _context: *mut core::ffi::c_void,
        handle: usize,
        has_kids: *mut bool,
        count_hint: *mut i32,
        object_number: *mut u32,
        child_count: *mut usize,
    ) -> bool {
        if has_kids.is_null()
            || count_hint.is_null()
            || object_number.is_null()
            || child_count.is_null()
        {
            return false;
        }
        let (kids, count, object, children) = match handle {
            1 => (true, 2, 1, 2),
            2 => (false, 0, 10, 0),
            3 => (false, 0, 11, 0),
            _ => return false,
        };
        unsafe {
            *has_kids = kids;
            *count_hint = count;
            *object_number = object;
            *child_count = children;
        }
        true
    }

    unsafe extern "C" fn describe_page_index_child(
        _context: *mut core::ffi::c_void,
        handle: usize,
        child_index: usize,
        child_handle: *mut usize,
        reference_object_number: *mut u32,
    ) -> bool {
        if child_handle.is_null() || reference_object_number.is_null() || handle != 1 {
            return false;
        }
        let (child, object) = match child_index {
            0 => (2, 10),
            1 => (3, 11),
            _ => return false,
        };
        unsafe {
            *child_handle = child;
            *reference_object_number = object;
        }
        true
    }

    unsafe extern "C" fn describe_page_mutation_node(
        _context: *mut core::ffi::c_void,
        handle: usize,
        child_count: *mut usize,
    ) -> bool {
        if child_count.is_null() {
            return false;
        }
        let count = match handle {
            1 | 2 => 2,
            6 => 1,
            _ => return false,
        };
        unsafe { *child_count = count };
        true
    }

    unsafe extern "C" fn describe_page_mutation_child(
        _context: *mut core::ffi::c_void,
        handle: usize,
        child_index: usize,
        child_handle: *mut usize,
        node_type: *mut u8,
        page_count: *mut i32,
    ) -> bool {
        if child_handle.is_null() || node_type.is_null() || page_count.is_null() {
            return false;
        }
        let value = match (handle, child_index) {
            (1, 0) => (2, 1, 2),
            (1, 1) => (5, 2, 0),
            (2, 0) => (3, 2, 0),
            (2, 1) => (4, 2, 0),
            (6, 0) => (7, 1, i32::MIN),
            _ => return false,
        };
        unsafe {
            *child_handle = value.0;
            *node_type = value.1;
            *page_count = value.2;
        }
        true
    }

    unsafe extern "C" fn retain_page_traversal_handle(
        context: *mut core::ffi::c_void,
        handle: usize,
    ) -> bool {
        let state = unsafe { &mut *context.cast::<PageTraversalTestState>() };
        state.retained.push(handle);
        true
    }

    unsafe extern "C" fn release_page_traversal_handle(
        context: *mut core::ffi::c_void,
        handle: usize,
    ) -> bool {
        let state = unsafe { &mut *context.cast::<PageTraversalTestState>() };
        let Some(index) = state.retained.iter().position(|value| *value == handle) else {
            return false;
        };
        state.retained.remove(index);
        true
    }

    unsafe extern "C" fn describe_page_traversal_node(
        _context: *mut core::ffi::c_void,
        handle: usize,
        has_kids_array: *mut bool,
        node_type: *mut u8,
        object_number: *mut u32,
        child_count: *mut usize,
    ) -> bool {
        if has_kids_array.is_null()
            || node_type.is_null()
            || object_number.is_null()
            || child_count.is_null()
        {
            return false;
        }
        let (has_kids, kind, object, count) = match handle {
            1 => (true, 1, 1, 2),
            2 => (true, 1, 2, 2),
            3 => (false, 2, 10, 0),
            4 => (false, 2, 11, 0),
            5 => (false, 2, 12, 0),
            _ => return false,
        };
        unsafe {
            *has_kids_array = has_kids;
            *node_type = kind;
            *object_number = object;
            *child_count = count;
        }
        true
    }

    unsafe extern "C" fn describe_page_traversal_child(
        _context: *mut core::ffi::c_void,
        handle: usize,
        child_index: usize,
        child_handle: *mut usize,
        has_kids: *mut bool,
        object_number: *mut u32,
    ) -> bool {
        if child_handle.is_null() || has_kids.is_null() || object_number.is_null() {
            return false;
        }
        let value = match (handle, child_index) {
            (1, 0) => (2, true, 2),
            (1, 1) => (5, false, 12),
            (2, 0) => (3, false, 10),
            (2, 1) => (4, false, 11),
            _ => return false,
        };
        unsafe {
            *child_handle = value.0;
            *has_kids = value.1;
            *object_number = value.2;
        }
        true
    }

    unsafe extern "C" fn cache_page_traversal_result(
        context: *mut core::ffi::c_void,
        page_index: i32,
        object_number: u32,
    ) -> bool {
        let state = unsafe { &mut *context.cast::<PageTraversalTestState>() };
        state.cached.push((page_index, object_number));
        true
    }

    unsafe extern "C" fn select_page_traversal_result(
        context: *mut core::ffi::c_void,
        handle: usize,
    ) -> bool {
        let state = unsafe { &mut *context.cast::<PageTraversalTestState>() };
        state.selected = handle;
        true
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
        assert!(!index.contains_handle(300));
        assert!(index.contains_handle(400));

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
            (b"true\0".as_slice(), true),
            (b"true\0false".as_slice(), true),
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
    fn pdf_reference_state_should_preserve_all_object_numbers() {
        for object_number in [0, 1, i32::MAX as u32, u32::MAX] {
            let mut state = PdfReferenceState { object_number };
            let mut output = 0;
            assert!(unsafe { pdfium_rust_pdf_reference_get(&state, &mut output) });
            assert_eq!(object_number, output);
            assert!(unsafe {
                pdfium_rust_pdf_reference_set(&mut state, object_number.wrapping_add(1))
            });
            assert_eq!(object_number.wrapping_add(1), state.object_number);
        }
    }

    #[test]
    fn pdf_array_state_should_own_ordered_slot_mutations() {
        let mut state = PdfArrayState::default();
        assert!(unsafe { pdfium_rust_pdf_array_append(&mut state, 10) });
        assert!(unsafe { pdfium_rust_pdf_array_append(&mut state, 20) });
        assert!(unsafe { pdfium_rust_pdf_array_insert(&mut state, 1, 15) });
        assert_eq!(vec![10, 15, 20], state.objects);
        assert!(state.objects.contains(&10));

        let mut old_handle = 0;
        assert!(unsafe { pdfium_rust_pdf_array_set(&mut state, 2, 25, &mut old_handle) });
        assert_eq!(20, old_handle);
        assert!(unsafe { pdfium_rust_pdf_array_remove(&mut state, 0, &mut old_handle) });
        assert_eq!(10, old_handle);
        assert_eq!(vec![15, 25], state.objects);

        assert!(!unsafe { pdfium_rust_pdf_array_insert(&mut state, 4, 30) });
        assert!(!unsafe { pdfium_rust_pdf_array_append(&mut state, 0) });
        assert!(unsafe { pdfium_rust_pdf_array_clear(&mut state) });
        assert!(state.objects.is_empty());
    }

    #[test]
    fn pdf_dictionary_state_should_own_sorted_key_mutations() {
        let mut state = PdfDictionaryState::default();
        let mut old_handle = 99;
        assert!(unsafe {
            pdfium_rust_pdf_dictionary_set(&mut state, b"z".as_ptr(), 1, 10, &mut old_handle)
        });
        assert_eq!(0, old_handle);
        assert!(unsafe {
            pdfium_rust_pdf_dictionary_set(&mut state, b"a\0b".as_ptr(), 3, 20, &mut old_handle)
        });
        assert!(unsafe {
            pdfium_rust_pdf_dictionary_set(&mut state, b"a".as_ptr(), 1, 30, &mut old_handle)
        });
        assert_eq!(
            vec![b"a".to_vec(), b"a\0b".to_vec(), b"z".to_vec()],
            state.objects.keys().cloned().collect::<Vec<_>>()
        );
        assert!(state.objects.values().any(|value| *value == 20));

        assert!(unsafe {
            pdfium_rust_pdf_dictionary_set(&mut state, b"a".as_ptr(), 1, 40, &mut old_handle)
        });
        assert_eq!(30, old_handle);
        assert!(unsafe {
            pdfium_rust_pdf_dictionary_remove(&mut state, b"z".as_ptr(), 1, &mut old_handle)
        });
        assert_eq!(10, old_handle);
        assert!(!state.objects.values().any(|value| *value == 10));
        assert!(!unsafe {
            pdfium_rust_pdf_dictionary_remove(&mut state, b"missing".as_ptr(), 7, &mut old_handle)
        });
    }

    #[test]
    fn pdf_string_state_should_preserve_binary_bytes_and_hex_mode() {
        let mut state = PdfStringState { bytes: b"a\0b".to_vec(), output_is_hex: true };
        assert!(unsafe { pdfium_rust_pdf_string_equals(&state, b"a\0b".as_ptr(), 3) });
        assert!(!unsafe { pdfium_rust_pdf_string_equals(&state, b"a".as_ptr(), 1) });
        assert!(unsafe { pdfium_rust_pdf_string_set(&mut state, b"next".as_ptr(), 4) });
        assert_eq!(b"next", state.bytes.as_slice());
        assert!(state.output_is_hex);
    }

    #[test]
    fn pdf_stream_data_state_should_own_exact_binary_bytes() {
        let input = [0, 1, 0xfe, 0xff, 0];
        let state = PdfStreamDataState { bytes: input.to_vec() };
        let mut data = core::ptr::null();
        let mut len = 0;
        assert!(unsafe { pdfium_rust_pdf_stream_data_span(&state, &mut data, &mut len) });
        assert_eq!(input.len(), len);
        assert_eq!(input, unsafe { core::slice::from_raw_parts(data, len) });
    }

    #[test]
    fn document_page_index_should_own_resize_insert_remove_and_lookup() {
        let mut state = DocumentPageIndexState::default();
        assert!(unsafe { pdfium_rust_document_page_index_resize(&mut state, 3) });
        assert!(unsafe { pdfium_rust_document_page_index_set(&mut state, 1, 20) });
        assert!(unsafe { pdfium_rust_document_page_index_insert(&mut state, 1, 10) });
        assert_eq!(vec![0, 10, 20, 0], state.object_numbers);
        assert!(unsafe { pdfium_rust_document_page_index_contains(&state, 20) });
        assert!(unsafe { pdfium_rust_document_page_index_remove(&mut state, 2) });
        assert_eq!(vec![0, 10, 0], state.object_numbers);
        assert!(!unsafe { pdfium_rust_document_page_index_contains(&state, 20) });
        assert!(!unsafe { pdfium_rust_document_page_index_set(&mut state, 4, 99) });
    }

    #[test]
    fn document_move_page_plan_should_validate_and_sort_deletions() {
        let indices = [4, 1, 3];
        let mut deletion_order = [0; 3];
        assert!(unsafe {
            pdfium_rust_document_move_page_plan(
                indices.as_ptr(),
                indices.len(),
                7,
                2,
                deletion_order.as_mut_ptr(),
            )
        });
        assert_eq!([4, 3, 1], deletion_order);

        for invalid in [[1, 1, 3], [-1, 2, 3], [1, 2, 7]] {
            assert!(!unsafe {
                pdfium_rust_document_move_page_plan(
                    invalid.as_ptr(),
                    invalid.len(),
                    7,
                    2,
                    deletion_order.as_mut_ptr(),
                )
            });
        }
        assert!(!unsafe {
            pdfium_rust_document_move_page_plan(
                indices.as_ptr(),
                indices.len(),
                7,
                5,
                deletion_order.as_mut_ptr(),
            )
        });
    }

    #[test]
    fn document_page_count_should_own_traversal_normalization_and_counts() {
        let mut state = PageCountState {
            normalized: [(0, 0); 4],
            normalized_count: 0,
            counts: [(0, 0); 4],
            count_count: 0,
        };
        let mut output = 0;
        assert!(unsafe {
            pdfium_rust_document_count_pages(
                1,
                (&mut state as *mut PageCountState).cast(),
                Some(describe_page_node),
                Some(describe_page_child),
                Some(normalize_page_node),
                Some(set_page_count),
                &mut output,
            )
        });
        assert_eq!(2, output);
        assert_eq!([(2, 1), (4, 2)], state.normalized[..state.normalized_count]);
        assert_eq!([(2, 1), (1, 2)], state.counts[..state.count_count]);
    }

    #[test]
    fn document_page_count_should_only_guard_the_active_recursion_path() {
        let mut state = PageCountState {
            normalized: [(0, 0); 4],
            normalized_count: 0,
            counts: [(0, 0); 4],
            count_count: 0,
        };
        let mut output = 0;
        assert!(unsafe {
            pdfium_rust_document_count_pages(
                1,
                (&mut state as *mut PageCountState).cast(),
                Some(describe_page_node),
                Some(describe_shared_page_child),
                Some(normalize_page_node),
                Some(set_page_count),
                &mut output,
            )
        });
        assert_eq!(2, output);
        assert_eq!(0, state.normalized_count);
        assert_eq!([(2, 1), (2, 1), (1, 2)], state.counts[..state.count_count]);
    }

    #[test]
    fn document_page_index_should_find_direct_references_and_misses() {
        for (target, expected) in [(11, 1), (12, -1)] {
            let mut output = -2;
            assert!(unsafe {
                pdfium_rust_document_find_page_index(
                    1,
                    target,
                    0,
                    core::ptr::null_mut(),
                    Some(describe_page_index_node),
                    Some(describe_page_index_child),
                    &mut output,
                )
            });
            assert_eq!(expected, output);
        }
    }

    #[test]
    fn sdk_page_range_should_preserve_order_duplicates_and_invalid_results() {
        for (input, page_count, expected) in [
            ("1-4,3-6", 10, vec![0, 1, 2, 3, 2, 3, 4, 5]),
            ("5  0, 1-2 ", 100, vec![49, 0, 1]),
            ("1-2,,,,3-4", 10, vec![]),
            ("clams", 10, vec![]),
        ] {
            let mut len = 0;
            assert!(unsafe {
                pdfium_rust_sdk_parse_page_range(
                    input.as_ptr(),
                    input.len(),
                    page_count,
                    core::ptr::null_mut(),
                    0,
                    &mut len,
                )
            });
            let mut output = vec![0; len];
            if len != 0 {
                assert!(unsafe {
                    pdfium_rust_sdk_parse_page_range(
                        input.as_ptr(),
                        input.len(),
                        page_count,
                        output.as_mut_ptr(),
                        output.len(),
                        &mut len,
                    )
                });
            }
            assert_eq!(expected, output);
        }
        let overflow = "999999999999999999999";
        let mut len = 0;
        assert!(!unsafe {
            pdfium_rust_sdk_parse_page_range(
                overflow.as_ptr(),
                overflow.len(),
                u32::MAX,
                core::ptr::null_mut(),
                0,
                &mut len,
            )
        });
    }

    #[test]
    fn sdk_nul_termination_should_copy_only_complete_results() {
        let input = b"a\0b";
        let mut short = [0x42; 3];
        let mut required = 0;
        assert!(unsafe {
            pdfium_rust_sdk_nul_terminate(
                input.as_ptr(),
                input.len(),
                short.as_mut_ptr(),
                short.len(),
                &mut required,
            )
        });
        assert_eq!(4, required);
        assert_eq!([0x42; 3], short);

        let mut complete = [0x42; 4];
        assert!(unsafe {
            pdfium_rust_sdk_nul_terminate(
                input.as_ptr(),
                input.len(),
                complete.as_mut_ptr(),
                complete.len(),
                &mut required,
            )
        });
        assert_eq!([b'a', 0, b'b', 0], complete);
    }

    #[test]
    fn document_page_mutation_should_plan_nested_and_root_leaf_paths() {
        for (page_index, expected) in [(1, vec![0, 1]), (2, vec![1])] {
            let mut len = 0;
            assert!(unsafe {
                pdfium_rust_document_page_mutation_path(
                    1,
                    page_index,
                    core::ptr::null_mut(),
                    Some(describe_page_mutation_node),
                    Some(describe_page_mutation_child),
                    core::ptr::null_mut(),
                    0,
                    &mut len,
                )
            });
            let mut output = vec![0; len];
            assert!(unsafe {
                pdfium_rust_document_page_mutation_path(
                    1,
                    page_index,
                    core::ptr::null_mut(),
                    Some(describe_page_mutation_node),
                    Some(describe_page_mutation_child),
                    output.as_mut_ptr(),
                    output.len(),
                    &mut len,
                )
            });
            assert_eq!(expected, output);
        }
        let mut len = 0;
        assert!(!unsafe {
            pdfium_rust_document_page_mutation_path(
                6,
                1,
                core::ptr::null_mut(),
                Some(describe_page_mutation_node),
                Some(describe_page_mutation_child),
                core::ptr::null_mut(),
                0,
                &mut len,
            )
        });
    }

    #[test]
    fn document_page_traversal_should_preserve_incremental_stack_and_lifetimes() {
        let mut traversal = DocumentPageTraversalState::default();
        let mut state = PageTraversalTestState::default();
        let context = (&mut state as *mut PageTraversalTestState).cast();
        assert!(unsafe {
            pdfium_rust_document_page_traversal_reset(
                &mut traversal,
                1,
                context,
                Some(retain_page_traversal_handle),
                Some(release_page_traversal_handle),
            )
        });
        for (page_index, expected_handle) in [(0, 3), (1, 4), (2, 5)] {
            state.selected = 0;
            let mut found = false;
            assert!(unsafe {
                pdfium_rust_document_page_traversal_run(
                    &mut traversal,
                    page_index,
                    context,
                    Some(describe_page_traversal_node),
                    Some(describe_page_traversal_child),
                    Some(cache_page_traversal_result),
                    Some(select_page_traversal_result),
                    Some(retain_page_traversal_handle),
                    Some(release_page_traversal_handle),
                    &mut found,
                )
            });
            assert!(found);
            assert_eq!(expected_handle, state.selected);
        }
        assert_eq!(vec![(0, 10), (1, 11), (2, 12)], state.cached);
        assert!(unsafe {
            pdfium_rust_document_page_traversal_clear(
                &mut traversal,
                context,
                Some(release_page_traversal_handle),
            )
        });
        assert!(state.retained.is_empty());
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

    #[test]
    fn redaction_plan_should_validate_atomically_and_select_removals() {
        let rects = [RedactionRect { left: 0.0, bottom: 0.0, right: 20.0, top: 20.0 }];
        let objects = [
            (true, 1, RedactionRect { left: 1.0, bottom: 1.0, right: 5.0, top: 5.0 }),
            (false, 4, RedactionRect { left: 1.0, bottom: 1.0, right: 5.0, top: 5.0 }),
            (true, 3, RedactionRect { left: 30.0, bottom: 30.0, right: 40.0, top: 40.0 }),
        ];
        let plan = redaction_plan(
            true,
            rects.len(),
            objects.len(),
            |index| rects.get(index).copied(),
            |index| objects.get(index).copied(),
        )
        .expect("callbacks are complete");
        assert_eq!(0, plan.status);
        assert_eq!(vec![0], plan.removals);

        let partial = redaction_plan(
            true,
            1,
            1,
            |_| Some(RedactionRect { left: 0.0, bottom: 0.0, right: 2.0, top: 2.0 }),
            |_| Some((true, 1, RedactionRect { left: 1.0, bottom: 1.0, right: 3.0, top: 3.0 })),
        )
        .expect("callbacks are complete");
        assert_eq!(5, partial.status);
        assert!(partial.removals.is_empty());
    }

    #[test]
    fn redaction_plan_should_reject_rects_and_unsupported_objects() {
        let invalid = redaction_plan(false, 1, 0, |_| None, |_| None).expect("no callback needed");
        assert_eq!(2, invalid.status);

        let unsupported = redaction_plan(
            true,
            1,
            1,
            |_| Some(RedactionRect { left: 0.0, bottom: 0.0, right: 10.0, top: 10.0 }),
            |_| Some((true, 4, RedactionRect { left: 1.0, bottom: 1.0, right: 2.0, top: 2.0 })),
        )
        .expect("callbacks are complete");
        assert_eq!(4, unsupported.status);
        assert!(unsupported.removals.is_empty());
    }

    #[test]
    fn page_object_insert_plan_should_validate_and_inherit_streams() {
        assert_eq!(
            Some(PageObjectInsertPlan { allowed: false, content_stream: -1, mark_dirty: false }),
            page_object_insert_plan(3, 2, -1, |_| None)
        );
        assert_eq!(
            Some(PageObjectInsertPlan { allowed: true, content_stream: 4, mark_dirty: true }),
            page_object_insert_plan(1, 2, -1, |index| (index == 1).then_some(4))
        );
        assert_eq!(
            Some(PageObjectInsertPlan { allowed: true, content_stream: -1, mark_dirty: false }),
            page_object_insert_plan(2, 2, -1, |_| None)
        );
        assert_eq!(
            Some(PageObjectInsertPlan { allowed: true, content_stream: 7, mark_dirty: false }),
            page_object_insert_plan(0, 2, 7, |_| None)
        );
    }

    #[test]
    fn page_object_remove_plan_should_find_handle_and_stream() {
        let objects = [(11, 0), (22, -1), (33, 4)];
        assert_eq!(
            Some(PageObjectRemovePlan { found: true, index: 2, content_stream: 4 }),
            page_object_remove_plan(objects.len(), 33, |index| objects.get(index).copied())
        );
        assert_eq!(
            Some(PageObjectRemovePlan { found: true, index: 1, content_stream: -1 }),
            page_object_remove_plan(objects.len(), 22, |index| objects.get(index).copied())
        );
        assert_eq!(
            Some(PageObjectRemovePlan { found: false, index: 0, content_stream: -1 }),
            page_object_remove_plan(objects.len(), 44, |index| objects.get(index).copied())
        );
        assert_eq!(None, page_object_remove_plan(1, 11, |_| None));
    }

    #[test]
    fn page_object_active_state_should_dirty_only_changes_and_count_exactly() {
        assert_eq!((false, false), page_object_active_update(false, false));
        assert_eq!((true, true), page_object_active_update(false, true));
        assert_eq!((false, true), page_object_active_update(true, false));
        assert_eq!(Some(2), page_object_active_count(4, |index| Some(matches!(index, 0 | 3))));
        assert_eq!(None, page_object_active_count(2, |index| (index == 0).then_some(true)));
    }

    #[test]
    fn page_object_matrix_policy_should_route_types_and_dirty_images_exactly() {
        let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let translated = [1.0, 0.0, 0.0, 1.0, 4.0, 5.0];
        assert_eq!(Some(1), page_object_matrix_route(1));
        assert_eq!(Some(5), page_object_matrix_route(5));
        assert_eq!(None, page_object_matrix_route(4));
        assert_eq!(Some(true), page_object_matrix_dirty(1, identity, identity));
        assert_eq!(Some(false), page_object_matrix_dirty(3, identity, identity));
        assert_eq!(Some(true), page_object_matrix_dirty(3, identity, translated));
        assert_eq!(None, page_object_matrix_dirty(4, identity, translated));
    }

    #[test]
    fn page_object_rotated_bounds_should_preserve_quadpoint_order() {
        let quad = page_object_rotated_bounds(
            1,
            [2.0, 1.0, -1.0, 3.0, 5.0, 7.0],
            [10.0, 20.0, 30.0, 40.0],
        )
        .expect("text objects have rotated bounds");
        assert_eq!([5.0, 77.0, 45.0, 97.0, 25.0, 157.0, -15.0, 137.0], quad);
        assert!(page_object_rotated_bounds(3, [1.0; 6], [0.0; 4]).is_some());
        assert!(page_object_rotated_bounds(2, [1.0; 6], [0.0; 4]).is_none());
    }
}
