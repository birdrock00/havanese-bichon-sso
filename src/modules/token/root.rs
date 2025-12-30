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

// use crate::{
//     decrypt, encrypt, generate_token,
//     modules::{
//         error::{code::ErrorCode, BichonResult},
//         settings::{dir::DATA_DIR_MANAGER, system::SystemSetting},
//     },
//     raise_error,
// };
// use std::fs::File;
// use std::io::Write;

// pub const ROOT_TOKEN: &str = "root-token";
// pub const ROOT_PASSWORD: &str = "root-password";
// pub const DEFAULT_ROOT_PASSWORD: &str = "root";
// pub const ROOT_TOKEN_FILE: &str = "root";

// async fn get_or_generate(
//     key: &str,
//     generate: impl Fn() -> String,
//     save_file_name: Option<&str>,
//     force: bool,
// ) -> BichonResult<String> {
//     if let Some(existing_value) = SystemSetting::get_existing_value(key)? {
//         if force {
//             // If force is true, write the existing value to the file
//             if let Some(filename) = save_file_name {
//                 save_to_file(&existing_value.to_string(), filename).await?;
//             }
//         }
//         Ok(existing_value)
//     } else {
//         // If no value exists, generate a new value
//         let new_value = generate();
//         SystemSetting::set_value(key, new_value.clone()).await?;

//         // Write the new value to the file, if specified
//         if let Some(filename) = save_file_name {
//             save_to_file(&new_value.to_string(), filename).await?;
//         }
//         Ok(new_value)
//     }
// }

// pub async fn ensure_root_token() -> BichonResult<()> {
//     get_or_generate(
//         ROOT_TOKEN,
//         || generate_token!(128),
//         Some(ROOT_TOKEN_FILE),
//         true,
//     )
//     .await?;
//     Ok(())
// }

// pub async fn reset_root_token() -> BichonResult<String> {
//     let new_token = generate_token!(128);
//     save_new_token(&new_token).await?;
//     save_to_file(&new_token, ROOT_TOKEN_FILE).await?;
//     Ok(new_token)
// }

// async fn save_new_token(token: &str) -> BichonResult<()> {
//     let setting = SystemSetting::new(ROOT_TOKEN.to_string(), token.to_string());
//     setting.set().await
// }

// async fn save_to_file(content: &str, filename: &str) -> BichonResult<()> {
//     let file_path = DATA_DIR_MANAGER.root_dir.join(filename);
//     let mut file = File::create(&file_path)
//         .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
//     writeln!(file, "{}", content)
//         .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
//     Ok(())
// }

// pub fn check_root_password(password: &str) -> BichonResult<String> {
//     let stored_encrypted_password = SystemSetting::get_existing_value(ROOT_PASSWORD)?;
//     let matched = match stored_encrypted_password {
//         Some(ref stored) => {
//             let decrypted = decrypt!(stored)?;
//             decrypted == password
//         }
//         None => DEFAULT_ROOT_PASSWORD == password,
//     };

//     if !matched {
//         return Err(raise_error!(
//             "Invalid password".into(),
//             ErrorCode::PermissionDenied
//         ));
//     }

//     let root_token = SystemSetting::get_existing_value(ROOT_TOKEN)?.ok_or_else(|| {
//         raise_error!(
//             "Root token not found — this should never happen".into(),
//             ErrorCode::InternalError
//         )
//     })?;

//     Ok(root_token)
// }

// pub async fn set_root_password(new_password: &str) -> BichonResult<()> {
//     let encrypted_password = encrypt!(new_password)?;
//     SystemSetting::set_value(ROOT_PASSWORD, encrypted_password).await
// }
