use crate::{
    encode_mailbox_name,
    {
        account::migration::{AccountModel, AccountType},
        envelope::extractor::reattach_eml_content,
        error::{code::ErrorCode, BichonResult},
        imap::executor::ImapExecutor,
    },
    raise_error,
};
//use poem_openapi::Object;
use serde::{Deserialize, Serialize};

const MAX_RESTORE_COUNT: usize = 100;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct RestoreMessagesRequest {
    /// envelope IDs to restore (max 100)
    pub envelope_ids: Vec<String>,
}

pub async fn restore_emails(account_id: u64, envelope_ids: Vec<String>) -> BichonResult<()> {
    if envelope_ids.len() > MAX_RESTORE_COUNT {
        return Err(raise_error!(
            format!(
                "Too many messages to restore: {} (max {})",
                envelope_ids.len(),
                MAX_RESTORE_COUNT
            ),
            ErrorCode::InvalidParameter
        ));
    }

    let account = AccountModel::check_account_exists(account_id).await?;
    if !matches!(account.account_type, AccountType::IMAP) {
        return Err(raise_error!(
            "Account type is not IMAP".into(),
            ErrorCode::Incompatible
        ));
    }

    let mut failed = Vec::new();
    let mut session = ImapExecutor::create_connection(account_id).await?;
    for envelope_id in envelope_ids {
        let result: BichonResult<()> = async {
            let (envelope, eml) = reattach_eml_content(account_id, envelope_id.clone()).await?;
            if let Some(mailbox_name) = envelope.mailbox_name {
                ImapExecutor::append(
                    &mut session,
                    encode_mailbox_name!(&mailbox_name),
                    None,
                    None,
                    &eml,
                )
                .await?;
            }

            Ok(())
        }
        .await;

        if let Err(err) = result {
            tracing::warn!(
                account_id = account_id,
                message_id = &envelope_id,
                error = ?err,
                "Failed to restore email"
            );
            failed.push(envelope_id);
        }
    }

    if !failed.is_empty() {
        tracing::info!(
            account_id = account_id,
            failed_count = failed.len(),
            failed_message_ids = ?failed,
            "Restore emails finished with partial failures"
        );
    }

    session.logout().await.ok();

    Ok(())
}
