//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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

use mail_parser::MessageParser;

use crate::{
    base64_encode_url_safe,
    modules::{account::entity::Encryption, imap::client::Client},
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
