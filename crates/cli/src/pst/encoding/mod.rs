//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


use compressed_rtf::*;
use outlook_pst::ltp::prop_context::PropertyValue;

pub fn decode_subject(value: &PropertyValue) -> Option<String> {
    match value {
        PropertyValue::String8(value) => {
            let offset = match value.buffer().first() {
                Some(1) => 2,
                _ => 0,
            };
            let buffer: Vec<_> = value
                .buffer()
                .iter()
                .skip(offset)
                .map(|&b| u16::from(b))
                .collect();
            Some(String::from_utf16_lossy(&buffer))
        }
        PropertyValue::Unicode(value) => {
            let offset = match value.buffer().first() {
                Some(1) => 2,
                _ => 0,
            };
            Some(String::from_utf16_lossy(&value.buffer()[offset..]))
        }
        _ => None,
    }
}

pub fn decode_html_body(buffer: &[u8], code_page: u16) -> Option<String> {
    match code_page {
        20127 => {
            let buffer: Vec<_> = buffer.iter().map(|&b| u16::from(b)).collect();
            Some(String::from_utf16_lossy(&buffer))
        }
        _ => {
            let coding = codepage_strings::Coding::new(code_page).ok()?;
            Some(coding.decode(buffer).ok()?.to_string())
        }
    }
}

pub fn decode_rtf_compressed(buffer: &[u8]) -> Option<String> {
    decompress_rtf(buffer).ok()
}
