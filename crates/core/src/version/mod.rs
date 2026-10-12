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

//use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::{bichon_version, error::BichonResult, users::UserModel};

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ReleaseNotification {
    /// Details of the latest release, if available. `None` if no release data is available.
    pub latest: Option<Release>,
    /// Indicates whether the latest release is newer than the current RustMailer service version.
    pub is_newer: bool,
    /// Optional error message if the release check failed (e.g., network or API issues).
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct Release {
    /// The tag name of the release (e.g., "v1.2.3").
    pub tag_name: String,
    /// The publication date of the release in string format (e.g., ISO 8601 format).
    pub published_at: String,
    /// The body of the release notes, typically in Markdown format.
    pub body: String,
    /// The URL to the release's web page (e.g., GitHub release page).
    pub html_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct Notifications {
    pub release: ReleaseNotification,
    /// Operational alerts derived from local state (backup warnings, and
    /// future license/storage issues), shown in the WebUI notifications bell
    /// alongside the release check. Empty when nothing needs attention.
    #[serde(default)]
    pub alerts: Vec<SystemAlert>,
}

/// One alert shown to the admin in the notifications bell.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct SystemAlert {
    /// Discriminator the UI uses to pick a label/icon (`"backup"`, ...).
    pub kind: String,
    /// `"warning"` (non-fatal, action recommended) or `"error"`.
    pub severity: String,
    /// Short one-line title.
    pub title: String,
    /// Longer human-readable detail.
    pub message: String,
}

pub async fn fetch_notifications(is_admin: bool) -> BichonResult<Notifications> {
    let current_version = bichon_version!();
    let release = check_new_release("rustmailer", "bichon", current_version).await;
    Ok(Notifications {
        release,
        alerts: system_alerts(is_admin).await,
    })
}

/// Alerts derived from local state. Currently the non-fatal warnings reported
/// by the most recent backup run (editions flag e.g. a database that has grown
/// large enough to slow backups), plus the admin-only reminder that the
/// default administrator password is still in use.
async fn system_alerts(is_admin: bool) -> Vec<SystemAlert> {
    let mut alerts = warnings_to_alerts(crate::backup::manager::BACKUP_MANAGER
        .state()
        .last_warnings);

    if is_admin {
        if let Some(alert) = default_password_alert().await {
            alerts.insert(0, alert);
        }
    }
    alerts
}

/// Reminder shown in the notifications bell while the default administrator
/// account still accepts the well-known initial password. Admin-only: it
/// must not tell other users the admin password is the public default.
async fn default_password_alert() -> Option<SystemAlert> {
    // argon2 verify is blocking work; keep it off the async executor.
    let active = tokio::task::spawn_blocking(UserModel::default_admin_password_active)
        .await
        .ok()?;
    default_password_alert_for(active)
}

/// Pure half of [`default_password_alert`], split out so the alert shape is
/// testable without a database.
fn default_password_alert_for(active: bool) -> Option<SystemAlert> {
    if !active {
        return None;
    }
    Some(SystemAlert {
        kind: "security".to_string(),
        severity: "warning".to_string(),
        title: "Default administrator password in use".to_string(),
        message: "The administrator account still accepts the initial password \
                  (admin / admin@bichon). Open Settings -> Profile and set a new \
                  password now."
            .to_string(),
    })
}

/// Map backup-run warnings to notification alerts. Pure so it is testable
/// without touching the process-global manager state.
fn warnings_to_alerts(warnings: Vec<String>) -> Vec<SystemAlert> {
    warnings
        .into_iter()
        .map(|w| SystemAlert {
            kind: "backup".to_string(),
            severity: "warning".to_string(),
            title: "Backup warning".to_string(),
            message: w,
        })
        .collect()
}

async fn check_new_release(owner: &str, repo: &str, current_version: &str) -> ReleaseNotification {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    );

    let client = reqwest::Client::new();
    let response = match client
        .get(&url)
        .header("User-Agent", "reqwest")
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return ReleaseNotification {
                latest: None,
                is_newer: false,
                error_message: Some(format!("Failed to send request: {}", e)),
            }
        }
    };

    if response.status().is_success() {
        match response.json::<Release>().await {
            Ok(release) => {
                let is_newer = release.tag_name != current_version;
                ReleaseNotification {
                    latest: Some(release),
                    is_newer,
                    error_message: None,
                }
            }
            Err(e) => ReleaseNotification {
                latest: None,
                is_newer: false,
                error_message: Some(format!("Failed to parse response: {}", e)),
            },
        }
    } else {
        ReleaseNotification {
            latest: None,
            is_newer: false,
            error_message: Some(format!("Request failed with status: {}", response.status())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{default_password_alert_for, warnings_to_alerts};

    #[test]
    fn backup_warnings_map_to_backup_alerts() {
        let alerts = warnings_to_alerts(vec!["audit database is 2048 MB".to_string()]);
        assert_eq!(alerts.len(), 1);
        let a = &alerts[0];
        assert_eq!(a.kind, "backup");
        assert_eq!(a.severity, "warning");
        assert!(a.message.contains("2048 MB"), "message must carry the detail");
    }

    #[test]
    fn empty_warnings_produce_no_alerts() {
        assert!(warnings_to_alerts(vec![]).is_empty());
    }

    #[test]
    fn default_password_alert_only_exists_while_active() {
        assert!(default_password_alert_for(true).is_some());
        assert!(default_password_alert_for(false).is_none());
    }

    #[test]
    fn default_password_alert_tells_the_admin_what_to_do() {
        let alert = default_password_alert_for(true).unwrap();
        assert_eq!(alert.kind, "security");
        assert_eq!(alert.severity, "warning");
        assert!(alert.message.contains("admin@bichon"));
        assert!(alert.message.contains("Profile"), "must say where to change it");
    }

    #[tokio::test]
    async fn test() {
        let current_version = crate::bichon_version!();
        println!("current_version: {}", crate::bichon_version!());
        let result =
            crate::version::check_new_release("rustmailer", "persistent-scheduler", current_version)
                .await;

        println!("{:#?}", result);
    }
}
