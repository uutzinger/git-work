// SPDX-License-Identifier: GPL-3.0-only

use crate::fl;

use octocrab::models::activity::Notification;

pub fn get_notification_url(notification: &Notification) -> Option<String> {
    if let Some(url) = &notification.subject.url {
        let url = url
            .to_string()
            .replace("api.github.com/repos", "github.com");

        // Convert API URL to web URL
        if url.contains("/pulls/") {
            Some(
                url.replace("api.github.com/repos", "github.com")
                    .replace("/pulls/", "/pull/"),
            )
        } else if url.contains("/issues/") {
            Some(
                url.replace("api.github.com/repos", "github.com")
                    .replace("/issues/", "/issues/"),
            )
        } else {
            notification.repository.html_url.clone().map(Into::into)
        }
    } else {
        notification.repository.html_url.clone().map(Into::into)
    }
}

pub fn format_reason(reason: &str) -> String {
    match reason {
        "assign" => fl!("reason-assign"),
        "author" => fl!("reason-author"),
        "comment" => fl!("reason-comment"),
        "invitation" => fl!("reason-invitation"),
        "manual" => fl!("reason-manual"),
        "mention" => fl!("reason-mention"),
        "review_requested" => fl!("reason-review-requested"),
        "security_alert" => fl!("reason-security-alert"),
        "state_change" => fl!("reason-state-change"),
        "subscribed" => fl!("reason-subscribed"),
        "team_mention" => fl!("reason-team-mention"),
        "ci_activity" => fl!("reason-ci-activity"),
        _ => reason.to_string(),
    }
}
