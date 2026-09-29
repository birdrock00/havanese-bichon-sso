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

use crate::account::migration::AccountModel;
use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::ext::audit_detail::{
    decide_batch, load_deleted_snapshot, BatchAuditDecision, DetailRef,
};
use crate::ext::event_bus::{emit, Event};
use crate::raise_error;
use crate::store::tantivy::attachment::ATTACHMENT_MANAGER;
use crate::store::tantivy::envelope::ENVELOPE_MANAGER;
use std::collections::HashMap;

pub use crate::ext::audit_detail::DeletedMessageSnapshot;

/// Collect a snapshot for every message about to be deleted.
///
/// Lives in core rather than the REST handler because there is now more than
/// one caller: the community handler and the Pro dual-control executor both
/// delete messages, and both must capture the subject and content summary
/// *before* the messages are gone. A handler-private copy would have meant the
/// approval path silently emitting `email.deleted` records with no subject.
///
/// The per-message loading itself lives in `ext::audit_detail` so the inline
/// path and the detail-file writer share one definition of "the detail".
pub fn audit_snapshots_for_deleted(
    request: &HashMap<u64, Vec<String>>,
) -> Vec<DeletedMessageSnapshot> {
    let mut out = Vec::new();
    for (account_id, envelope_ids) in request {
        for eid in envelope_ids {
            out.push(load_deleted_snapshot(*account_id, eid));
        }
    }
    out
}

/// The plan a delete batch produces for the audit trail: either inline
/// per-message `email.deleted` events (snapshots already captured) or a single
/// `email.batch_deleted` referencing an externalized detail file.
pub enum DeleteAuditPlan {
    Inline(Vec<DeletedMessageSnapshot>),
    Detail(DetailRef),
}

/// Decide the audit shape and capture whatever must be captured *before* the
/// messages are gone. Call this, run the delete, and — only on success — hand
/// the plan to `emit_delete_audit`.
///
/// A delete that succeeds must never be missing from the trail, so the detail
/// file's failure to write degrades to the inline path rather than aborting.
pub fn plan_delete_audit(request: &HashMap<u64, Vec<String>>) -> DeleteAuditPlan {
    match decide_batch(request) {
        BatchAuditDecision::Detail(r) => DeleteAuditPlan::Detail(r),
        BatchAuditDecision::Inline => DeleteAuditPlan::Inline(audit_snapshots_for_deleted(request)),
    }
}

/// Emit the audit records for a delete that has already succeeded. Calling this
/// on a failed delete would write "deleted" records for messages that still
/// exist — the two call sites are the only enforcement of that ordering.
pub fn emit_delete_audit(plan: DeleteAuditPlan, user: &str) {
    match plan {
        DeleteAuditPlan::Inline(snapshots) => {
            for s in snapshots {
                emit(Event::EmailDeleted {
                    email_id: s.email_id,
                    user: user.to_string(),
                    account_id: s.account_id,
                    mailbox_id: s.mailbox_id,
                    subject: s.subject,
                    snapshot: s.snapshot,
                });
            }
        }
        DeleteAuditPlan::Detail(r) => {
            emit(Event::EmailBatchDeleted {
                user: user.to_string(),
                account_ids: r.account_ids,
                count: r.count,
                batch_id: r.batch_id,
                detail_file: r.rel_path,
                detail_size: r.size,
                detail_sha256: r.sha256,
            });
        }
    }
}

pub async fn delete_messages_impl(request: HashMap<u64, Vec<String>>) -> BichonResult<()> {
    // Legal hold: an account under a hold is frozen — no message may be
    // deleted from it, including manual single/batch deletes from the web UI
    // (retention and auto-expunge already skip held accounts at their own call
    // sites; this closes the manual-delete path). The check refuses the whole
    // request if any target account is held, so a batch can never partially
    // delete. In the community edition `legal_hold` is always false, so this
    // guard is a no-op there.
    for account_id in request.keys() {
        let account = AccountModel::get(*account_id)?;
        if account.is_on_hold() {
            return Err(raise_error!(
                format!(
                    "Account {} ({}) is under a legal hold; deleting its messages is disabled while the hold is active",
                    account.id, account.email
                ),
                ErrorCode::Forbidden
            ));
        }
    }
    ENVELOPE_MANAGER
        .delete_envelopes_multi_account(request.clone())
        .await?;
    ATTACHMENT_MANAGER
        .delete_attachments_multi_account(request)
        .await
}
