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


use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::modules::database::Paginated;

/// Represents a paginated response containing a subset of items along with pagination metadata.
///
/// This generic structure is commonly used to return paged data from list or search endpoints.
/// The type `S` represents the individual item type within the result set.
///
/// # Type Parameters
/// - `S`: The type of each item in the `items` list. Must implement several traits for serialization
///   and OpenAPI documentation.
///
/// # Fields
/// - `current_page`: The current page number (1-based). `None` if unspecified or not applicable.
/// - `page_size`: The number of items per page. `None` if unspecified.
/// - `total_items`: The total number of items matching the query.
/// - `items`: The list of items returned for the current page.
/// - `total_pages`: The total number of pages available. `None` if not calculated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Object)]
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
