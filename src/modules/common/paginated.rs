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


use std::cmp::min;

use crate::{
    modules::{
        database::Paginated,
        error::{code::ErrorCode, BichonResult},
    },
    raise_error,
};

pub fn  paginate_vec<T: Clone>(
    items: &Vec<T>,
    page: Option<u64>,
    page_size: Option<u64>,
) -> BichonResult<Paginated<T>> {
    let total_items = items.len() as u64;

    let (offset, total_pages) = match (page, page_size) {
        (Some(p), Some(s)) if p > 0 && s > 0 => {
            let offset = (p - 1) * s;
            let total_pages = if total_items > 0 {
                (total_items + s - 1) / s
            } else {
                0
            };
            (Some(offset), Some(total_pages))
        }
        (Some(0), _) | (_, Some(0)) => {
            return Err(raise_error!(
                "'page' and 'page_size' must be greater than 0.".into(),
                ErrorCode::InvalidParameter
            ));
        }
        _ => (None, None),
    };

    let data = match offset {
        Some(offset) if offset >= total_items => vec![],
        Some(offset) => {
            let end = min(offset + page_size.unwrap_or(total_items), total_items) as usize;
            items[offset as usize..end].to_vec()
        }
        None => items.clone(),
    };

    Ok(Paginated::new(
        page,
        page_size,
        total_items,
        total_pages,
        data,
    ))
}
