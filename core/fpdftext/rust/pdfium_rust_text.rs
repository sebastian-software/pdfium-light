// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![allow(unsafe_code)]

type IndexMapCallback = unsafe extern "C" fn(*mut core::ffi::c_void, i32, *mut i32) -> bool;

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

fn read_u32_span(data: *const u32, len: usize) -> Option<Vec<u32>> {
    if len == 0 {
        return Some(Vec::new());
    }
    (!data.is_null()).then(|| unsafe { core::slice::from_raw_parts(data, len).to_vec() })
}

/// # Safety
/// All spans must remain readable for this call. `word_offsets` has
/// `word_count + 1` entries and partitions `word_data` monotonically.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_find_new(
    page_text: *const u32,
    page_text_len: usize,
    word_data: *const u32,
    word_data_len: usize,
    word_offsets: *const usize,
    word_count: usize,
    match_whole_word: bool,
    consecutive: bool,
    has_start: bool,
    start: usize,
) -> *mut TextFindState {
    let (Some(page_text), Some(word_data)) =
        (read_u32_span(page_text, page_text_len), read_u32_span(word_data, word_data_len))
    else {
        return core::ptr::null_mut();
    };
    if word_count == usize::MAX || word_offsets.is_null() {
        return core::ptr::null_mut();
    }
    let offsets = unsafe { core::slice::from_raw_parts(word_offsets, word_count + 1) };
    if offsets.first() != Some(&0)
        || offsets.last() != Some(&word_data.len())
        || offsets.windows(2).any(|pair| pair[0] > pair[1])
    {
        return core::ptr::null_mut();
    }
    let words = offsets.windows(2).map(|pair| word_data[pair[0]..pair[1]].to_vec()).collect();
    Box::into_raw(Box::new(TextFindState::new(
        page_text,
        words,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
