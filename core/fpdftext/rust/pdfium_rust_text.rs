// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![allow(unsafe_code)]

type IndexMapCallback = unsafe extern "C" fn(*mut core::ffi::c_void, i32, *mut i32) -> bool;
type IsAlphanumericCallback = unsafe extern "C" fn(*mut core::ffi::c_void, u32) -> bool;
type TextRectCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    *mut f32,
    *mut f32,
    *mut f32,
    *mut f32,
) -> bool;
type TextSelectionRectCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    *mut bool,
    *mut usize,
    *mut f32,
    *mut f32,
    *mut f32,
    *mut f32,
) -> bool;
type TextPredicateCharacterCallback =
    unsafe extern "C" fn(*mut core::ffi::c_void, usize, *mut bool, *mut u32, *mut f32) -> bool;
type TextOrientationObjectCallback = unsafe extern "C" fn(
    *mut core::ffi::c_void,
    usize,
    *mut bool,
    *mut bool,
    *mut f32,
    *mut f32,
    *mut f32,
    *mut f32,
) -> bool;

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

pub struct TextIndexMapState {
    segments: Vec<(i32, i32)>,
}

pub struct TextSelectionRectState {
    rects: Vec<[f32; 4]>,
}

pub struct TextPredicateResultState {
    text: Vec<u32>,
}

pub struct TextLinePlanState {
    kept_indices: Vec<usize>,
    emissions: Vec<(usize, bool)>,
}

struct TextCharacterEmission {
    unicode: u32,
    append_text: bool,
    set_unicode: bool,
    char_type: u8,
}

pub struct TextAddCharacterPlanState {
    display_unicode: u32,
    original_char_type: u8,
    is_rtl: bool,
    needs_display_unicode: bool,
    needs_normalization: bool,
    emissions: Vec<TextCharacterEmission>,
}

impl TextAddCharacterPlanState {
    fn new(
        char_type: u8,
        char_code: u32,
        info_unicode: u32,
        display_unicode: u32,
        is_rtl: bool,
    ) -> Option<Self> {
        if char_type > 4 {
            return None;
        }
        let is_control =
            matches!(info_unicode, 0x2 | 0x3 | 0x93 | 0x94 | 0x96 | 0x97 | 0x98 | 0xfffe)
                && char_type != 3;
        let is_normal = if info_unicode != 0 { !is_control } else { char_code != 0 };
        let needs_display_unicode = is_normal && is_rtl;
        let needs_normalization =
            is_normal && !needs_display_unicode && (0xfb00..=0xfb06).contains(&display_unicode);
        let emissions = if is_normal && !needs_display_unicode && !needs_normalization {
            vec![TextCharacterEmission {
                unicode: display_unicode,
                append_text: true,
                set_unicode: is_rtl,
                char_type,
            }]
        } else if !is_normal {
            vec![TextCharacterEmission {
                unicode: info_unicode,
                append_text: false,
                set_unicode: false,
                char_type,
            }]
        } else {
            Vec::new()
        };
        Some(Self {
            display_unicode,
            original_char_type: char_type,
            is_rtl,
            needs_display_unicode,
            needs_normalization,
            emissions,
        })
    }

    fn set_display_unicode(&mut self, display_unicode: u32) -> bool {
        if !self.needs_display_unicode {
            return false;
        }
        self.display_unicode = display_unicode;
        self.needs_display_unicode = false;
        self.needs_normalization = true;
        true
    }

    fn set_normalization(&mut self, normalized: &[u32]) -> bool {
        if !self.needs_normalization {
            return false;
        }
        self.emissions = if normalized.is_empty() {
            vec![TextCharacterEmission {
                unicode: self.display_unicode,
                append_text: true,
                set_unicode: self.is_rtl,
                char_type: self.original_char_type,
            }]
        } else {
            normalized
                .iter()
                .map(|unicode| TextCharacterEmission {
                    unicode: *unicode,
                    append_text: true,
                    set_unicode: true,
                    char_type: 4,
                })
                .collect()
        };
        self.needs_normalization = false;
        true
    }
}

#[repr(C)]
pub struct TextBidiSegment {
    start: usize,
    count: usize,
    direction: u8,
}

impl TextLinePlanState {
    fn new(text: &[u32]) -> Self {
        let mut kept_indices = Vec::with_capacity(text.len());
        let mut previous_was_space = false;
        for (index, character) in text.iter().enumerate() {
            let is_space = *character == b' ' as u32;
            if !is_space || !previous_was_space {
                kept_indices.push(index);
            }
            previous_was_space = is_space;
        }
        Self { kept_indices, emissions: Vec::new() }
    }

    fn set_segments(&mut self, overall_direction: u8, segments: &[TextBidiSegment]) -> bool {
        if overall_direction != 1 && overall_direction != 2 {
            return false;
        }
        let mut emissions = Vec::with_capacity(self.kept_indices.len());
        let mut current_direction = overall_direction;
        for segment in segments {
            let Some(end) = segment.start.checked_add(segment.count) else {
                return false;
            };
            if end > self.kept_indices.len() || segment.direction > 3 {
                return false;
            }
            let is_rtl =
                segment.direction == 2 || (segment.direction == 0 && current_direction == 2);
            if is_rtl {
                current_direction = 2;
                for index in (segment.start..end).rev() {
                    emissions.push((index, true));
                }
            } else {
                if segment.direction != 3 {
                    current_direction = 1;
                }
                for index in segment.start..end {
                    emissions.push((index, false));
                }
            }
        }
        self.emissions = emissions;
        true
    }
}

impl TextIndexMapState {
    fn new(included: &[u8]) -> Option<Self> {
        if included.len() > i32::MAX as usize {
            return None;
        }

        let mut segments = Vec::new();
        let mut current = (0, 0);
        let mut skipped = false;
        for (index, is_included) in included.iter().enumerate() {
            if *is_included != 0 {
                current.1 += 1;
                skipped = true;
            } else if skipped {
                segments.push(current);
                current = (index as i32 + 1, 0);
                skipped = false;
            } else {
                current.0 = index as i32 + 1;
            }
        }
        if !included.is_empty() {
            segments.push(current);
        }
        Some(Self { segments })
    }

    fn character_from_text(&self, text_index: i32) -> i32 {
        let mut count = 0;
        for &(index, segment_count) in &self.segments {
            count += segment_count;
            if count > text_index {
                return text_index - count + segment_count + index;
            }
        }
        -1
    }

    fn text_from_character(&self, character_index: i32) -> i32 {
        let mut count = 0;
        for &(index, segment_count) in &self.segments {
            let text_index = character_index - index;
            if text_index < segment_count {
                return if text_index >= 0 { text_index + count } else { -1 };
            }
            count += segment_count;
        }
        -1
    }

    fn page_text_range(
        &self,
        mut start: i32,
        count: i32,
        character_count: i32,
    ) -> Option<(i32, i32)> {
        if start < 0 || start >= character_count || count <= 0 {
            return None;
        }

        let mut text_start = self.text_from_character(start);
        while text_start < 0 {
            if start >= character_count {
                return None;
            }
            start += 1;
            text_start = self.text_from_character(start);
        }

        let count = count.min(character_count - start);
        let mut last = start + count - 1;
        let mut text_last = self.text_from_character(last);
        while text_last < 0 {
            if last < text_start {
                return None;
            }
            last -= 1;
            text_last = self.text_from_character(last);
        }
        (text_last >= text_start).then_some((text_start, text_last - text_start + 1))
    }
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

fn normalize_rect(rect: &mut [f32; 4]) {
    if rect[0] > rect[2] {
        rect.swap(0, 2);
    }
    if rect[1] > rect[3] {
        rect.swap(1, 3);
    }
}

fn contains_point(rect: &[f32; 4], x: f32, y: f32) -> bool {
    x <= rect[2] && x >= rect[0] && y <= rect[3] && y >= rect[1]
}

fn union_rect(rect: &mut [f32; 4], mut other: [f32; 4]) {
    normalize_rect(rect);
    normalize_rect(&mut other);
    rect[0] = if other[0] < rect[0] { other[0] } else { rect[0] };
    rect[1] = if other[1] < rect[1] { other[1] } else { rect[1] };
    rect[2] = if rect[2] < other[2] { other[2] } else { rect[2] };
    rect[3] = if rect[3] < other[3] { other[3] } else { rect[3] };
}

fn selection_rects(
    character_count: usize,
    start: i32,
    count: i32,
    mut get_character: impl FnMut(usize) -> Option<(bool, usize, [f32; 4])>,
) -> Option<Vec<[f32; 4]>> {
    let mut rects = Vec::new();
    if start < 0 || count == 0 {
        return Some(rects);
    }
    let start = start as usize;
    if start >= character_count {
        return Some(rects);
    }
    let end = if count < 0 {
        character_count
    } else {
        start.saturating_add(count as usize).min(character_count)
    };

    let mut text_object = 0;
    let mut rect = [0.0; 4];
    let mut is_new_rect = true;
    for position in start..end {
        let (generated, character_object, character_rect) = get_character(position)?;
        if generated
            || character_rect[2] - character_rect[0] < 0.01
            || character_rect[3] - character_rect[1] < 0.01
        {
            continue;
        }
        if text_object == 0 {
            text_object = character_object;
        }
        if text_object != character_object {
            rects.push(rect);
            text_object = character_object;
            is_new_rect = true;
        }
        if is_new_rect {
            is_new_rect = false;
            rect = character_rect;
            normalize_rect(&mut rect);
        } else {
            union_rect(&mut rect, character_rect);
        }
    }
    rects.push(rect);
    Some(rects)
}

fn text_by_predicate(
    character_count: usize,
    mut get_character: impl FnMut(usize) -> Option<(bool, u32, f32)>,
) -> Option<Vec<u32>> {
    let mut position_y = 0.0_f32;
    let mut contains_previous_character = false;
    let mut add_line_feed = false;
    let mut text = Vec::new();
    for index in 0..character_count {
        let (included, unicode, origin_y) = get_character(index)?;
        if included {
            if (position_y - origin_y).abs() > 0.0 && !contains_previous_character && add_line_feed
            {
                position_y = origin_y;
                if !text.is_empty() {
                    text.extend([13, 10]);
                }
            }
            contains_previous_character = true;
            add_line_feed = false;
            if unicode != 0 {
                text.push(unicode);
            }
        } else if unicode == b' ' as u32 {
            if contains_previous_character {
                text.push(unicode);
                contains_previous_character = false;
                add_line_feed = false;
            }
        } else {
            contains_previous_character = false;
            add_line_feed = true;
        }
    }
    Some(text)
}

fn clamp_coordinate(value: f32, maximum: i32) -> i32 {
    if value < 0.0 {
        0
    } else if value > maximum as f32 {
        maximum
    } else {
        value as i32
    }
}

fn mask_percent_filled(mask: &[bool], start: i32, end: i32) -> f32 {
    if start >= end {
        return 0.0;
    }
    let start = start as usize;
    let end = end as usize;
    mask[start..end].iter().filter(|value| **value).count() as f32 / (end - start) as f32
}

fn text_flow_orientation(
    page_width: i32,
    page_height: i32,
    object_count: usize,
    mut get_object: impl FnMut(usize) -> Option<(bool, bool, [f32; 4])>,
) -> Option<u8> {
    if page_width <= 0 || page_height <= 0 {
        return Some(0);
    }

    let mut horizontal_mask = vec![false; page_width as usize];
    let mut vertical_mask = vec![false; page_height as usize];
    let mut line_height = 0.0_f32;
    let mut start_h = page_width;
    let mut end_h = 0;
    let mut start_v = page_height;
    let mut end_v = 0;
    for index in 0..object_count {
        let (active, is_text, rect) = get_object(index)?;
        if !active || !is_text {
            continue;
        }
        let min_h = clamp_coordinate(rect[0], page_width);
        let max_h = clamp_coordinate(rect[2], page_width);
        let min_v = clamp_coordinate(rect[1], page_height);
        let max_v = clamp_coordinate(rect[3], page_height);
        if min_h >= max_h || min_v >= max_v {
            continue;
        }
        horizontal_mask[min_h as usize..max_h as usize].fill(true);
        vertical_mask[min_v as usize..max_v as usize].fill(true);
        start_h = start_h.min(min_h);
        end_h = end_h.max(max_h);
        start_v = start_v.min(min_v);
        end_v = end_v.max(max_v);
        if line_height <= 0.0 {
            line_height = rect[3] - rect[1];
        }
    }

    let double_line_height = (2.0 * line_height) as i32;
    if end_v - start_v < double_line_height {
        return Some(1);
    }
    if end_h - start_h < double_line_height {
        return Some(2);
    }
    let sum_h = mask_percent_filled(&horizontal_mask, start_h, end_h);
    if sum_h > 0.8 {
        return Some(1);
    }
    let sum_v = mask_percent_filled(&vertical_mask, start_v, end_v);
    Some(if sum_h > sum_v {
        1
    } else if sum_h < sum_v {
        2
    } else {
        0
    })
}

fn text_object_writing_mode(
    character_count: usize,
    fallback_orientation: u8,
    first_origin: [f32; 2],
    last_origin: [f32; 2],
) -> Option<u8> {
    if fallback_orientation > 2 {
        return None;
    }
    if character_count <= 1 {
        return Some(fallback_orientation);
    }

    let difference_x = (last_origin[0] - first_origin[0]).abs();
    let difference_y = (last_origin[1] - first_origin[1]).abs();
    if difference_x <= 0.0001 && difference_y <= 0.0001 {
        return Some(0);
    }
    let length = (difference_x * difference_x + difference_y * difference_y).sqrt();
    let x_under_threshold = difference_x / length <= 0.0872;
    if difference_y / length <= 0.0872 {
        return Some(if x_under_threshold { fallback_orientation } else { 1 });
    }
    Some(if x_under_threshold { 2 } else { fallback_orientation })
}

fn index_at_position(
    character_count: usize,
    point_x: f32,
    point_y: f32,
    tolerance_width: f32,
    tolerance_height: f32,
    mut get_rect: impl FnMut(usize) -> Option<[f32; 4]>,
) -> Option<i32> {
    if character_count > i32::MAX as usize {
        return None;
    }

    let mut near_position = -1;
    let mut x_difference = 5000.0_f64;
    let mut y_difference = 5000.0_f64;
    for position in 0..character_count {
        let mut rect = get_rect(position)?;
        normalize_rect(&mut rect);
        if contains_point(&rect, point_x, point_y) {
            return Some(position as i32);
        }
        if tolerance_width <= 0.0 && tolerance_height <= 0.0 {
            continue;
        }

        let extended = [
            rect[0] - tolerance_width / 2.0,
            rect[1] - tolerance_height / 2.0,
            rect[2] + tolerance_width / 2.0,
            rect[3] + tolerance_height / 2.0,
        ];
        if !contains_point(&extended, point_x, point_y) {
            continue;
        }
        let current_x_difference =
            f64::from((point_x - rect[0]).abs().min((point_x - rect[2]).abs()));
        let current_y_difference =
            f64::from((point_y - rect[1]).abs().min((point_y - rect[3]).abs()));
        if current_y_difference + current_x_difference < x_difference + y_difference {
            x_difference = current_x_difference;
            y_difference = current_y_difference;
            near_position = position as i32;
        }
    }
    Some(near_position)
}

/// Computes the dominant text-line flow orientation from page-object bounds.
///
/// # Safety
/// The callback, context, and output must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_flow_orientation(
    page_width: i32,
    page_height: i32,
    object_count: usize,
    context: *mut core::ffi::c_void,
    get_object: Option<TextOrientationObjectCallback>,
    output: *mut u8,
) -> bool {
    let (Some(get_object), Some(output)) = (get_object, unsafe { output.as_mut() }) else {
        return false;
    };
    let orientation = text_flow_orientation(page_width, page_height, object_count, |index| {
        let mut active = false;
        let mut is_text = false;
        let mut rect = [0.0; 4];
        // SAFETY: The caller guarantees that the callback and context remain valid
        // for this synchronous call, and all output pointers reference local values.
        if !unsafe {
            get_object(
                context,
                index,
                &mut active,
                &mut is_text,
                &mut rect[0],
                &mut rect[1],
                &mut rect[2],
                &mut rect[3],
            )
        } {
            return None;
        }
        Some((active, is_text, rect))
    });
    let Some(orientation) = orientation else {
        return false;
    };
    *output = orientation;
    true
}

/// Selects the writing mode for a transformed text object's endpoint origins.
///
/// # Safety
/// The output must be valid for one byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_object_writing_mode(
    character_count: usize,
    fallback_orientation: u8,
    first_x: f32,
    first_y: f32,
    last_x: f32,
    last_y: f32,
    output: *mut u8,
) -> bool {
    let Some(output) = (unsafe { output.as_mut() }) else {
        return false;
    };
    let Some(orientation) = text_object_writing_mode(
        character_count,
        fallback_orientation,
        [first_x, first_y],
        [last_x, last_y],
    ) else {
        return false;
    };
    *output = orientation;
    true
}

/// Computes the character nearest to a point within the requested tolerance.
///
/// # Safety
/// The callback, context, and output must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_index_at_position(
    character_count: usize,
    point_x: f32,
    point_y: f32,
    tolerance_width: f32,
    tolerance_height: f32,
    context: *mut core::ffi::c_void,
    get_rect: Option<TextRectCallback>,
    output: *mut i32,
) -> bool {
    let (Some(get_rect), Some(output)) = (get_rect, unsafe { output.as_mut() }) else {
        return false;
    };
    let result = index_at_position(
        character_count,
        point_x,
        point_y,
        tolerance_width,
        tolerance_height,
        |position| {
            let mut rect = [0.0; 4];
            unsafe {
                get_rect(context, position, &mut rect[0], &mut rect[1], &mut rect[2], &mut rect[3])
            }
            .then_some(rect)
        },
    );
    let Some(result) = result else {
        return false;
    };
    *output = result;
    true
}

/// Builds and owns the selected character rectangles.
///
/// # Safety
/// The callback and context must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_selection_rects_new(
    character_count: usize,
    start: i32,
    count: i32,
    context: *mut core::ffi::c_void,
    get_character: Option<TextSelectionRectCallback>,
) -> *mut TextSelectionRectState {
    let Some(get_character) = get_character else {
        return core::ptr::null_mut();
    };
    let rects = selection_rects(character_count, start, count, |position| {
        let mut generated = false;
        let mut text_object = 0;
        let mut rect = [0.0; 4];
        unsafe {
            get_character(
                context,
                position,
                &mut generated,
                &mut text_object,
                &mut rect[0],
                &mut rect[1],
                &mut rect[2],
                &mut rect[3],
            )
        }
        .then_some((generated, text_object, rect))
    });
    let Some(rects) = rects else {
        return core::ptr::null_mut();
    };
    Box::into_raw(Box::new(TextSelectionRectState { rects }))
}

/// # Safety
/// `state` must be null or returned by the matching constructor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_selection_rects_free(state: *mut TextSelectionRectState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_selection_rects_count(
    state: *const TextSelectionRectState,
) -> usize {
    unsafe { state.as_ref() }.map_or(0, |state| state.rects.len())
}

/// # Safety
/// State and rectangle outputs must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_selection_rects_get(
    state: *const TextSelectionRectState,
    index: usize,
    left: *mut f32,
    bottom: *mut f32,
    right: *mut f32,
    top: *mut f32,
) -> bool {
    let (Some(rect), Some(left), Some(bottom), Some(right), Some(top)) = (
        unsafe { state.as_ref() }.and_then(|state| state.rects.get(index)),
        unsafe { left.as_mut() },
        unsafe { bottom.as_mut() },
        unsafe { right.as_mut() },
        unsafe { top.as_mut() },
    ) else {
        return false;
    };
    *left = rect[0];
    *bottom = rect[1];
    *right = rect[2];
    *top = rect[3];
    true
}

/// Builds and owns text selected by a caller-provided character predicate.
///
/// # Safety
/// The callback and context must remain valid synchronously.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_predicate_result_new(
    character_count: usize,
    context: *mut core::ffi::c_void,
    get_character: Option<TextPredicateCharacterCallback>,
) -> *mut TextPredicateResultState {
    let Some(get_character) = get_character else {
        return core::ptr::null_mut();
    };
    let text = text_by_predicate(character_count, |index| {
        let mut included = false;
        let mut unicode = 0;
        let mut origin_y = 0.0;
        unsafe { get_character(context, index, &mut included, &mut unicode, &mut origin_y) }
            .then_some((included, unicode, origin_y))
    });
    let Some(text) = text else {
        return core::ptr::null_mut();
    };
    Box::into_raw(Box::new(TextPredicateResultState { text }))
}

/// # Safety
/// `state` must be null or returned by the matching constructor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_predicate_result_free(
    state: *mut TextPredicateResultState,
) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_predicate_result_len(
    state: *const TextPredicateResultState,
) -> usize {
    unsafe { state.as_ref() }.map_or(0, |state| state.text.len())
}

/// # Safety
/// State and a sufficiently large output span must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_predicate_result_copy(
    state: *const TextPredicateResultState,
    output: *mut u32,
    output_capacity: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_ref() }) else {
        return false;
    };
    if state.text.is_empty() {
        return output_capacity == 0;
    }
    if output.is_null() || output_capacity < state.text.len() {
        return false;
    }
    unsafe { core::slice::from_raw_parts_mut(output, state.text.len()) }
        .copy_from_slice(&state.text);
    true
}

/// Creates the duplicate-space normalization and Bidi emission plan state.
///
/// # Safety
/// The code-point span must remain readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_line_plan_new(
    text: *const u32,
    text_len: usize,
) -> *mut TextLinePlanState {
    let text = if text_len == 0 {
        &[]
    } else if text.is_null() {
        return core::ptr::null_mut();
    } else {
        unsafe { core::slice::from_raw_parts(text, text_len) }
    };
    Box::into_raw(Box::new(TextLinePlanState::new(text)))
}

/// # Safety
/// `state` must be null or returned by the matching constructor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_line_plan_free(state: *mut TextLinePlanState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_line_plan_kept_count(
    state: *const TextLinePlanState,
) -> usize {
    unsafe { state.as_ref() }.map_or(0, |state| state.kept_indices.len())
}

/// # Safety
/// State and output must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_line_plan_kept_index(
    state: *const TextLinePlanState,
    index: usize,
    source_index: *mut usize,
) -> bool {
    let (Some(source), Some(output)) =
        (unsafe { state.as_ref() }.and_then(|state| state.kept_indices.get(index)), unsafe {
            source_index.as_mut()
        })
    else {
        return false;
    };
    *output = *source;
    true
}

/// Replaces the emission order using native-classified Bidi segments.
///
/// # Safety
/// State and the segment span must remain readable/writable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_line_plan_set_segments(
    state: *mut TextLinePlanState,
    overall_direction: u8,
    segments: *const TextBidiSegment,
    segment_count: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    let segments = if segment_count == 0 {
        &[]
    } else if segments.is_null() {
        return false;
    } else {
        unsafe { core::slice::from_raw_parts(segments, segment_count) }
    };
    state.set_segments(overall_direction, segments)
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_line_plan_emission_count(
    state: *const TextLinePlanState,
) -> usize {
    unsafe { state.as_ref() }.map_or(0, |state| state.emissions.len())
}

/// # Safety
/// State and outputs must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_line_plan_emission(
    state: *const TextLinePlanState,
    index: usize,
    character_index: *mut usize,
    is_rtl: *mut bool,
) -> bool {
    let (Some(emission), Some(character_index), Some(is_rtl)) = (
        unsafe { state.as_ref() }.and_then(|state| state.emissions.get(index)),
        unsafe { character_index.as_mut() },
        unsafe { is_rtl.as_mut() },
    ) else {
        return false;
    };
    *character_index = emission.0;
    *is_rtl = emission.1;
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn pdfium_rust_text_add_character_plan_new(
    char_type: u8,
    char_code: u32,
    info_unicode: u32,
    display_unicode: u32,
    is_rtl: bool,
) -> *mut TextAddCharacterPlanState {
    let Some(state) =
        TextAddCharacterPlanState::new(char_type, char_code, info_unicode, display_unicode, is_rtl)
    else {
        return core::ptr::null_mut();
    };
    Box::into_raw(Box::new(state))
}

/// # Safety
/// `state` must be null or returned by the matching constructor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_add_character_plan_free(
    state: *mut TextAddCharacterPlanState,
) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_add_character_plan_needs_normalization(
    state: *const TextAddCharacterPlanState,
) -> bool {
    unsafe { state.as_ref() }.is_some_and(|state| state.needs_normalization)
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_add_character_plan_needs_display_unicode(
    state: *const TextAddCharacterPlanState,
) -> bool {
    unsafe { state.as_ref() }.is_some_and(|state| state.needs_display_unicode)
}

/// Supplies the native mirrored display character when requested by Rust.
///
/// # Safety
/// `state` must remain writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_add_character_plan_set_display_unicode(
    state: *mut TextAddCharacterPlanState,
    display_unicode: u32,
) -> bool {
    unsafe { state.as_mut() }.is_some_and(|state| state.set_display_unicode(display_unicode))
}

/// Supplies native Unicode normalization output when requested by Rust.
///
/// # Safety
/// State and the code-point span must remain readable/writable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_add_character_plan_set_normalization(
    state: *mut TextAddCharacterPlanState,
    normalized: *const u32,
    normalized_len: usize,
) -> bool {
    let Some(state) = (unsafe { state.as_mut() }) else {
        return false;
    };
    let normalized = if normalized_len == 0 {
        &[]
    } else if normalized.is_null() {
        return false;
    } else {
        unsafe { core::slice::from_raw_parts(normalized, normalized_len) }
    };
    state.set_normalization(normalized)
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_add_character_plan_emission_count(
    state: *const TextAddCharacterPlanState,
) -> usize {
    unsafe { state.as_ref() }.map_or(0, |state| state.emissions.len())
}

/// # Safety
/// State and all outputs must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_add_character_plan_emission(
    state: *const TextAddCharacterPlanState,
    index: usize,
    unicode: *mut u32,
    append_text: *mut bool,
    set_unicode: *mut bool,
    char_type: *mut u8,
) -> bool {
    let (Some(emission), Some(unicode), Some(append_text), Some(set_unicode), Some(char_type)) = (
        unsafe { state.as_ref() }.and_then(|state| state.emissions.get(index)),
        unsafe { unicode.as_mut() },
        unsafe { append_text.as_mut() },
        unsafe { set_unicode.as_mut() },
        unsafe { char_type.as_mut() },
    ) else {
        return false;
    };
    *unicode = emission.unicode;
    *append_text = emission.append_text;
    *set_unicode = emission.set_unicode;
    *char_type = emission.char_type;
    true
}

/// # Safety
/// The inclusion span must remain readable for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_index_map_new(
    included: *const u8,
    included_len: usize,
) -> *mut TextIndexMapState {
    let included = if included_len == 0 {
        &[]
    } else if included.is_null() {
        return core::ptr::null_mut();
    } else {
        unsafe { core::slice::from_raw_parts(included, included_len) }
    };
    let Some(state) = TextIndexMapState::new(included) else {
        return core::ptr::null_mut();
    };
    Box::into_raw(Box::new(state))
}

/// # Safety
/// `state` must be null or returned by the matching constructor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_index_map_free(state: *mut TextIndexMapState) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_index_map_character_from_text(
    state: *const TextIndexMapState,
    text_index: i32,
) -> i32 {
    unsafe { state.as_ref() }.map_or(-1, |state| state.character_from_text(text_index))
}

/// # Safety
/// `state` must remain readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_index_map_text_from_character(
    state: *const TextIndexMapState,
    character_index: i32,
) -> i32 {
    unsafe { state.as_ref() }.map_or(-1, |state| state.text_from_character(character_index))
}

/// Selects the visible text-buffer range for a character range.
///
/// # Safety
/// State and outputs must remain readable/writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_index_map_page_text_range(
    state: *const TextIndexMapState,
    start: i32,
    count: i32,
    character_count: i32,
    text_start: *mut i32,
    text_count: *mut i32,
) -> bool {
    let (Some(state), Some(text_start), Some(text_count)) =
        (unsafe { state.as_ref() }, unsafe { text_start.as_mut() }, unsafe { text_count.as_mut() })
    else {
        return false;
    };
    let Some((range_start, range_count)) = state.page_text_range(start, count, character_count)
    else {
        return false;
    };
    *text_start = range_start;
    *text_count = range_count;
    true
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
    fn text_index_map_should_skip_excluded_character_runs() {
        let state = TextIndexMapState::new(&[1, 1, 0, 1, 0, 0, 1])
            .expect("the test input fits the index representation");
        assert_eq!(
            vec![0, 1, 3, 6, -1],
            (0..=4).map(|index| state.character_from_text(index)).collect::<Vec<_>>()
        );
        assert_eq!(
            vec![0, 1, -1, 2, -1, -1, 3, -1],
            (0..=7).map(|index| state.text_from_character(index)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn page_text_range_should_trim_excluded_range_edges() {
        let state = TextIndexMapState::new(&[0, 1, 1, 0, 1])
            .expect("the test input fits the index representation");
        assert_eq!(Some((0, 3)), state.page_text_range(0, 5, 5));
        assert_eq!(Some((0, 3)), state.page_text_range(0, 4, 5));
        assert_eq!(Some((0, 1)), state.page_text_range(0, 1, 5));
    }

    #[test]
    fn index_at_position_should_prefer_containment_then_nearest_character() {
        let rects = vec![[0.0, 0.0, 10.0, 10.0], [20.0, 0.0, 30.0, 10.0]];
        assert_eq!(
            Some(0),
            index_at_position(rects.len(), 5.0, 5.0, 0.0, 0.0, |i| rects.get(i).copied())
        );
        assert_eq!(
            Some(1),
            index_at_position(rects.len(), 18.0, 5.0, 6.0, 0.0, |i| rects.get(i).copied())
        );
    }

    #[test]
    fn selection_rects_should_skip_and_group_characters_by_text_object() {
        let characters = [
            (false, 1, [0.0, 0.0, 5.0, 10.0]),
            (true, 1, [5.0, 0.0, 6.0, 10.0]),
            (false, 1, [5.0, 0.0, 10.0, 10.0]),
            (false, 2, [20.0, 0.0, 30.0, 10.0]),
        ];
        assert_eq!(
            Some(vec![[0.0, 0.0, 10.0, 10.0], [20.0, 0.0, 30.0, 10.0]]),
            selection_rects(characters.len(), 0, -1, |index| characters.get(index).copied())
        );
    }

    #[test]
    fn text_by_predicate_should_preserve_spaces_and_insert_line_breaks() {
        let characters = [
            (true, b'A' as u32, 10.0),
            (false, b' ' as u32, 10.0),
            (true, b'B' as u32, 10.0),
            (false, b'X' as u32, 10.0),
            (true, b'C' as u32, 20.0),
        ];
        assert_eq!(
            Some(vec![b'A' as u32, b' ' as u32, b'B' as u32, 13, 10, b'C' as u32]),
            text_by_predicate(characters.len(), |index| characters.get(index).copied())
        );
    }

    #[test]
    fn text_flow_orientation_should_distinguish_horizontal_and_vertical_runs() {
        let horizontal = [(true, true, [10.0, 10.0, 90.0, 20.0])];
        assert_eq!(
            Some(1),
            text_flow_orientation(100, 100, horizontal.len(), |index| {
                horizontal.get(index).copied()
            })
        );

        let vertical = [
            (true, true, [10.0, 10.0, 20.0, 20.0]),
            (true, true, [10.0, 40.0, 20.0, 50.0]),
            (true, true, [10.0, 70.0, 20.0, 80.0]),
        ];
        assert_eq!(
            Some(2),
            text_flow_orientation(100, 100, vertical.len(), |index| {
                vertical.get(index).copied()
            })
        );
    }

    #[test]
    fn text_object_writing_mode_should_use_endpoints_and_fallback() {
        assert_eq!(Some(1), text_object_writing_mode(2, 0, [0.0, 0.0], [10.0, 0.0]));
        assert_eq!(Some(2), text_object_writing_mode(2, 0, [0.0, 0.0], [0.0, 10.0]));
        assert_eq!(Some(1), text_object_writing_mode(1, 1, [0.0, 0.0], [0.0, 0.0]));
        assert_eq!(Some(0), text_object_writing_mode(2, 2, [5.0, 5.0], [5.0, 5.0]));
    }

    #[test]
    fn text_line_plan_should_collapse_spaces_and_reverse_rtl_segments() {
        let mut plan =
            TextLinePlanState::new(&[b'A' as u32, b' ' as u32, b' ' as u32, 0x05d0, 0x05d1]);
        assert_eq!(vec![0, 1, 3, 4], plan.kept_indices);
        assert!(plan.set_segments(
            1,
            &[
                TextBidiSegment { start: 0, count: 2, direction: 1 },
                TextBidiSegment { start: 2, count: 2, direction: 2 },
            ],
        ));
        assert_eq!(vec![(0, false), (1, false), (3, true), (2, true)], plan.emissions);
    }

    #[test]
    fn add_character_plan_should_classify_controls_and_expand_ligatures() {
        let control = TextAddCharacterPlanState::new(0, 2, 2, 2, false)
            .expect("the test character type is valid");
        assert!(!control.emissions[0].append_text);

        let mut ligature = TextAddCharacterPlanState::new(0, 0xfb00, 0xfb00, 0xfb00, false)
            .expect("the test character type is valid");
        assert!(ligature.needs_normalization);
        assert!(ligature.set_normalization(&[b'f' as u32, b'f' as u32]));
        assert_eq!(
            vec![(b'f' as u32, 4), (b'f' as u32, 4)],
            ligature
                .emissions
                .iter()
                .map(|emission| (emission.unicode, emission.char_type))
                .collect::<Vec<_>>()
        );

        let mut rtl =
            TextAddCharacterPlanState::new(0, b'(' as u32, b'(' as u32, b'(' as u32, true)
                .expect("the test character type is valid");
        assert!(rtl.needs_display_unicode);
        assert!(rtl.set_display_unicode(b')' as u32));
        assert!(rtl.needs_normalization);
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
