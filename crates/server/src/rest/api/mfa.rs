// TOTP two-factor authentication — self-service management API (community).

use crate::common::auth::WrappedContext;
use crate::rest::api::ApiTags;
use crate::rest::ApiResult;
use bichon_core::error::code::ErrorCode;
use bichon_core::ext::event_bus::{emit, Event};
use bichon_core::raise_error;
use bichon_core::users::permissions::Permission;
use bichon_core::users::UserModel;
use bichon_core::utils::totp;
use poem::web::Path;
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;
use serde::{Deserialize, Serialize};

pub struct MfaApi;

#[derive(Clone, Debug, Serialize, Deserialize, poem_openapi::Object)]
pub struct MfaStatusResp {
    pub enabled: bool,
    pub has_secret: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, poem_openapi::Object)]
pub struct MfaEnrollResp {
    pub secret: String,
    pub otpauth_uri: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, poem_openapi::Object)]
pub struct MfaCodePayload {
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, poem_openapi::Object)]
pub struct MfaConfirmResp {
    pub recovery_codes: Vec<String>,
}

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::Mfa")]
impl MfaApi {
    /// Current two-factor authentication status for the calling user.
    #[oai(path = "/mfa/status", method = "get", operation_id = "mfa_status")]
    async fn status(&self, context: WrappedContext) -> ApiResult<Json<MfaStatusResp>> {
        Ok(Json(MfaStatusResp {
            enabled: context.user.totp_enabled,
            has_secret: context.user.totp_secret.is_some(),
        }))
    }

    /// Start enrolling TOTP: generates a fresh secret and otpauth URI.
    /// TOTP is only activated once `/mfa/confirm` succeeds.
    #[oai(path = "/mfa/enroll", method = "post", operation_id = "mfa_enroll")]
    async fn enroll(&self, context: WrappedContext) -> ApiResult<Json<MfaEnrollResp>> {
        let secret = totp::generate_secret();
        context.user.set_totp_secret(&secret)?;
        let account = if context.user.email.is_empty() {
            context.user.username.clone()
        } else {
            context.user.email.clone()
        };
        Ok(Json(MfaEnrollResp {
            otpauth_uri: totp::otpauth_uri("Bichon", &account, &secret),
            secret,
        }))
    }

    /// Confirm enrollment by submitting the current 6-digit code from the app.
    /// On success enables TOTP and returns one-time recovery codes.
    #[oai(path = "/mfa/confirm", method = "post", operation_id = "mfa_confirm")]
    async fn confirm(
        &self,
        context: WrappedContext,
        payload: Json<MfaCodePayload>,
    ) -> ApiResult<Json<MfaConfirmResp>> {
        let code = payload.0.code.trim().to_string();
        if !context.user.verify_totp_code(&code, 1)? {
            return Err(raise_error!(
                "Invalid verification code. Make sure your authenticator app shows the same code."
                    .into(),
                ErrorCode::InvalidParameter
            )
            .into());
        }
        let recovery_codes = context.user.enable_totp_with_recovery_codes()?;
        emit(Event::MfaEnabled {
            user: context.user.username.clone(),
            ip: context.ip_addr,
        });
        Ok(Json(MfaConfirmResp { recovery_codes }))
    }

    /// Disable TOTP. Requires the current code or a one-time recovery code.
    #[oai(path = "/mfa/disable", method = "post", operation_id = "mfa_disable")]
    async fn disable(
        &self,
        context: WrappedContext,
        payload: Json<MfaCodePayload>,
    ) -> ApiResult<()> {
        let code = payload.0.code.trim().to_string();
        let ok = context.user.verify_totp_code(&code, 1)?
            || context.user.verify_recovery_code(&code)?;
        if !ok {
            return Err(raise_error!(
                "Invalid verification code.".into(),
                ErrorCode::InvalidParameter
            )
            .into());
        }
        context.user.disable_totp()?;
        emit(Event::MfaDisabled {
            user: context.user.username.clone(),
            ip: context.ip_addr,
        });
        Ok(())
    }

    /// Administrator forcibly resets another user's TOTP two-factor
    /// authentication (e.g. the user lost access to their authenticator or
    /// recovery codes). The affected user must re-enroll to use MFA again.
    #[oai(path = "/mfa/admin-reset/:user_id", method = "post", operation_id = "mfa_admin_reset")]
    async fn admin_reset(&self, context: WrappedContext, user_id: Path<u64>) -> ApiResult<()> {
        context.require_permission(None, Permission::USER_MANAGE)?;
        let target = match UserModel::find(*user_id)? {
            Some(user) => user,
            None => {
                return Err(raise_error!(
                    "User not found.".into(),
                    ErrorCode::InvalidParameter
                )
                .into())
            }
        };
        if target.id == context.user.id {
            return Err(raise_error!(
                "Cannot reset your own two-factor authentication. Use the self-service disable flow instead."
                    .into(),
                ErrorCode::InvalidParameter
            )
            .into());
        }
        target.disable_totp()?;
        emit(Event::MfaResetByAdmin {
            admin: context.user.username.clone(),
            target_user: target.username.clone(),
            ip: context.ip_addr,
        });
        Ok(())
    }
}
