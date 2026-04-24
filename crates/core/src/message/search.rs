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

//use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::{
    common::paginated::DataPage,
    error::{code::ErrorCode, BichonResult},
    raise_error,
    store::{
        envelope::Envelope,
        tantivy::{
            attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER, model::AttachmentModel,
        },
    },
};

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct EmailSearchFilter {
    pub text: Option<String>,
    pub subject: Option<String>,
    pub id: Option<String>,
    pub body: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub cc: Option<String>,
    pub bcc: Option<String>,
    pub since: Option<i64>,
    pub before: Option<i64>,
    pub account_ids: Option<HashSet<u64>>,
    pub mailbox_ids: Option<HashSet<u64>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub message_id: Option<String>,
    pub has_attachment: Option<bool>,
    pub attachment_name: Option<String>,
    pub tags: Option<HashSet<String>>,
    pub attachment_extension: Option<String>,
    pub attachment_category: Option<String>,
    pub attachment_content_type: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum SortBy {
    #[default]
    DATE,
    SIZE,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct EmailSearchRequest {
    pub filter: EmailSearchFilter,
    pub page: u64,
    pub page_size: u64,
    pub sort_by: Option<SortBy>,
    pub desc: Option<bool>,
}
impl EmailSearchRequest {
    pub fn validate(&self) -> BichonResult<()> {
        if self.page == 0 || self.page_size == 0 {
            return Err(raise_error!(
                "Both page and page_size must be greater than 0.".into(),
                ErrorCode::InvalidParameter
            ));
        }
        if self.page_size > 500 {
            return Err(raise_error!(
                "The page_size exceeds the maximum allowed limit of 500.".into(),
                ErrorCode::InvalidParameter
            ));
        }

        Ok(())
    }
}

pub async fn search_messages_impl(
    accounts: Option<HashSet<u64>>,
    request: EmailSearchRequest,
) -> BichonResult<DataPage<Envelope>> {
    request.validate()?;
    ENVELOPE_MANAGER
        .search(
            accounts,
            request.filter,
            request.page,
            request.page_size,
            request.desc.unwrap_or(true),
            request.sort_by.unwrap_or(SortBy::DATE),
        )
        .await
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AttachmentSearchFilter {
    pub id: Option<String>,
    pub text: Option<String>,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub since: Option<i64>,
    pub before: Option<i64>,
    pub account_ids: Option<HashSet<u64>>,
    pub mailbox_ids: Option<HashSet<u64>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,

    pub attachment_name: Option<String>,
    pub content_hash: Option<String>,

    pub tags: Option<HashSet<String>>,
    pub attachment_extension: Option<String>,
    pub attachment_category: Option<String>,
    pub attachment_content_type: Option<String>,

    pub is_ocr: Option<bool>,
    pub is_message: Option<bool>,
    pub has_text: Option<bool>,

    pub min_page_count: Option<u64>,
    pub max_page_count: Option<u64>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct AttachmentSearchRequest {
    filter: AttachmentSearchFilter,
    page: u64,
    page_size: u64,
    sort_by: Option<SortBy>,
    desc: Option<bool>,
}
impl AttachmentSearchRequest {
    pub fn validate(&self) -> BichonResult<()> {
        if self.page == 0 || self.page_size == 0 {
            return Err(raise_error!(
                "Both page and page_size must be greater than 0.".into(),
                ErrorCode::InvalidParameter
            ));
        }
        if self.page_size > 500 {
            return Err(raise_error!(
                "The page_size exceeds the maximum allowed limit of 500.".into(),
                ErrorCode::InvalidParameter
            ));
        }

        Ok(())
    }
}

pub async fn search_attachment_impl(
    accounts: Option<HashSet<u64>>,
    request: AttachmentSearchRequest,
) -> BichonResult<DataPage<AttachmentModel>> {
    request.validate()?;
    ATTACHMENT_MANAGER
        .search(
            accounts,
            request.filter,
            request.page,
            request.page_size,
            request.desc.unwrap_or(true),
            request.sort_by.unwrap_or(SortBy::DATE),
        )
        .await
}
