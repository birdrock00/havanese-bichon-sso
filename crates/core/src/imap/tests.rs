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

use mail_parser::{parsers::MessageStream, MessageParser, MimeHeaders};

use crate::{
    base64_encode_url_safe,
    {account::entity::Encryption, imap::client::Client},
};

#[tokio::test]
async fn testxx() {
    rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider())
        .unwrap();
    let client = Client::connection("imap.zoho.com".into(), &Encryption::Ssl, 993, None, false)
        .await
        .unwrap();
    let mut session = client.login("xx@zohomail.com", "xxx").await.unwrap();
    session.select("INBOX").await.unwrap();
    let result = session.uid_search("LARGER 1024").await.unwrap();
    println!("{:#?}", result);
}

#[tokio::test]
async fn test1() {
    let path = r"C:\Users\polly\Downloads\test.eml";
    let eml_data = std::fs::read(path).unwrap();
    let input = base64_encode_url_safe!(eml_data);
    let message = MessageParser::default().parse(&input).unwrap();
    let parts = message.parts;
    for part in parts {
        println!("{}", part.is_message());
        println!("{}", part.is_multipart());
    }
}

#[tokio::test]
async fn test2() {
    const MESSAGE: &str = r#"From: Art Vandelay <art@vandelay.com> (Vandelay Industries)
X-Gmail-Labels: =?UTF-8?Q?Archiv=C3=A9s,Envoy=C3=A9?=
To: "Colleagues": "James Smythe" <james@vandelay.com>; Friends:
    jane@example.com, =?UTF-8?Q?John_Sm=C3=AEth?= <john@example.com>;
Date: Sat, 20 Nov 2021 14:22:01 -0800
Subject: =?utf-8?B?SnVzdCAxNSBkYXlzIGxlZnQgdG8gdmlzaXQgTkFSTklBISDinYTvuI/wn462?=
Content-Type: multipart/mixed; boundary="festivus";

--festivus
Content-Type: text/html; charset="us-ascii"
Content-Transfer-Encoding: base64

PGh0bWw+PHA+SSB3YXMgdGhpbmtpbmcgYWJvdXQgcXVpdHRpbmcgdGhlICZsZHF1bztle
HBvcnRpbmcmcmRxdW87IHRvIGZvY3VzIGp1c3Qgb24gdGhlICZsZHF1bztpbXBvcnRpbm
cmcmRxdW87LDwvcD48cD5idXQgdGhlbiBJIHRob3VnaHQsIHdoeSBub3QgZG8gYm90aD8
gJiN4MjYzQTs8L3A+PC9odG1sPg==
--festivus
Content-Type: message/rfc822

From: "Cosmo Kramer" <kramer@kramerica.com>
Subject: Exporting my book about coffee tables
Content-Type: multipart/mixed; boundary="giddyup";

--giddyup
Content-Type: text/plain; charset="utf-16"
Content-Transfer-Encoding: quoted-printable

=FF=FE=0C!5=D8"=DD5=D8)=DD5=D8-=DD =005=D8*=DD5=D8"=DD =005=D8"=
=DD5=D85=DD5=D8-=DD5=D8,=DD5=D8/=DD5=D81=DD =005=D8*=DD5=D86=DD =
=005=D8=1F=DD5=D8,=DD5=D8,=DD5=D8(=DD =005=D8-=DD5=D8)=DD5=D8"=
=DD5=D8=1E=DD5=D80=DD5=D8"=DD!=00
--giddyup
Content-Type: image/gif; name*1="about "; name*0="Book ";
              name*2*=utf-8''%e2%98%95 tables.gif
Content-Transfer-Encoding: Base64
Content-Disposition: attachment

R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7
--giddyup--
--festivus--
"#;

    let message = MessageParser::default().parse(MESSAGE).unwrap();
    let header = message.header_raw("X-Gmail-Labels").unwrap().as_bytes();

    let data = MessageStream::new(header)
        .parse_unstructured()
        .unwrap_text()
        .to_string();

    println!("{}", data);
    // RFC2047 support for encoded text in message readers
    //println!("{}", message.subject().unwrap());
}

#[tokio::test]
async fn test_bulk_attachment_stripping_blake3() {
    let path = r"C:\Users\polly\Downloads\test666.eml";
    let input = std::fs::read(path).expect("Failed to read EML file");

    // 1. Initial Parse
    let message = MessageParser::default()
        .parse(&input)
        .expect("Failed to parse EML");

    // 2. Collect and cast types explicitly
    // We map the u32 offsets to usize here to satisfy the Vec<(usize, usize, ...)> requirement
    let mut attachments: Vec<(usize, usize, Vec<u8>)> = message
        .attachments()
        .map(|att| {
            (
                att.raw_body_offset() as usize,
                att.raw_end_offset() as usize,
                att.contents().to_vec(),
            )
        })
        .collect();

    // 3. Sort by offset descending (BACK TO FRONT)
    // This ensures that modifying the file length doesn't invalidate earlier offsets
    attachments.sort_by(|a, b| b.0.cmp(&a.0));

    let mut modified_eml = input.clone();

    println!(
        "Processing {} attachments in reverse order...",
        attachments.len()
    );

    for (start, end, raw_content) in attachments {
        // Calculate BLAKE3 Hash
        let hash = blake3::hash(&raw_content).to_hex().to_string();
        let placeholder = format!("STRIPPED_BLAKE3:{}", hash);
        let placeholder_bytes = placeholder.as_bytes();

        // Perform the byte surgery
        let mut new_buffer =
            Vec::with_capacity(modified_eml.len() - (end - start) + placeholder_bytes.len());
        new_buffer.extend_from_slice(&modified_eml[..start]);
        new_buffer.extend_from_slice(placeholder_bytes);
        new_buffer.extend_from_slice(&modified_eml[end..]);

        modified_eml = new_buffer;
        println!(
            "Stripped attachment at offset {}. New hash: {}",
            start, hash
        );
    }

    std::fs::write("test.eml", &modified_eml).unwrap();

    // 4. Final Verification
    let final_message = MessageParser::default().parse(&modified_eml).unwrap();

    println!("\n--- Verification Report ---");
    for (i, att) in final_message.attachments().enumerate() {
        let content = String::from_utf8_lossy(att.contents());
        println!(
            "Part [{}]: {}, Content: {}",
            i,
            att.attachment_name().unwrap_or("unknown"),
            content
        );
        assert!(content.contains("STRIPPED_BLAKE3:"));
    }

    println!("✅ All attachments replaced successfully from back to front.");
}

#[tokio::test]
async fn test_667() {
    let path = r"C:\Users\polly\Downloads\test777.eml";
    let input = std::fs::read(path).expect("Failed to read EML file");

    let message = MessageParser::default()
        .parse(&input)
        .expect("Failed to parse EML");

    for att in message.attachments() {
        println!("name: {:#?}", att.attachment_name());
        println!("content_type: {:#?}", att.content_type());
        println!("is_message: {:#?}", att.is_message());
        println!("content_disposition: {:#?}", att.content_disposition());
        println!(
            "content_transfer_encoding: {:#?}",
            att.content_transfer_encoding()
        );
        println!("content_id: {:#?}", att.content_id());
    }
}
