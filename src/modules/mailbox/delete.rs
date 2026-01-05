use crate::modules::{
    cache::imap::mailbox::MailBox,
    error::BichonResult,
    indexer::manager::{EML_INDEX_MANAGER, ENVELOPE_INDEX_MANAGER},
};

pub async fn delete_mailbox_impl(account_id: u64, mailbox_id: u64) -> BichonResult<()> {
    let mailbox = MailBox::get(mailbox_id).await?;

    let name = mailbox.name;
    let delimiter = mailbox.delimiter.unwrap_or("/".to_owned());
    let all_mailboxes = MailBox::list_all(account_id).await?;

    let prefix = format!("{}{}", name, delimiter);
    let ids_to_delete: Vec<u64> = all_mailboxes
        .into_iter()
        .filter(|m| m.id == mailbox_id || m.name.starts_with(&prefix))
        .map(|m| m.id)
        .collect();

    if ids_to_delete.is_empty() {
        return Ok(());
    }

    for id in &ids_to_delete {
        MailBox::delete(*id).await?;
    }

    ENVELOPE_INDEX_MANAGER
        .delete_mailbox_envelopes(account_id, ids_to_delete.clone())
        .await?;

    EML_INDEX_MANAGER
        .delete_mailbox_envelopes(account_id, ids_to_delete)
        .await?;

    Ok(())
}
