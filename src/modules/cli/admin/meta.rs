use std::{path::Path, rc::Rc};

use native_db::{Builder, Database};

use crate::{
    modules::{
        database::META_MODELS,
        error::{code::ErrorCode, BichonResult},
        token::{AccessTokenModel, AccessTokenModelKey, TokenType},
        users::{UserModel, DEFAULT_ADMIN_USER_ID},
        utils::encrypt::internal_encrypt_string,
    },
    raise_error,
};
use itertools::Itertools;

pub fn init_meta_database(path: impl AsRef<Path>) -> BichonResult<Rc<Database<'static>>> {
    let database = Builder::new()
        .set_cache_size(134217728)
        .create(&META_MODELS, path)
        .map_err(|e| {
            raise_error!(
                format!("Failed to open database: {:?}", e),
                ErrorCode::InternalError
            )
        })?;

    Ok(Rc::new(database))
}

pub fn find_admin(database: &Rc<Database<'static>>) -> BichonResult<Option<UserModel>> {
    let r_transaction = database
        .r_transaction()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    let entity: Option<UserModel> = r_transaction
        .get()
        .primary(DEFAULT_ADMIN_USER_ID)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    return Ok(entity);
}

pub fn update_admin_password(
    database: &Rc<Database<'static>>,
    password: String,
    encrypt_key: &str,
) -> BichonResult<()> {
    let rw_transaction = database
        .rw_transaction()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    let entity: UserModel = rw_transaction
        .get()
        .primary(DEFAULT_ADMIN_USER_ID)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
        .ok_or_else(|| raise_error!("admin is not found".into(), ErrorCode::InternalError))?;

    let mut updated = entity.clone();
    updated.password = Some(
        internal_encrypt_string(encrypt_key, &password)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?,
    );

    rw_transaction
        .update(entity, updated)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    rw_transaction
        .commit()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    return Ok(());
}

pub fn reset_webui_token(database: &Rc<Database<'static>>) -> BichonResult<()> {
    let rw_transaction = database
        .rw_transaction()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    let tokens: Vec<AccessTokenModel> = rw_transaction
        .scan()
        .secondary(AccessTokenModelKey::user_id)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
        .start_with(DEFAULT_ADMIN_USER_ID)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?
        .try_collect()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    let webui_token = tokens
        .into_iter()
        .find(|t| t.token_type == TokenType::WebUI);

    let new_token = AccessTokenModel::new_webui_token(DEFAULT_ADMIN_USER_ID);
    match webui_token {
        Some(current) => {
            rw_transaction
                .remove(current)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

            rw_transaction
                .insert(new_token)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        }
        None => {
            rw_transaction
                .insert(new_token)
                .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        }
    }

    rw_transaction
        .commit()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
    Ok(())
}
