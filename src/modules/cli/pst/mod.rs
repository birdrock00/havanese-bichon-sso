use chrono::{DateTime, TimeZone, Utc};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Input;
use mail_send::mail_builder::headers::text::Text;
use mail_send::mail_builder::MessageBuilder;
use outlook_pst::ltp::prop_context::PropertyValue;

use crate::base64_encode_url_safe;
use crate::modules::cli::pst::encoding::decode_subject;
use crate::modules::cli::sender::send_batch_request;
use crate::modules::cli::BichonCtlConfig;
use dialoguer::Confirm;
use outlook_pst::messaging::attachment::AttachmentProperties;
use outlook_pst::messaging::folder::Folder;
use outlook_pst::messaging::message::{Message, MessageProperties};
use outlook_pst::ndb::node_id::NodeId;
use reqwest::Client;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;

mod encoding;

#[derive(Debug, Default)]
pub struct EmailMetadata {
    pub message_id: Option<String>,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub html: Option<String>,
    pub text: Option<String>,
    pub in_reply_to: Option<String>,
}

#[derive(Debug, Default)]
pub struct EmailAttachment {
    pub name: Option<String>,
    pub mime_type: Option<String>,
    pub data: Option<Vec<u8>>,
}

pub async fn handle_pst_import(config: &BichonCtlConfig, account_id: u64, theme: &ColorfulTheme) {
    let path_str: String = Input::with_theme(theme)
        .with_prompt("Enter the path to your SINGLE .pst file")
        .validate_with(|input: &String| {
            let p = std::path::Path::new(input);
            if !p.exists() {
                return Err("The specified path does not exist.");
            }

            if !p.is_file() {
                return Err("PST mode requires a SINGLE file, not a directory.");
            }
            let is_pst = p
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("pst"))
                .unwrap_or(false);

            if !is_pst {
                return Err("The selected file must have a .pst extension.");
            }

            Ok(())
        })
        .interact_text()
        .unwrap();

    let pst_path = std::path::PathBuf::from(path_str);

    println!(
        "\n{} Ready to process PST file: {}",
        console::style("✔").green(),
        console::style(pst_path.display()).cyan()
    );

    if let Ok(meta) = std::fs::metadata(&pst_path) {
        let size_mb = meta.len() as f64 / 1024.0 / 1024.0;
        println!(
            "{}",
            console::style(format!("PST File Size: {:.1} MB", size_mb)).dim()
        );
    }

    if Confirm::with_theme(theme)
        .with_prompt("Start importing emails from this PST?")
        .default(true)
        .interact()
        .unwrap()
    {
        parse_pst(pst_path, config, account_id).await;
    } else {
        println!("{}", console::style("Operation cancelled by user.").red());
    }
}

async fn parse_pst(pst_path: PathBuf, config: &BichonCtlConfig, account_id: u64) {
    let client = Client::new();

    let pst_store = match outlook_pst::open_store(&pst_path) {
        Ok(store) => store,
        Err(e) => {
            println!(
                "{} Failed to open PST file: {}",
                console::style("✘").red(),
                console::style(format!("{:#?}", e)).dim()
            );
            return;
        }
    };

    let ipm_sub_tree = match pst_store.properties().ipm_sub_tree_entry_id() {
        Ok(id) => id,
        Err(e) => {
            println!(
                "{} Could not find IPM_SUBTREE (Mailbox Root): {}",
                console::style("✘").red(),
                console::style(format!("{:#?}", e)).dim()
            );
            return;
        }
    };

    let ipm_subtree_folder = match pst_store.open_folder(&ipm_sub_tree) {
        Ok(folder) => folder,
        Err(e) => {
            println!(
                "{} Failed to open the root mailbox folder: {}",
                console::style("✘").red(),
                console::style(format!("{:#?}", e)).dim()
            );
            return;
        }
    };

    process_folder_recursively(&client, &ipm_subtree_folder, "", config, account_id).await;
}

fn process_folder_recursively<'a>(
    client: &'a Client,
    folder: &'a Rc<dyn Folder>,
    parent_path: &'a str,
    config: &'a BichonCtlConfig,
    account_id: u64,
) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
    Box::pin(async move {
        let folder_name = folder
            .properties()
            .display_name()
            .unwrap_or_else(|_| "Unknown".to_string());

        let current_path = if parent_path.is_empty() {
            folder_name
        } else {
            format!("{}/{}", parent_path, folder_name)
        };

        println!(
            "{} {}",
            console::style("📁 Folder:").dim(),
            console::style(&current_path).cyan()
        );

        let mut emls_batch = Vec::new();

        if let Some(contents_table) = folder.contents_table() {
            for row in contents_table.rows_matrix() {
                let store = folder.store().clone();

                let entry_id = match store
                    .properties()
                    .make_entry_id(NodeId::from(u32::from(row.id())))
                {
                    Ok(id) => id,
                    Err(e) => {
                        eprintln!(
                            "  {} Skip row {}: {:?}",
                            console::style("⚠").yellow(),
                            row.unique(),
                            e
                        );
                        continue;
                    }
                };

                match store.open_message(&entry_id, None) {
                    Ok(message) => match build_eml_base64(message) {
                        Some(base64_eml) => emls_batch.push(base64_eml),
                        None => {}
                    },
                    Err(e) => eprintln!("  {} Open error: {:?}", console::style("⚠").yellow(), e),
                }

                if emls_batch.len() >= 50 {
                    let batch = emls_batch.clone();
                    emls_batch.clear();
                    send_to_bichon(client, config, account_id, &current_path, batch).await;
                }
            }
        }

        if !emls_batch.is_empty() {
            send_to_bichon(client, config, account_id, &current_path, emls_batch).await;
        }

        if let Some(hierarchy_table) = folder.hierarchy_table() {
            for row in hierarchy_table.rows_matrix() {
                let node = NodeId::from(u32::from(row.id()));
                if let Ok(entry_id) = folder.store().properties().make_entry_id(node) {
                    if let Ok(sub_folder) = folder.store().open_folder(&entry_id) {
                        process_folder_recursively(
                            client,
                            &sub_folder,
                            &current_path,
                            config,
                            account_id,
                        )
                        .await;
                    }
                }
            }
        }
    })
}

fn build_eml_base64(message: Rc<dyn Message>) -> Option<String> {
    let properties = message.properties();

    let mut builder = MessageBuilder::new();
    if let Some(sub) = extract_subject(properties) {
        builder = builder.subject(sub);
    }
    if let Some(mid) = extract_string_property(properties, 0x1035) {
        builder = builder.message_id(mid);
    }
    if let Some(irt) = extract_string_property(properties, 0x1042) {
        builder = builder.in_reply_to(irt);
    }

    if let Some(refs) = extract_string_property(properties, 0x1039) {
        builder = builder.header("References", Text::new(refs));
    }

    if let Some(cid_val) = properties.get(0x3013) {
        if let PropertyValue::Binary(bin) = cid_val {
            builder = builder.header(
                "X-Bichon-Conversation-ID",
                Text::new(hex::encode(bin.buffer())),
            );
        }
    }

    let from = extract_string_property(properties, 0x5D01)
        .or_else(|| extract_string_property(properties, 0x5D02));

    if let Some(f) = from {
        builder = builder.from(f);
    }

    if let Some(filetime) = extract_i64_property(properties, &[0x0039, 0x0E06]) {
        let dt = filetime_to_datetime(filetime).timestamp();
        builder = builder.date(dt);
    }

    let (to, cc, bcc) = extract_recipients_list(&message);
    if !to.is_empty() {
        builder = builder.to(to.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    }
    if !cc.is_empty() {
        builder = builder.cc(cc.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    }
    if !bcc.is_empty() {
        builder = builder.bcc(bcc.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    }

    if let Some(html) = extract_html(properties) {
        builder = builder.html_body(html);
    }

    if let Some(text) = extract_text(properties) {
        builder = builder.text_body(text);
    }

    if let Some(attachment_table) = message.attachment_table() {
        for row in attachment_table.rows_matrix() {
            let node_id = NodeId::from(u32::from(row.id()));
            if let Ok(attachment) = message.clone().read_attachment(node_id, None) {
                let att_props = attachment.properties();
                let name = extract_attachment_string_property(att_props, 0x3707);
                let mime = extract_attachment_string_property(att_props, 0x370E)
                    .unwrap_or_else(|| "application/octet-stream".into());
                let cid = extract_attachment_string_property(att_props, 0x3712);
                let is_inline = att_props
                    .get(0x3714)
                    .and_then(|val| {
                        if let PropertyValue::Integer32(f) = val {
                            Some(f)
                        } else {
                            None
                        }
                    })
                    .map(|flag| (flag & 0x4) != 0)
                    .unwrap_or(false);

                if let Some(PropertyValue::Binary(bin)) = att_props.get(0x3701) {
                    let data = bin.buffer().to_vec();
                    let file_name = name.unwrap_or_else(|| "unnamed_attachment".to_string());

                    if is_inline && cid.is_some() {
                        let content_id = cid.unwrap();
                        builder = builder.inline(mime, content_id, data);
                    } else {
                        builder = builder.attachment(mime, file_name, data);
                    }
                }
            }
        }
    }

    match builder.write_to_vec() {
        Ok(eml_vec) => Some(base64_encode_url_safe!(eml_vec)),
        Err(e) => {
            eprintln!("Failed to generate EML: {:?}", e);
            None
        }
    }
}

fn filetime_to_datetime(filetime: i64) -> DateTime<Utc> {
    let unix_secs = (filetime / 10_000_000) - 11_644_473_600;
    let nsecs = (filetime % 10_000_000) * 100;
    Utc.timestamp_opt(unix_secs, nsecs as u32).unwrap()
}

fn extract_recipients_list(message: &Rc<dyn Message>) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut to = Vec::new();
    let mut cc = Vec::new();
    let mut bcc = Vec::new();

    let recipient_table = message.recipient_table();
    let context = recipient_table.context();

    for row in recipient_table.rows_matrix() {
        if let Ok(cols) = row.columns(context) {
            let mut r_type = 0;
            let mut email = String::new();

            for (col, val) in context.columns().iter().zip(cols) {
                let prop_val = val
                    .as_ref()
                    .and_then(|v| recipient_table.read_column(v, col.prop_type()).ok());
                match col.prop_id() {
                    0x0C15 => {
                        if let Some(PropertyValue::Integer32(t)) = prop_val {
                            r_type = t;
                        }
                    }
                    0x39FE | 0x3003 => {
                        if let Some(s) = prop_val.and_then(|v| extract_string(&v)) {
                            email = s;
                        }
                    }
                    _ => {}
                }
            }

            if !email.is_empty() {
                match r_type {
                    1 => to.push(email),
                    2 => cc.push(email),
                    3 => bcc.push(email),
                    _ => {}
                }
            }
        }
    }
    (to, cc, bcc)
}

async fn send_to_bichon(
    client: &Client,
    config: &BichonCtlConfig,
    account_id: u64,
    folder_path: &str,
    emls: Vec<String>,
) {
    send_batch_request(client, config, account_id, folder_path, emls).await;
}

fn extract_subject(props: &MessageProperties) -> Option<String> {
    props.get(0x0037).and_then(|val| decode_subject(val))
}

fn extract_string_property(properties: &MessageProperties, prop_id: u16) -> Option<String> {
    properties
        .get(prop_id)
        .and_then(|value| extract_string(value))
}

fn extract_attachment_string_property(
    properties: &AttachmentProperties,
    prop_id: u16,
) -> Option<String> {
    properties
        .get(prop_id)
        .and_then(|value| extract_string(value))
}

fn extract_string(value: &PropertyValue) -> Option<String> {
    match value {
        PropertyValue::String8(value) => Some(value.to_string()),
        PropertyValue::Unicode(value) => Some(value.to_string()),
        _ => None,
    }
}

fn extract_text(properties: &MessageProperties) -> Option<String> {
    properties.get(0x1000).and_then(extract_string).or_else(|| {
        properties.get(0x1009).and_then(|value| match value {
            PropertyValue::Binary(value) => encoding::decode_rtf_compressed(value.buffer()),
            _ => None,
        })
    })
}

fn extract_html(properties: &MessageProperties) -> Option<String> {
    properties.get(0x1013).and_then(|value| match value {
        PropertyValue::Binary(value) => {
            let code_page = properties
                .get(0x3FDE)
                .and_then(|v| {
                    if let PropertyValue::Integer32(cpid) = v {
                        Some(*cpid as u16)
                    } else {
                        None
                    }
                })
                .unwrap_or(65001);
            encoding::decode_html_body(value.buffer(), code_page)
        }
        PropertyValue::String8(value) => Some(value.to_string()),
        PropertyValue::Unicode(value) => Some(value.to_string()),
        _ => None,
    })
}

fn extract_i64_property(properties: &MessageProperties, prop_ids: &[u16]) -> Option<i64> {
    for &prop_id in prop_ids {
        if let Some(PropertyValue::Time(value)) = properties.get(prop_id) {
            return Some(*value);
        }
    }
    None
}
