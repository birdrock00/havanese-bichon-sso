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

use crate::{
    error::{code::ErrorCode, BichonResult},
    raise_error,
};
use serde::{Deserialize, Serialize};
use std::cmp::min;

pub fn paginate_vec<T: Clone>(
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


#[cfg(not(feature = "web-api"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataPage<S>
where
    S: Serialize + std::fmt::Debug + std::marker::Unpin + Send + Sync,
{
    /// The current page number (starting from 1).
    pub current_page: Option<u64>,
    /// The number of items per page.
    pub page_size: Option<u64>,
    /// The total number of items across all pages.
    pub total_items: u64,
    /// The list of items returned on the current page.
    pub items: Vec<S>,
    /// The total number of pages. This is optional and may not be set if not calculated.
    pub total_pages: Option<u64>,
}
#[cfg(not(feature = "web-api"))]
impl<S: Serialize + std::fmt::Debug + std::marker::Unpin + Send + Sync> From<Paginated<S>>
    for DataPage<S>
{
    fn from(paginated: Paginated<S>) -> Self {
        DataPage {
            current_page: paginated.page,
            page_size: paginated.page_size,
            total_items: paginated.total_items,
            total_pages: paginated.total_pages,
            items: paginated.items,
        }
    }
}




#[cfg(feature = "web-api")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, poem_openapi::Object)]
pub struct DataPage<S>
where
    S: Serialize
        + std::fmt::Debug
        + std::marker::Unpin
        + Send
        + Sync
        + poem_openapi::types::Type
        + poem_openapi::types::ParseFromJSON
        + poem_openapi::types::ToJSON,
{
    /// The current page number (starting from 1).
    pub current_page: Option<u64>,
    /// The number of items per page.
    pub page_size: Option<u64>,
    /// The total number of items across all pages.
    pub total_items: u64,
    /// The list of items returned on the current page.
    pub items: Vec<S>,
    /// The total number of pages. This is optional and may not be set if not calculated.
    pub total_pages: Option<u64>,
}

#[cfg(feature = "web-api")]
impl<
        S: Serialize
            + std::fmt::Debug
            + std::marker::Unpin
            + Send
            + Sync
            + poem_openapi::types::Type
            + poem_openapi::types::ParseFromJSON
            + poem_openapi::types::ToJSON,
    > From<Paginated<S>> for DataPage<S>
{
    fn from(paginated: Paginated<S>) -> Self {
        DataPage {
            current_page: paginated.page,
            page_size: paginated.page_size,
            total_items: paginated.total_items,
            total_pages: paginated.total_pages,
            items: paginated.items,
        }
    }
}


#[derive(Debug)]
pub struct Paginated<T> {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub total_items: u64,
    pub total_pages: Option<u64>,
    pub items: Vec<T>,
}

impl<T> Paginated<T> {
    pub fn new(
        page: Option<u64>,
        page_size: Option<u64>,
        total_items: u64,
        total_pages: Option<u64>,
        items: Vec<T>,
    ) -> Self {
        Paginated {
            page,
            page_size,
            total_items,
            total_pages,
            items,
        }
    }
}
