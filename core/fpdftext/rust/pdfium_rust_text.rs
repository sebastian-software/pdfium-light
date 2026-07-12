// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![allow(unsafe_code)]

type IndexMapCallback = unsafe extern "C" fn(*mut core::ffi::c_void, i32, *mut i32) -> bool;
type IsAlphanumericCallback = unsafe extern "C" fn(*mut core::ffi::c_void, u32) -> bool;

#[derive(Clone)]
pub struct TextFindState {
    page_text: Vec<u32>,
    words: Vec<Vec<u32>>,
    match_whole_word: bool,
    consecutive: bool,
    next_start: Option<usize>,
    previous_start: Option<usize>,
    result_start: usize,
    result_end: usize,
}

struct TextLink {
    start: usize,
    count: usize,
    url: Vec<u32>,
}

#[derive(Default)]
pub struct TextLinkExtractState {
    links: Vec<TextLink>,
}

fn is_ignored_space_character(character: u32) -> bool {
    !(character < 255
        || (0x0600..=0x06ff).contains(&character)
        || (0xfe70..=0xfeff).contains(&character)
        || (0xfb50..=0xfdff).contains(&character)
        || (0x0400..=0x04ff).contains(&character)
        || (0x0500..=0x052f).contains(&character)
        || (0xa640..=0xa69f).contains(&character)
        || (0x2de0..=0x2dff).contains(&character)
        || character == 8467
        || (0x2000..=0x206f).contains(&character))
}

fn is_decimal_digit(character: u32) -> bool {
    (b'0' as u32..=b'9' as u32).contains(&character)
}

fn is_match_whole_word(page_text: &[u32], start: usize, end: usize) -> bool {
    if start > end {
        return false;
    }
    let character_count = end - start + 1;
    if character_count == 1 && page_text[start] > 255 {
        return true;
    }
    let left = start.checked_sub(1).map_or(0, |index| page_text[index]);
    let right = page_text.get(start + character_count).copied().unwrap_or(0);
    if (left > b'A' as u32 && left < b'a' as u32)
        || (left > b'a' as u32 && left < b'z' as u32)
        || (left > 0xfb00 && left < 0xfb06)
        || is_decimal_digit(left)
        || (right > b'A' as u32 && right < b'a' as u32)
        || (right > b'a' as u32 && right < b'z' as u32)
        || (right > 0xfb00 && right < 0xfb06)
        || is_decimal_digit(right)
    {
        return false;
    }
    if !((left < b'A' as u32 || left > b'Z' as u32)
        && (left < b'a' as u32 || left > b'z' as u32)
        && (right < b'A' as u32 || right > b'Z' as u32)
        && (right < b'a' as u32 || right > b'z' as u32))
    {
        return false;
    }
    if is_decimal_digit(left) && is_decimal_digit(page_text[start]) {
        return false;
    }
    if is_decimal_digit(right) && is_decimal_digit(page_text[end]) {
        return false;
    }
    true
}

fn find_subslice(haystack: &[u32], needle: &[u32], start: usize) -> Option<usize> {
    if needle.is_empty() || start > haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|position| start + position)
}

fn extract_find_words(query: &[u32]) -> Vec<Vec<u32>> {
    if query.iter().all(|character| *character == b' ' as u32) {
        return vec![query.to_vec()];
    }
    let mut raw_words = Vec::new();
    let mut start = 0;
    loop {
        let end = query[start..]
            .iter()
            .position(|character| *character == b' ' as u32)
            .map(|offset| start + offset)
            .unwrap_or(query.len());
        raw_words.push(query[start..end].to_vec());
        if end == query.len() {
            break;
        }
        start = end + 1;
        while start < query.len() && query[start] == b' ' as u32 {
            start += 1;
        }
        if start == query.len() {
            raw_words.push(Vec::new());
            break;
        }
    }

    let mut words = Vec::new();
    for mut word in raw_words {
        if word.is_empty() {
            words.push(word);
            continue;
        }
        let mut position = 0;
        while position < word.len() {
            let character = word[position];
            if is_ignored_space_character(character) {
                if position > 0 && character == 0x2019 {
                    position += 1;
                    continue;
                }
                if position > 0 {
                    words.push(word[..position].to_vec());
                }
                words.push(vec![character]);
                if position == word.len() - 1 {
                    word.clear();
                    break;
                }
                word = word[position + 1..].to_vec();
                position = 0;
                continue;
            }
            position += 1;
        }
        if !word.is_empty() {
            words.push(word);
        }
    }
    words
}

impl TextFindState {
    fn new(
        page_text: Vec<u32>,
        words: Vec<Vec<u32>>,
        match_whole_word: bool,
        consecutive: bool,
        start: Option<usize>,
    ) -> Self {
        let (next_start, previous_start) = if page_text.is_empty() {
            (None, None)
        } else {
            (start, Some(start.unwrap_or(page_text.len() - 1)))
        };
        Self {
            page_text,
            words,
            match_whole_word,
            consecutive,
            next_start,
            previous_start,
            result_start: 0,
            result_end: usize::MAX,
        }
    }

    fn find_first(&self) -> bool {
        self.page_text.is_empty() || !self.words.is_empty()
    }

    fn update_cursors(&mut self) {
        if self.consecutive {
            self.next_start = Some(self.result_start.wrapping_add(1));
            self.previous_start = Some(self.result_end.wrapping_sub(1));
        } else {
            self.next_start = Some(self.result_end.wrapping_add(1));
            self.previous_start = Some(self.result_start.wrapping_sub(1));
        }
    }

    fn find_next(&mut self) -> bool {
        let Some(mut start_position) = self.next_start else {
            return false;
        };
        if self.page_text.is_empty() || start_position >= self.page_text.len() {
            return false;
        }
        let mut result_position = 0;
        let mut space_start = false;
        let mut word_index: isize = 0;
        while word_index < self.words.len() as isize {
            let word = &self.words[word_index as usize];
            if word.is_empty() {
                if word_index as usize == self.words.len() - 1 {
                    if start_position >= self.page_text.len() {
                        return false;
                    }
                    let character = self.page_text[start_position];
                    if matches!(character, 10 | 13 | 32 | 160) {
                        result_position = start_position + 1;
                        break;
                    }
                    word_index = -1;
                } else if word_index == 0 {
                    space_start = true;
                }
                word_index += 1;
                continue;
            }
            let Some(found) = find_subslice(&self.page_text, word, start_position) else {
                return false;
            };
            result_position = found;
            let end_index = found + word.len() - 1;
            if word_index == 0 {
                self.result_start = found;
            }
            let mut matches = true;
            if word_index != 0 && !space_start {
                let previous_end = start_position;
                let current_character = word[0];
                let previous_word = &self.words[word_index as usize - 1];
                let previous_character = *previous_word.last().unwrap_or(&0);
                if start_position == found
                    && !(is_ignored_space_character(previous_character)
                        || is_ignored_space_character(current_character))
                {
                    matches = false;
                }
                if self.page_text[previous_end..found]
                    .iter()
                    .any(|character| !matches!(*character, 10 | 13 | 32 | 160))
                {
                    matches = false;
                }
            } else if space_start && found > 0 {
                let character = self.page_text[found - 1];
                if !matches!(character, 10 | 13 | 32 | 160) {
                    matches = false;
                    self.result_start = found;
                } else {
                    self.result_start = found - 1;
                }
            }
            if self.match_whole_word && matches {
                matches = is_match_whole_word(&self.page_text, found, end_index);
            }
            if matches {
                start_position = end_index + 1;
                word_index += 1;
            } else {
                word_index = 0;
                let index = usize::from(space_start);
                start_position = self.result_start + self.words[index].len();
            }
        }
        let last_length = self.words.last().map_or(0, Vec::len);
        self.result_end = result_position.wrapping_add(last_length).wrapping_sub(1);
        self.update_cursors();
        true
    }

    fn map_index(
        context: *mut core::ffi::c_void,
        callback: IndexMapCallback,
        index: i32,
    ) -> Option<i32> {
        let mut output = 0;
        unsafe { callback(context, index, &mut output) }.then_some(output)
    }

    fn find_previous(
        &mut self,
        context: *mut core::ffi::c_void,
        text_to_character: IndexMapCallback,
        character_to_text: IndexMapCallback,
    ) -> Option<bool> {
        let Some(previous_start) = self.previous_start else {
            return Some(false);
        };
        if self.page_text.is_empty() {
            return Some(false);
        }
        let mut engine = Self::new(
            self.page_text.clone(),
            self.words.clone(),
            self.match_whole_word,
            self.consecutive,
            Some(0),
        );
        if !engine.find_first() {
            return Some(false);
        }
        let mut order = -1;
        let mut matched = 0;
        while engine.find_next() {
            let start = Self::map_index(context, text_to_character, engine.result_start as i32)?;
            let end = Self::map_index(context, text_to_character, engine.result_end as i32)?;
            let count = end.checked_sub(start)?.checked_add(1)?;
            let limit = start.checked_add(count)?;
            if limit < 0 || limit as usize > previous_start.wrapping_add(1) {
                break;
            }
            order = start;
            matched = count;
        }
        if order == -1 {
            return Some(false);
        }
        self.result_start = Self::map_index(context, character_to_text, order)? as usize;
        self.result_end = Self::map_index(
            context,
            character_to_text,
            order.checked_add(matched)?.checked_sub(1)?,
        )? as usize;
        self.update_cursors();
        Some(true)
    }
}

fn find_web_link_ending(text: &[u32], start: usize, mut end: usize) -> usize {
    if text[start..].contains(&(b'/' as u32)) {
        return end;
    }
    if text[start] == b'[' as u32 {
        if let Some(relative_end) = text[start + 1..].iter().position(|value| *value == b']' as u32)
        {
            end = start + 1 + relative_end;
            if end > start + 1 {
                let mut offset = end + 1;
                if offset < text.len() && text[offset] == b':' as u32 {
                    offset += 1;
                    while offset < text.len() && is_decimal_digit(text[offset]) {
                        offset += 1;
                    }
                    if offset > end + 2 {
                        end = offset - 1;
                    }
                }
            }
        }
        return end;
    }
    while end > start && text[end] < 0x80 {
        let character = text[end];
        if is_decimal_digit(character)
            || (b'a' as u32..=b'z' as u32).contains(&character)
            || character == b'.' as u32
        {
            break;
        }
        end -= 1;
    }
    end
}

fn trim_backwards_to_character(text: &[u32], character: u32, start: usize, end: &mut usize) {
    if let Some(position) = (start..=*end).rev().find(|position| text[*position] == character) {
        *end = position.wrapping_sub(1);
    }
}

fn trim_external_brackets(text: &[u32], start: usize, mut end: usize) -> usize {
    for &character in &text[..start] {
        let closing = match character {
            value if value == b'(' as u32 => b')' as u32,
            value if value == b'[' as u32 => b']' as u32,
            value if value == b'{' as u32 => b'}' as u32,
            value if value == b'<' as u32 => b'>' as u32,
            value if value == b'"' as u32 => b'"' as u32,
            value if value == b'\'' as u32 => b'\'' as u32,
            _ => continue,
        };
        trim_backwards_to_character(text, closing, start, &mut end);
    }
    end
}

fn find_sequence(text: &[u32], needle: &[u32], start: usize) -> Option<usize> {
    text.get(start..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|position| start + position)
}

fn check_web_link(text: &[u32]) -> Option<TextLink> {
    let lower: Vec<u32> = text
        .iter()
        .map(|character| {
            if (b'A' as u32..=b'Z' as u32).contains(character) {
                character + u32::from(b'a' - b'A')
            } else {
                *character
            }
        })
        .collect();
    if let Some(start) =
        find_sequence(&lower, &[b'h' as u32, b't' as u32, b't' as u32, b'p' as u32], 0)
    {
        let mut offset = start + 4;
        if lower.len() > offset + 4 {
            if lower[offset] == b's' as u32 {
                offset += 1;
            }
            if lower[offset..].starts_with(&[b':' as u32, b'/' as u32, b'/' as u32]) {
                offset += 3;
                let end = find_web_link_ending(
                    &lower,
                    offset,
                    trim_external_brackets(&lower, start, lower.len() - 1),
                );
                if end > offset {
                    return Some(TextLink {
                        start,
                        count: end - start + 1,
                        url: text[start..=end].to_vec(),
                    });
                }
            }
        }
    }
    let www = [b'w' as u32, b'w' as u32, b'w' as u32, b'.' as u32];
    if let Some(start) = find_sequence(&lower, &www, 0) {
        let offset = start + www.len();
        if lower.len() > offset {
            let end = find_web_link_ending(
                &lower,
                start,
                trim_external_brackets(&lower, start, lower.len() - 1),
            );
            if end > offset {
                let mut url: Vec<u32> = "http://".chars().map(u32::from).collect();
                url.extend_from_slice(&text[start..=end]);
                return Some(TextLink { start, count: end - start + 1, url });
            }
        }
    }
    None
}

fn check_mail_link(
    text: &mut Vec<u32>,
    context: *mut core::ffi::c_void,
    is_alphanumeric: IsAlphanumericCallback,
) -> bool {
    let Some(mut at_position) = text.iter().position(|character| *character == b'@' as u32) else {
        return false;
    };
    if at_position == 0 || at_position == text.len() - 1 {
        return false;
    }
    let mut period_position = at_position;
    for index in (1..=at_position).rev() {
        let character = text[index - 1];
        if character == b'_' as u32
            || character == b'-' as u32
            || unsafe { is_alphanumeric(context, character) }
        {
            continue;
        }
        if character != b'.' as u32 || index == period_position || index == 1 {
            if index == at_position {
                return false;
            }
            let removed = if index == period_position { index + 1 } else { index };
            text.drain(..removed);
            break;
        }
        period_position = index - 1;
    }

    let Some(updated_at) = text.iter().position(|character| *character == b'@' as u32) else {
        return false;
    };
    at_position = updated_at;
    while text.last() == Some(&(b'.' as u32)) {
        text.pop();
    }
    let Some(first_period) = text[at_position + 1..]
        .iter()
        .position(|character| *character == b'.' as u32)
        .map(|position| at_position + 1 + position)
    else {
        return false;
    };
    if first_period == at_position + 1 {
        return false;
    }

    period_position = 0;
    let mut index = at_position + 1;
    while index < text.len() {
        let character = text[index];
        if character == b'-' as u32 || unsafe { is_alphanumeric(context, character) } {
            index += 1;
            continue;
        }
        if character != b'.' as u32 || index == period_position + 1 {
            let host_end =
                if index == period_position + 1 { index.wrapping_sub(2) } else { index - 1 };
            if period_position > 0 && host_end.wrapping_sub(at_position) >= 3 {
                text.truncate(host_end + 1);
                break;
            }
            return false;
        }
        period_position = index;
        index += 1;
    }

    let mailto: Vec<u32> = "mailto:".chars().map(u32::from).collect();
    if find_sequence(text, &mailto, 0).is_none() {
        let mut result = mailto;
        result.append(text);
        *text = result;
    }
    true
}

impl TextLinkExtractState {
    fn extract(
        &mut self,
        page_text: &[u32],
        characters: &[u32],
        flags: &[u8],
        context: *mut core::ffi::c_void,
        is_alphanumeric: IsAlphanumericCallback,
    ) -> bool {
        if characters.len() != flags.len() || page_text.len() < characters.len() {
            return false;
        }
        self.links.clear();
        let mut start = 0;
        let mut position = 0;
        let mut after_hyphen = false;
        let mut line_break = false;
        while position < characters.len() {
            let character = characters[position];
            let generated = flags[position] & 1 != 0;
            let hyphen = flags[position] & 2 != 0;
            if !generated && character != b' ' as u32 && position != characters.len() - 1 {
                after_hyphen = hyphen || character == b'-' as u32;
                position += 1;
                continue;
            }
            let mut count = position - start;
            if position == characters.len() - 1 {
                count += 1;
            } else if after_hyphen && matches!(character, 10 | 13) {
                line_break = true;
                position += 1;
                continue;
            }
            let Some(candidate_span) = page_text.get(start..start + count) else {
                return false;
            };
            let mut candidate: Vec<u32> = candidate_span
                .iter()
                .filter(|character| !line_break || !matches!(**character, 10 | 13))
                .map(|character| if *character == 0xfffe { b'-' as u32 } else { *character })
                .collect();
            line_break = false;
            if candidate.len() > 5 {
                while matches!(candidate.last(), Some(value) if matches!(*value, 41 | 44 | 46 | 62))
                {
                    candidate.pop();
                    count -= 1;
                }
                if count > 5 {
                    if let Some(mut link) = check_web_link(&candidate) {
                        link.start += start;
                        self.links.push(link);
                    } else if check_mail_link(&mut candidate, context, is_alphanumeric) {
                        self.links.push(TextLink { start, count, url: candidate });
                    }
                }
            }
            position += 1;
            start = position;
        }
        true
    }
}

fn read_u32_span(data: *const u32, len: usize) -> Option<Vec<u32>> {
    if len == 0 {
        return Some(Vec::new());
    }
    (!data.is_null()).then(|| unsafe { core::slice::from_raw_parts(data, len).to_vec() })
}

/// # Safety
/// Both code-point spans must remain readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_find_new(
    page_text: *const u32,
    page_text_len: usize,
    query: *const u32,
    query_len: usize,
    match_whole_word: bool,
    consecutive: bool,
    has_start: bool,
    start: usize,
) -> *mut TextFindState {
    let (Some(page_text), Some(query)) =
        (read_u32_span(page_text, page_text_len), read_u32_span(query, query_len))
    else {
        return core::ptr::null_mut();
    };
    Box::into_raw(Box::new(TextFindState::new(
        page_text,
        extract_find_words(&query),
        match_whole_word,
        consecutive,
        has_start.then_some(start),
    )))
}

/// # Safety
/// `state` must be null or returned by the matching constructor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_find_free(state: *mut TextFindState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_find_first(state: *const TextFindState) -> bool {
    unsafe { state.as_ref() }.is_some_and(TextFindState::find_first)
}

/// # Safety
/// `state` and `matched` must remain writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_find_next(
    state: *mut TextFindState,
    matched: *mut bool,
) -> bool {
    let (Some(state), Some(matched)) = (unsafe { state.as_mut() }, unsafe { matched.as_mut() })
    else {
        return false;
    };
    *matched = state.find_next();
    true
}

/// # Safety
/// State, callbacks, and `matched` must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_find_previous(
    state: *mut TextFindState,
    context: *mut core::ffi::c_void,
    text_to_character: Option<IndexMapCallback>,
    character_to_text: Option<IndexMapCallback>,
    matched: *mut bool,
) -> bool {
    let (Some(state), Some(text_to_character), Some(character_to_text), Some(matched)) =
        (unsafe { state.as_mut() }, text_to_character, character_to_text, unsafe {
            matched.as_mut()
        })
    else {
        return false;
    };
    let Some(result) = state.find_previous(context, text_to_character, character_to_text) else {
        return false;
    };
    *matched = result;
    true
}

/// # Safety
/// State and outputs must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_find_result(
    state: *const TextFindState,
    start: *mut usize,
    end: *mut usize,
) -> bool {
    let (Some(state), Some(start), Some(end)) =
        (unsafe { state.as_ref() }, unsafe { start.as_mut() }, unsafe { end.as_mut() })
    else {
        return false;
    };
    *start = state.result_start;
    *end = state.result_end;
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_text_link_extract_new() -> *mut TextLinkExtractState {
    Box::into_raw(Box::new(TextLinkExtractState::default()))
}

/// # Safety
/// `state` must be null or returned by the matching constructor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_link_extract_free(state: *mut TextLinkExtractState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// Replaces the extracted link list from one page snapshot.
///
/// # Safety
/// State, callbacks, and all equally indexed character spans remain valid for
/// this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_link_extract_run(
    state: *mut TextLinkExtractState,
    page_text: *const u32,
    page_text_len: usize,
    characters: *const u32,
    flags: *const u8,
    character_count: usize,
    context: *mut core::ffi::c_void,
    is_alphanumeric: Option<IsAlphanumericCallback>,
) -> bool {
    let (Some(state), Some(page_text), Some(characters), Some(flags), Some(is_alphanumeric)) = (
        unsafe { state.as_mut() },
        read_u32_span(page_text, page_text_len),
        read_u32_span(characters, character_count),
        if character_count == 0 {
            Some(Vec::new())
        } else if flags.is_null() {
            None
        } else {
            Some(unsafe { core::slice::from_raw_parts(flags, character_count).to_vec() })
        },
        is_alphanumeric,
    ) else {
        return false;
    };
    state.extract(&page_text, &characters, &flags, context, is_alphanumeric)
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_link_extract_count(
    state: *const TextLinkExtractState,
) -> usize {
    unsafe { state.as_ref() }.map_or(0, |state| state.links.len())
}

/// # Safety
/// State and outputs must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_link_extract_range(
    state: *const TextLinkExtractState,
    index: usize,
    start: *mut usize,
    count: *mut usize,
) -> bool {
    let (Some(link), Some(start), Some(count)) = (
        unsafe { state.as_ref() }.and_then(|state| state.links.get(index)),
        unsafe { start.as_mut() },
        unsafe { count.as_mut() },
    ) else {
        return false;
    };
    *start = link.start;
    *count = link.count;
    true
}

/// # Safety
/// State, `url_len`, and a sufficiently large non-null output must remain
/// readable/writable. A null output performs the sizing pass.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_link_extract_url(
    state: *const TextLinkExtractState,
    index: usize,
    output: *mut u32,
    output_capacity: usize,
    url_len: *mut usize,
) -> bool {
    let (Some(link), Some(url_len)) =
        (unsafe { state.as_ref() }.and_then(|state| state.links.get(index)), unsafe {
            url_len.as_mut()
        })
    else {
        return false;
    };
    *url_len = link.url.len();
    if output_capacity == 0 {
        return output.is_null();
    }
    if output.is_null() || output_capacity < link.url.len() {
        return false;
    }
    unsafe { core::slice::from_raw_parts_mut(output, link.url.len()) }.copy_from_slice(&link.url);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn test_is_alphanumeric(
        _context: *mut core::ffi::c_void,
        character: u32,
    ) -> bool {
        char::from_u32(character).is_some_and(char::is_alphanumeric)
    }

    #[test]
    fn find_next_should_preserve_overlap_and_whole_word_behavior() {
        let text: Vec<u32> = "aaaa world worldly".chars().map(u32::from).collect();
        let mut overlap = TextFindState::new(
            text.clone(),
            vec!["aaaa".chars().map(u32::from).collect()],
            false,
            true,
            Some(0),
        );
        assert!(overlap.find_next());
        assert_eq!((0, 3), (overlap.result_start, overlap.result_end));

        let mut whole_word = TextFindState::new(
            text,
            vec!["world".chars().map(u32::from).collect()],
            true,
            false,
            Some(0),
        );
        assert!(whole_word.find_next());
        assert_eq!((5, 9), (whole_word.result_start, whole_word.result_end));
        assert!(!whole_word.find_next());
    }

    #[test]
    fn find_next_should_match_space_separated_words() {
        let text: Vec<u32> = "hello\r\nworld".chars().map(u32::from).collect();
        let mut state = TextFindState::new(
            text,
            vec![
                "hello".chars().map(u32::from).collect(),
                "world".chars().map(u32::from).collect(),
            ],
            false,
            false,
            Some(0),
        );
        assert!(state.find_next());
        assert_eq!((0, 11), (state.result_start, state.result_end));
    }

    #[test]
    fn query_tokenization_should_preserve_space_and_script_boundaries() {
        let code_points = |value: &str| value.chars().map(u32::from).collect::<Vec<_>>();
        assert_eq!(
            vec![Vec::new(), code_points("world")],
            extract_find_words(&code_points("  world"))
        );
        assert_eq!(
            vec![code_points("world"), Vec::new()],
            extract_find_words(&code_points("world  "))
        );
        assert_eq!(
            vec![code_points("abc"), code_points("漢"), code_points("def")],
            extract_find_words(&code_points("abc漢def"))
        );
        assert_eq!(vec![code_points("l’homme")], extract_find_words(&code_points("l’homme")));
    }

    #[test]
    fn link_extraction_should_preserve_web_and_mail_ranges() {
        let text: Vec<u32> =
            "See (www.Example.com), mail a.b@example.com.".chars().map(u32::from).collect();
        let mut state = TextLinkExtractState::default();
        assert!(state.extract(
            &text,
            &text,
            &vec![0; text.len()],
            core::ptr::null_mut(),
            test_is_alphanumeric,
        ));
        assert_eq!(2, state.links.len());
        let url = |link: &TextLink| {
            link.url.iter().filter_map(|value| char::from_u32(*value)).collect::<String>()
        };
        assert_eq!("http://www.Example.com", url(&state.links[0]));
        assert_eq!("mailto:a.b@example.com", url(&state.links[1]));
        assert_eq!(text.iter().position(|value| *value == b'w' as u32), Some(state.links[0].start));
    }

    #[test]
    fn web_link_ending_should_keep_ipv6_ports_and_trim_host_punctuation() {
        let ipv6: Vec<u32> = "[::1]:8080 trailing".chars().map(u32::from).collect();
        assert_eq!(9, find_web_link_ending(&ipv6, 0, ipv6.len() - 1));
        let host: Vec<u32> = "example.com)-".chars().map(u32::from).collect();
        assert_eq!(10, find_web_link_ending(&host, 0, host.len() - 1));
    }
}
