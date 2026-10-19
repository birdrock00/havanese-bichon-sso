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
//
// View accounts: read-only, filter-scoped access to other accounts.
//
// A view is a `NoSync` account with `kind.role = view`. It owns no documents;
// it is an *authorization and query* construct. Two things must therefore
// happen for it to be safe:
//
//   1. **Search** must expand the view into `(source account AND source
//      filter)` groups, so the filter narrows every result.
//   2. **Every read path** must check the filter against the concrete message,
//      because search expansion alone is not a security boundary — a client
//      can ask for a message by id.
//
// Both live here so the query side and the per-message side cannot drift apart:
// a message the search returns must be a message `allows` accepts.
//
// Matching is **exact, case-insensitive address equality** on the envelope's
// from/to/cc/bcc. That is deliberately stricter than the fuzzy full-text
// `any_participant` search filter (which tokenizes and stems, so
// `zhang@firma.de` would also match unrelated mail from `firma.de`): a fuzzy
// rule is fine as a convenience filter and unacceptable as an authorization
// boundary.

use std::collections::HashSet;

use crate::account::migration::AccountModel;
use crate::common::auth::ClientContext;
use crate::database::{list_all_impl, manager::DB_MANAGER};
use crate::error::BichonResult;
use crate::store::envelope::Envelope;
use crate::users::permissions::Permission;

/// One `(account, filter)` pair a search may read.
///
/// `None` filters mean "the whole account" — a view source without conditions
/// is a deliberate, administrator-chosen read-only delegation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewScopeGroup {
    pub account_id: u64,
    /// OR within the field: at least one must match.
    pub any_participant: Vec<String>,
    /// OR within the field: at least one must match.
    pub any_recipient: Vec<String>,
}

impl ViewScopeGroup {
    pub fn unbounded(account_id: u64) -> Self {
        Self {
            account_id,
            any_participant: Vec::new(),
            any_recipient: Vec::new(),
        }
    }

    /// A source without any condition: the whole account is delegated.
    ///
    /// The distinction matters wherever a caller can only work with *account
    /// ids* — the attachment index, the aggregate endpoints, the dashboard —
    /// because those cannot evaluate a participant filter and must therefore
    /// leave a filtered source out entirely rather than expose all of it.
    pub fn is_unfiltered(&self) -> bool {
        self.any_participant.is_empty() && self.any_recipient.is_empty()
    }

    /// Whether `envelope` satisfies this group's filter.
    pub fn matches(&self, envelope: &Envelope) -> bool {
        // Empty = no restriction on that field. Non-empty = at least one of
        // the addresses must match (OR), and both fields must hold (AND).
        if !self.any_participant.is_empty()
            && !self
                .any_participant
                .iter()
                .any(|address| participant_matches(envelope, address, true))
        {
            return false;
        }
        if !self.any_recipient.is_empty()
            && !self
                .any_recipient
                .iter()
                .any(|address| participant_matches(envelope, address, false))
        {
            return false;
        }
        true
    }
}

/// Exact, case-insensitive address match against the envelope.
///
/// `include_sender` selects participant (from + recipients) vs recipient
/// (recipients only) semantics.
///
/// Case folding must stay in step with the index query
/// (`IndexManager::participant_exact_query`), which matches with a `(?i)`
/// regex: that one folds **Unicode**, so folding only ASCII here would let a
/// search return a message this check then refuses — `JÜRGEN@firma.de` is hit
/// by the filter `jürgen@firma.de` in the index but not by
/// `eq_ignore_ascii_case`. ASCII, the overwhelming case, takes the
/// allocation-free path.
pub fn participant_matches(envelope: &Envelope, address: &str, include_sender: bool) -> bool {
    let address = address.trim();
    if address.is_empty() {
        return false;
    }
    let eq = |value: &str| addresses_equal(value, address);
    let in_list = |list: &[String]| list.iter().any(|value| eq(value.as_str()));
    (include_sender && eq(envelope.from.as_str()))
        || in_list(&envelope.to)
        || in_list(&envelope.cc)
        || in_list(&envelope.bcc)
}

/// Case-insensitive address equality, folded the way the index folds it.
fn addresses_equal(left: &str, right: &str) -> bool {
    if left.is_ascii() && right.is_ascii() {
        return left.eq_ignore_ascii_case(right);
    }
    left.to_lowercase() == right.to_lowercase()
}

/// Resolve the accounts a search may read into scope groups.
///
/// * `None` — no restriction (an admin searching everything). The caller keeps
///   today's behaviour, including its ability to filter by `account_ids`.
/// * `Some(groups)` — the query must match one of these groups. A view is
///   replaced by its sources; an ordinary account stays as itself. Requested
///   accounts the caller may not read are dropped, which yields an empty result
///   rather than an error — the same outcome the old two-clause query produced.
pub fn search_scope(
    context: &ClientContext,
    requested: Option<&HashSet<u64>>,
) -> BichonResult<Option<Vec<ViewScopeGroup>>> {
    let sees_everything = context.has_permission(None, Permission::DATA_READ_ALL);

    let candidates: Vec<u64> = match (requested, sees_everything) {
        (Some(requested), _) => requested.iter().copied().collect(),
        // No explicit request: an unrestricted reader searches everything.
        (None, true) => return Ok(None),
        (None, false) => context.user.account_access_map.keys().copied().collect(),
    };

    let all = readable_accounts()?;
    Ok(Some(scope_groups_for(&all, candidates.into_iter(), |id| {
        context.has_permission(Some(id), Permission::DATA_READ)
    })))
}

/// Pure core of the scope resolution: one group per candidate account, a view
/// replaced by its sources with each source's filter.
///
/// `allowed` decides whether the caller may read an account at all; a
/// candidate it rejects contributes nothing, which yields an empty result
/// rather than an error — the same outcome the old two-clause query produced.
///
/// Pulled out as a pure function (like [`first_unmanaged_source`]) so the
/// expansion rule is testable without a session.
pub fn scope_groups_for(
    accounts: &[AccountModel],
    candidates: impl Iterator<Item = u64>,
    allowed: impl Fn(u64) -> bool,
) -> Vec<ViewScopeGroup> {
    let mut groups = Vec::new();
    for id in candidates {
        let Some(account) = accounts.iter().find(|a| a.id == id) else {
            continue;
        };
        if !allowed(id) {
            continue;
        }
        if account.is_view_account() {
            if let Some(view) = account.view.as_ref() {
                for source in &view.sources {
                    groups.push(ViewScopeGroup {
                        account_id: source.account_id,
                        any_participant: source.any_participant.clone(),
                        any_recipient: source.any_recipient.clone(),
                    });
                }
            }
        } else {
            // Ordinary accounts, and the journal account itself, are read
            // without a filter — the journal is a normal archive account for a
            // reader who holds permission on it.
            groups.push(ViewScopeGroup::unbounded(id));
        }
    }
    groups
}

/// Accounts a caller may **export** from.
///
/// `None` = unrestricted (holds `data:export:batch` globally, or is an admin).
/// `Some(ids)` = the granted accounts, **with a view replaced by its
/// unfiltered sources only**.
///
/// Why a filtered source is refused rather than exported: the export engine
/// pages the index by account id, and the dual-control gate stores those ids in
/// the approval record. The filter a view source carries has nowhere to live on
/// that path, so exporting a filtered source would hand out the *whole* account
/// — to the requester, and to the approver who signed off on a message count
/// computed with the filter applied. A caller whose only exportable grant is a
/// filtered view is refused with that reason, rather than silently producing an
/// empty artifact.
pub fn export_scope_accounts(
    context: &ClientContext,
) -> BichonResult<Option<HashSet<u64>>> {
    use crate::error::code::ErrorCode;
    use crate::raise_error;

    if context.has_permission(None, Permission::DATA_EXPORT_BATCH) {
        return Ok(None);
    }

    let all = readable_accounts()?;
    let groups = scope_groups_for(
        &all,
        context.user.account_access_map.keys().copied(),
        |id| context.has_permission(Some(id), Permission::DATA_EXPORT_BATCH),
    );
    let scoped: HashSet<u64> = unfiltered_source_accounts(&groups).into_iter().collect();

    if scoped.is_empty() {
        // "No permission" and "permission, but only through a filtered view"
        // are different answers: the second is a real request the operator can
        // act on, and the bare permission error would send them looking for a
        // role they already hold.
        let reason = if groups.is_empty() {
            "Missing permission 'data:export:batch'".to_string()
        } else {
            "a batch export cannot be scoped by a view's address filter, so it is refused \
             rather than exporting the whole source account. Export the source account \
             directly, or hold 'data:export:batch' on it."
                .to_string()
        };
        return Err(raise_error!(
            format!("Access Denied: {reason}"),
            ErrorCode::Forbidden
        ));
    }
    Ok(Some(scoped))
}

/// Accounts whose **whole** contents a caller may aggregate over.
///
/// `None` = unrestricted. `Some(ids)` = the readable accounts, with a view
/// replaced by its **unfiltered** sources only: the endpoints built on this
/// (facet counts, sender lists, attachment metadata, dashboard totals) are
/// account-level queries that cannot evaluate a source's participant filter, so
/// counting a filtered source would report messages the view must not expose.
/// A filtered source therefore contributes nothing — the same fail-closed rule
/// the attachment search uses.
pub fn aggregate_accounts(
    context: &ClientContext,
) -> BichonResult<Option<HashSet<u64>>> {
    aggregate_accounts_with(
        context,
        context.has_permission(None, Permission::DATA_READ_ALL),
        |id| context.has_permission(Some(id), Permission::DATA_READ),
    )
}

/// [`aggregate_accounts`] with caller-supplied permission rules.
///
/// `unrestricted` is the global case (`None` — every account). It is a
/// parameter because the callers do not agree on what grants it: the
/// account-level endpoints accept `data:read:all`, the dashboard and analytics
/// also accept `account:manage:all`, and analytics accepts `data:read` *or*
/// `account:manage` on a grant. What they must not each re-implement is the
/// view expansion — hence this shape.
pub fn aggregate_accounts_with(
    context: &ClientContext,
    unrestricted: bool,
    can_read: impl Fn(u64) -> bool,
) -> BichonResult<Option<HashSet<u64>>> {
    if unrestricted {
        return Ok(None);
    }
    let all = readable_accounts()?;
    Ok(Some(aggregate_account_ids(
        &all,
        context.user.account_access_map.keys().copied(),
        can_read,
    )))
}

/// Pure core of [`aggregate_accounts`]. See there for why a filtered view
/// source is left out.
pub fn aggregate_account_ids(
    accounts: &[AccountModel],
    granted: impl Iterator<Item = u64>,
    can_read: impl Fn(u64) -> bool,
) -> HashSet<u64> {
    let mut ids = HashSet::new();
    for id in granted {
        match accounts.iter().find(|a| a.id == id) {
            // A view has no documents of its own, so aggregating over its id
            // would report an empty archive; its unfiltered sources are what
            // the viewer can actually read in full.
            Some(account) if account.is_view_account() => {
                if !can_read(id) {
                    continue;
                }
                if let Some(view) = account.view.as_ref() {
                    for source in view.sources.iter().filter(|s| s.is_unfiltered()) {
                        ids.insert(source.account_id);
                    }
                }
            }
            Some(_) => {
                ids.insert(id);
            }
            // A grant whose account is gone contributes nothing.
            None => {}
        }
    }
    ids
}

/// The view through which `envelope` may be read, if any.
///
/// Used by read endpoints as the fallback when the caller does not hold
/// `data:read` on the owning account directly. The filter is checked against
/// the concrete envelope, which is the security-relevant part: a view is a
/// grant for *matching messages*, not for the account.
///
/// Returns *which* view granted it rather than a boolean, because the audit
/// trail has to name the delegation — see [`ViewAccess`] and
/// `Event::EmailViewed`. A caller that only needs the decision calls
/// [`allows`].
///
/// **Cost.** This runs once per opened message, so it walks the caller's
/// *grants* rather than the account collection: an account-scoped role can only
/// come from `account_access_map`, and a viewer typically holds one or two. A
/// global `data:read` would make every account directly readable, so this path
/// is only reached for callers who hold per-account grants anyway.
pub fn view_granting(
    context: &ClientContext,
    account_id: u64,
    envelope: &Envelope,
) -> Option<ViewAccess> {
    if !view_grants_enabled() {
        return None;
    }
    for view_id in context.user.account_access_map.keys() {
        if !context.has_permission(Some(*view_id), Permission::DATA_READ) {
            continue;
        }
        let Ok(Some(account)) = AccountModel::find(*view_id) else {
            continue;
        };
        if !account.is_view_account() {
            continue;
        }
        let Some(view) = account.view.as_ref() else {
            continue;
        };
        for source in view
            .sources
            .iter()
            .filter(|source| source.account_id == account_id)
        {
            let group = ViewScopeGroup {
                account_id,
                any_participant: source.any_participant.clone(),
                any_recipient: source.any_recipient.clone(),
            };
            if group.matches(envelope) {
                return Some(ViewAccess {
                    view_account_id: account.id,
                    view_account_name: account
                        .account_name
                        .clone()
                        .unwrap_or_else(|| account.email.clone()),
                });
            }
        }
    }
    None
}

/// Whether the message may be read through one of the caller's views.
pub fn allows(context: &ClientContext, account_id: u64, envelope: &Envelope) -> bool {
    view_granting(context, account_id, envelope).is_some()
}

/// The view a read was authorized by, as the audit trail records it.
///
/// The filter itself is deliberately *not* copied in: it can be edited after
/// the fact, and an audit record that quotes a scope the view no longer has is
/// worse than one that names the view and lets a reader check it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewAccess {
    pub view_account_id: u64,
    /// Display name of the view account, falling back to its address.
    pub view_account_name: String,
}

/// Whether view accounts still grant reads.
///
/// The edition/licence gate is checked on the **read** side too, not only where
/// a view is created or edited: a view built during a Pro trial must stop
/// granting access when the licence lapses, and a community build that has the
/// row (same data directory, or a downgrade) must not honour it. Otherwise
/// using the feature once would make it permanent.
fn view_grants_enabled() -> bool {
    crate::ext::account_roles::view_role_enabled()
}

/// Every account, minus the view accounts when view grants are switched off.
///
/// One place decides whether a view counts as readable at all, so search,
/// aggregates, export and the per-message check cannot disagree: with the gate
/// off, a view row resolves to nothing, exactly as if it had never been
/// created. Callers pass the result to the pure helpers, which skip an account
/// id they cannot find.
fn readable_accounts() -> BichonResult<Vec<AccountModel>> {
    let all = list_all_impl::<AccountModel>(DB_MANAGER.db())?;
    if view_grants_enabled() {
        return Ok(all);
    }
    Ok(all
        .into_iter()
        .filter(|account| !account.is_view_account())
        .collect())
}



/// The first source in `sources` the caller may not administer.
///
/// Pulled out as a pure function so the rule is testable without a session:
/// creating or widening a view delegates read access to the source accounts,
/// so the caller must hold `account:manage` on every account it exposes. Being
/// able to *create an account* is not enough — otherwise a user could point a
/// view at an account they cannot administer and read it through it.
pub fn first_unmanaged_source(
    sources: &[crate::account::migration::ViewSource],
    can_manage: impl Fn(u64) -> bool,
) -> Option<u64> {
    sources
        .iter()
        .map(|source| source.account_id)
        .find(|account_id| !can_manage(*account_id))
}


/// Enforce source management on **edit**, allowing an unmanageable source to
/// stay exactly as it was.
///
/// Editing a view requires management of the view account itself (checked by
/// the caller). For its sources the rule is narrower: a source the caller
/// cannot manage may be *kept unchanged* — graying it out in the UI is the
/// intended experience — but it may not be added, and its filters may not be
/// widened. Removing it is allowed: that only narrows the view.
pub fn require_view_source_management_for_update(
    context: &ClientContext,
    old_sources: &[crate::account::migration::ViewSource],
    new_sources: &[crate::account::migration::ViewSource],
) -> BichonResult<()> {
    use crate::error::code::ErrorCode;
    use crate::raise_error;

    for source in new_sources {
        let manageable = context.has_permission(Some(source.account_id), Permission::ACCOUNT_MANAGE);
        if manageable {
            continue;
        }
        if old_sources.iter().any(|old| old == source) {
            // Present before and byte-identical: the operator is not touching
            // this source, only saving the rest of the view.
            continue;
        }
        return Err(raise_error!(
            format!(
                "Access denied: changing account {} in this view requires account management permission on it",
                source.account_id
            ),
            ErrorCode::Forbidden
        ));
    }
    Ok(())
}

/// Views that name `account_id` as one of their sources.
///
/// Deleting such an account would leave those views pointing at an id that no
/// longer exists: their searches silently return less, and the row cannot be
/// repaired from the UI — the account is gone, so an operator has nothing to
/// select. The delete is refused instead, naming the views to fix first.
pub fn views_referencing(accounts: &[AccountModel], account_id: u64) -> Vec<u64> {
    accounts
        .iter()
        .filter(|account| account.is_view_account())
        .filter(|account| {
            account
                .view
                .as_ref()
                .is_some_and(|view| view.sources.iter().any(|s| s.account_id == account_id))
        })
        .map(|account| account.id)
        .collect()
}

/// Enforce [`first_unmanaged_source`] against the caller's permissions.
pub fn require_view_source_management(
    context: &ClientContext,
    sources: &[crate::account::migration::ViewSource],
) -> BichonResult<()> {
    use crate::error::code::ErrorCode;
    use crate::raise_error;

    if let Some(account_id) = first_unmanaged_source(sources, |id| {
        context.has_permission(Some(id), Permission::ACCOUNT_MANAGE)
    }) {
        return Err(raise_error!(
            format!(
                "Access denied: creating or editing a view over account {account_id} requires account management permission on that account"
            ),
            ErrorCode::Forbidden
        ));
    }
    Ok(())
}

/// The accounts in `scope` that carry no filter, in scope order.
///
/// **Only unfiltered sources qualify**, for every caller that can work with
/// account ids but not with a filter. The attachment index carries the parent
/// message's sender (`from`) but not its recipients, so a participant filter
/// cannot be evaluated on an attachment document without a join or an index
/// rebuild; the aggregate endpoints (facet counts, sender lists, dashboard
/// totals) are account-level queries for the same reason. Including a filtered
/// source would return attachments — or counts — of messages the view must not
/// expose, so it is excluded instead: through a filtered view these yield
/// nothing rather than too much.
pub fn unfiltered_source_accounts(scope: &[ViewScopeGroup]) -> Vec<u64> {
    scope
        .iter()
        .filter(|group| group.is_unfiltered())
        .map(|group| group.account_id)
        .collect()
}

/// Whether the caller may read the message: direct permission, or a view.
///
/// The predicate form of [`load_readable`], for a caller that already holds the
/// envelope. The read endpoints call `load_readable` instead — it resolves the
/// permission *and* loads the message, so the "through a view" path cannot be
/// forgotten on one endpoint.
pub fn may_read(context: &ClientContext, account_id: u64, envelope: &Envelope) -> bool {
    context.has_permission(Some(account_id), Permission::DATA_READ)
        || allows(context, account_id, envelope)
}

/// Load a message and decide whether the caller may read it.
///
/// Read endpoints call this instead of `require_permission` + a separate load,
/// so the "through a view" path cannot be forgotten on one endpoint.
///
/// **Existence is not leaked to callers without direct access.** A missing
/// message and a message outside every view's filter return the same
/// `Forbidden`; a direct reader still gets `ResourceNotFound`, because for them
/// existence is not secret.
pub fn load_readable(
    context: &ClientContext,
    account_id: u64,
    envelope_id: &str,
) -> BichonResult<Envelope> {
    load_readable_with_access(context, account_id, envelope_id).map(|(envelope, _)| envelope)
}

/// [`load_readable`], reporting **which view** authorized the read.
///
/// The endpoints that *record* the access (the audit trail) need the view, not
/// just the decision: "user U read a message in account A" cannot be audited
/// against the delegation U was given. See [`ViewAccess`].
pub fn load_readable_with_access(
    context: &ClientContext,
    account_id: u64,
    envelope_id: &str,
) -> BichonResult<(Envelope, Option<ViewAccess>)> {
    use crate::error::code::ErrorCode;
    use crate::raise_error;
    use crate::store::tantivy::envelope::ENVELOPE_MANAGER;

    let direct = context.has_permission(Some(account_id), Permission::DATA_READ);
    let envelope = ENVELOPE_MANAGER
        .get_envelope_by_id(account_id, envelope_id)?
        .map(|ea| ea.envelope);

    match envelope {
        Some(envelope) if direct => Ok((envelope, None)),
        Some(envelope) => match view_granting(context, account_id, &envelope) {
            Some(access) => Ok((envelope, Some(access))),
            None => Err(raise_error!(
                "Access denied: this message is outside the scope of any view you can read"
                    .to_string(),
                ErrorCode::Forbidden
            )),
        },
        None if direct => Err(raise_error!(
            format!("Message {envelope_id} not found in account {account_id}"),
            ErrorCode::ResourceNotFound
        )),
        None => Err(raise_error!(
            "Access denied: this message is outside the scope of any view you can read".to_string(),
            ErrorCode::Forbidden
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(from: &str, to: &[&str], cc: &[&str]) -> Envelope {
        Envelope {
            from: from.to_string(),
            to: to.iter().map(|s| s.to_string()).collect(),
            cc: cc.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    /// Matching is exact: the address must equal a header value, and case does
    /// not matter. Token overlap (`zhang` in a display name, the domain alone)
    /// must not qualify — that is the difference between this check and the
    /// full-text search filter.
    #[test]
    fn participant_matching_is_exact() {
        let env = envelope("Kunde@Example.COM", &["zhang@firma.de"], &["li@firma.de"]);

        assert!(participant_matches(&env, "zhang@firma.de", false));
        assert!(participant_matches(&env, "ZHANG@FIRMA.DE", false));
        assert!(participant_matches(&env, "kunde@example.com", true));
        assert!(participant_matches(&env, "li@firma.de", false));

        // Not participants: the bare domain, a prefix, a display name.
        assert!(!participant_matches(&env, "firma.de", true));
        assert!(!participant_matches(&env, "zhang", false));
        assert!(!participant_matches(&env, "unknown@firma.de", true));
        // A recipient-only filter must not match the sender.
        assert!(!participant_matches(&env, "kunde@example.com", false));
    }

    /// Case folding is **not** ASCII-only, because the index query it has to
    /// agree with is not: `IndexManager::participant_exact_query` matches with
    /// a `(?i)` regex, which folds Unicode. Folding only ASCII here made the
    /// search return a message the read check then refused — a hit the viewer
    /// cannot open. Pinned together with
    /// `participant_exact_query_folds_case_the_same_way` in
    /// `store::tantivy::envelope`, which runs the same pair through the index.
    #[test]
    fn participant_matching_folds_case_like_the_index() {
        let env = envelope("JÜRGEN@firma.de", &["info@MÜNCHEN.de"], &[]);

        assert!(participant_matches(&env, "jürgen@firma.de", true));
        assert!(participant_matches(&env, "info@münchen.de", false));
        assert!(participant_matches(&env, "JÜRGEN@FIRMA.DE", true));

        // Still exact: the folding must not turn into a substring match.
        assert!(!participant_matches(&env, "ürgen@firma.de", true));
        assert!(!participant_matches(&env, "münchen.de", true));
    }

    #[test]
    fn group_filters_are_anded() {
        let env = envelope("a@example.com", &["zhang@firma.de"], &["li@firma.de"]);

        let participant_only = ViewScopeGroup {
            account_id: 1,
            any_participant: vec!["zhang@firma.de".into()],
            any_recipient: Vec::new(),
        };
        assert!(participant_only.matches(&env));

        // Both conditions must hold; the sender is not a recipient.
        let contradictory = ViewScopeGroup {
            account_id: 1,
            any_participant: vec!["zhang@firma.de".into()],
            any_recipient: vec!["a@example.com".into()],
        };
        assert!(!contradictory.matches(&env));
    }

    /// Creating or widening a view requires `account:manage` on every source.
    #[test]
    fn view_sources_must_all_be_manageable() {
        let sources = vec![
            crate::account::migration::ViewSource {
                account_id: 1,
                ..Default::default()
            },
            crate::account::migration::ViewSource {
                account_id: 2,
                ..Default::default()
            },
        ];
        assert_eq!(first_unmanaged_source(&sources, |_| true), None);
        assert_eq!(first_unmanaged_source(&sources, |id| id == 1), Some(2));
        assert_eq!(first_unmanaged_source(&sources, |_| false), Some(1));
        assert_eq!(first_unmanaged_source(&[], |_| false), None);
    }

    /// Attachment search only expands sources without a participant filter:
    /// the attachment index cannot evaluate one, and guessing would leak.
    #[test]
    fn attachment_scope_keeps_only_unfiltered_sources() {
        let scope = vec![
            ViewScopeGroup::unbounded(1),
            ViewScopeGroup {
                account_id: 2,
                any_participant: vec!["a@example.com".into()],
                any_recipient: Vec::new(),
            },
            ViewScopeGroup {
                account_id: 3,
                any_participant: Vec::new(),
                any_recipient: vec!["b@example.com".into()],
            },
            ViewScopeGroup::unbounded(4),
        ];
        assert_eq!(unfiltered_source_accounts(&scope), vec![1, 4]);
    }

    fn local_account(id: u64) -> AccountModel {
        let mut account = AccountModel::new(
            1,
            crate::account::payload::AccountCreateRequest {
                email: format!("local{id}@example.com"),
                enabled: true,
                account_type: crate::account::migration::AccountType::NoSync,
                ..Default::default()
            },
        )
        .unwrap();
        account.id = id;
        account
    }

    fn view_account(id: u64, sources: Vec<crate::account::migration::ViewSource>) -> AccountModel {
        let mut account = AccountModel::new(
            1,
            crate::account::payload::AccountCreateRequest {
                email: format!("view{id}@example.com"),
                enabled: true,
                account_type: crate::account::migration::AccountType::View,
                view: Some(crate::account::migration::ViewConfig { sources }),
                ..Default::default()
            },
        )
        .unwrap();
        account.id = id;
        account
    }

    fn source(account_id: u64, participants: &[&str]) -> crate::account::migration::ViewSource {
        crate::account::migration::ViewSource {
            account_id,
            any_participant: participants.iter().map(|s| s.to_string()).collect(),
            any_recipient: Vec::new(),
        }
    }

    /// A view is replaced by its sources, filters included; an ordinary account
    /// stays itself; a candidate the caller may not read, and one whose account
    /// is gone, contribute nothing (an empty result, never an error).
    #[test]
    fn scope_expands_views_and_drops_unreadable_candidates() {
        let accounts = vec![
            local_account(1),
            local_account(2),
            view_account(9, vec![source(1, &["a@example.com"]), source(2, &[])]),
        ];

        let groups = scope_groups_for(&accounts, [9].into_iter(), |_| true);
        assert_eq!(
            groups,
            vec![
                ViewScopeGroup {
                    account_id: 1,
                    any_participant: vec!["a@example.com".into()],
                    any_recipient: Vec::new(),
                },
                ViewScopeGroup::unbounded(2),
            ]
        );

        // A direct account is read whole.
        assert_eq!(
            scope_groups_for(&accounts, [1].into_iter(), |_| true),
            vec![ViewScopeGroup::unbounded(1)]
        );

        // Not readable, and not present: both vanish rather than erroring.
        assert!(scope_groups_for(&accounts, [1].into_iter(), |id| id != 1).is_empty());
        assert!(scope_groups_for(&accounts, [404].into_iter(), |_| true).is_empty());
    }

    /// Aggregates (facet counts, senders, dashboard totals) can only be scoped
    /// by account id, so a view contributes its unfiltered sources — never a
    /// filtered one, whose messages it must not count — and only when the
    /// caller can read through the view.
    #[test]
    fn aggregate_accounts_keep_only_unfiltered_view_sources() {
        let accounts = vec![
            local_account(3),
            view_account(9, vec![source(1, &["a@example.com"]), source(2, &[])]),
        ];

        assert_eq!(
            aggregate_account_ids(&accounts, [9, 3].into_iter(), |_| true),
            HashSet::from([2, 3])
        );

        // The view itself is not readable: it contributes nothing, and the
        // direct account is unaffected.
        assert_eq!(
            aggregate_account_ids(&accounts, [9, 3].into_iter(), |id| id != 9),
            HashSet::from([3])
        );

        // A grant whose account is gone contributes nothing.
        assert_eq!(
            aggregate_account_ids(&accounts, [404].into_iter(), |_| true),
            HashSet::new()
        );
    }

    #[test]
    fn an_unbounded_group_matches_everything() {
        let env = envelope("a@example.com", &["b@example.com"], &[]);
        assert!(ViewScopeGroup::unbounded(7).matches(&env));
    }

    /// Deleting a source account is refused, naming the views that read from
    /// it: those rows would otherwise point at an id that no longer exists.
    #[test]
    fn views_referencing_finds_only_views_that_name_the_account() {
        let accounts = vec![
            local_account(1),
            local_account(2),
            local_account(4),
            view_account(9, vec![source(1, &[]), source(2, &[])]),
            view_account(10, vec![source(2, &["a@example.com"])]),
            view_account(11, vec![source(3, &[])]),
        ];

        assert_eq!(views_referencing(&accounts, 1), vec![9]);
        assert_eq!(views_referencing(&accounts, 2), vec![9, 10]);
        assert_eq!(views_referencing(&accounts, 3), vec![11]);
        // A local account is never a reference, and neither is an id no view
        // names.
        assert!(views_referencing(&accounts, 4).is_empty());
        assert!(views_referencing(&accounts, 404).is_empty());
    }
}
