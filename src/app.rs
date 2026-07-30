// SPDX-License-Identifier: GPL-3.0-only

use crate::fl;

use crate::config::Config;
use crate::github::*;
use cosmic::cosmic_config::{self, Config as CosmicConfigHandler, CosmicConfigEntry};
use cosmic::iced::{window::Id, Alignment, Length, Limits, Subscription, Task};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::theme::spacing;
use cosmic::widget;
use octocrab::models::activity::Notification;
use octocrab::models::NotificationId;
use octocrab::Octocrab;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
#[derive(Default)]
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The popup id.
    popup: Option<Id>,
    /// Handle for persisting config changes.
    config_handler: Option<CosmicConfigHandler>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// GitHub service for API interactions
    client: Option<Octocrab>,
    /// Current notifications
    notifications: Vec<Notification>,
    /// Loading state
    is_loading: bool,
    /// Error state
    error_message: Option<String>,
    /// Last refresh time
    last_refresh: Option<std::time::Instant>,
    /// Unread count for the icon
    unread_count: usize,
    /// Token input field value for the setup UI
    token_input: String,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    RefreshNotifications,
    NotificationsLoaded(Result<Vec<Notification>, String>),
    OpenNotification(Notification),
    MarkAsRead(NotificationId),
    MarkAllAsRead,
    NotificationMarkedAsRead(Result<Option<NotificationId>, String>),
    ToggleShowAll(bool),
    UpdateConfig(Config),
    TokenLoaded(Result<String, String>),
    TokenInputChanged(String),
    SaveToken,
    TokenSaved(Result<(), String>),
    Tick,
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "dev.edfloreshz.GitWork";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let config_handler = cosmic_config::Config::new(Self::APP_ID, Config::VERSION).ok();
        let config = config_handler
            .as_ref()
            .map(|ctx| match Config::get_entry(ctx) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();

        let app = AppModel {
            core,
            is_loading: true,
            config_handler,
            config,
            ..Default::default()
        };

        // Load token from the system keyring asynchronously
        let task = Task::perform(
            async move {
                tokio::task::spawn_blocking(|| {
                    keyring::Entry::new("git-work", "github-token")
                        .and_then(|e| e.get_password())
                        .map_err(|e| e.to_string())
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |result| cosmic::Action::App(Message::TokenLoaded(result)),
        );

        (app, task)
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        let icon = if self.unread_count > 0 {
            "mail-unread-symbolic"
        } else {
            "mail-read-symbolic"
        };

        self.core
            .applet
            .icon_button(icon)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let header = widget::row()
            .push(widget::text(fl!("app-title")).size(spacing().space_s))
            .push(widget::horizontal_space().width(Length::Fill))
            .push(
                widget::button::icon(widget::icon::from_name("checkbox-checked-symbolic"))
                    .tooltip(fl!("mark-all-read"))
                    .padding([spacing().space_xxxs, spacing().space_xxs])
                    .on_press_maybe(
                        (!self.notifications.is_empty()).then_some(Message::MarkAllAsRead),
                    ),
            )
            .push(
                widget::button::icon(cosmic::widget::icon::from_name("view-refresh-symbolic"))
                    .padding(spacing().space_xxs)
                    .on_press_maybe((!self.is_loading).then_some(Message::RefreshNotifications)),
            )
            .spacing(spacing().space_xxs)
            .align_y(Alignment::Center);

        let content = if self.client.is_none() && !self.is_loading {
            widget::column()
                .push(widget::text(fl!("token-setup-title")).size(spacing().space_s))
                .push(widget::text(fl!("token-setup-instructions")).size(spacing().space_xs))
                .push(
                    widget::text_input("ghp_...", &self.token_input)
                        .on_input(Message::TokenInputChanged),
                )
                .push_maybe(
                    self.error_message
                        .as_ref()
                        .map(|e| widget::text(e.as_str()).size(spacing().space_xs)),
                )
                .push(
                    widget::button::standard(fl!("token-save")).on_press_maybe(
                        (!self.token_input.is_empty()).then_some(Message::SaveToken),
                    ),
                )
                .spacing(spacing().space_xs)
                .padding(spacing().space_s)
        } else if let Some(error) = &self.error_message {
            widget::column()
                .push(header)
                .push(
                    widget::container(
                        widget::column()
                            .push(widget::text(fl!("error")).size(spacing().space_s))
                            .push(widget::text(error).size(spacing().space_xs))
                            .width(Length::Fill)
                            .spacing(spacing().space_xxs),
                    )
                    .padding(spacing().space_s)
                    .class(cosmic::theme::Container::Card),
                )
                .spacing(spacing().space_xs)
        } else if self.is_loading {
            widget::column()
                .push(header)
                .push(
                    widget::container(
                        widget::row()
                            .push(widget::text(fl!("loading")).size(spacing().space_xs))
                            .push(widget::horizontal_space().width(Length::Fill))
                            .align_y(Alignment::Center),
                    )
                    .padding(spacing().space_s)
                    .class(cosmic::theme::Container::Card),
                )
                .spacing(spacing().space_xs)
        } else if self.notifications.is_empty() {
            widget::column()
                .push(header)
                .push(
                    widget::container(
                        widget::column()
                            .push(widget::icon::from_name("emblem-ok-symbolic").size(32))
                            .push(widget::text(fl!("no-notifications")).size(spacing().space_s))
                            .push(
                                widget::text(fl!("no-notifications-desc")).size(spacing().space_xs),
                            )
                            .spacing(spacing().space_xxs)
                            .align_x(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .padding(spacing().space_l)
                    .class(cosmic::theme::Container::Card),
                )
                .spacing(spacing().space_xs)
        } else {
            let controls = widget::row()
                .push(
                    widget::text(fl!("notifications-count", count = self.notifications.len()))
                        .size(spacing().space_xs),
                )
                .push(widget::horizontal_space().width(Length::Fill))
                .push(
                    widget::toggler(self.config.show_all)
                        .label(fl!("show-all"))
                        .spacing(spacing().space_xxs)
                        .on_toggle(Message::ToggleShowAll),
                )
                .align_y(Alignment::Center)
                .spacing(spacing().space_xxs)
                .apply(widget::container)
                .class(cosmic::style::Container::Card)
                .padding(spacing().space_xxs);

            let mut notifications_list = widget::column().spacing(spacing().space_xxxs);
            for notification in &self.notifications {
                notifications_list = notifications_list.push(self.notification_item(notification));
            }
            let notifications = widget::scrollable(
                widget::container(notifications_list)
                    .padding([spacing().space_none, spacing().space_xxxs]),
            )
            .height(Length::Fill);

            widget::column()
                .push(header)
                .push(notifications)
                .push(controls)
                .spacing(spacing().space_xxs)
        };

        self.core
            .applet
            .popup_container(
                widget::container(content)
                    .padding(spacing().space_xs)
                    .class(cosmic::theme::Container::Dialog),
            )
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
            cosmic::iced::time::every(std::time::Duration::from_secs(
                self.config.refresh_interval_secs,
            ))
            .map(|_| Message::Tick),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        let mut tasks = vec![];

        match message {
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits =
                        Limits::NONE.max_width(500.0).max_height(620.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::RefreshNotifications => {
                if let Some(client) = &self.client {
                    self.is_loading = true;
                    self.error_message = None;
                    let all = self.config.show_all;
                    let client = client.clone();
                    return Task::perform(
                        async move {
                            client
                                .activity()
                                .notifications()
                                .list()
                                .all(all)
                                .send()
                                .await
                                .map(|r| r.items)
                                .map_err(|e| e.to_string())
                        },
                        |result| cosmic::Action::App(Message::NotificationsLoaded(result)),
                    );
                }
            }
            Message::NotificationsLoaded(result) => {
                self.is_loading = false;
                self.last_refresh = Some(std::time::Instant::now());
                match result {
                    Ok(notifications) => {
                        self.unread_count = notifications.iter().filter(|n| n.unread).count();
                        self.notifications = notifications;
                        self.error_message = None;
                    }
                    Err(error) => {
                        self.error_message = Some(error);
                    }
                }
            }
            Message::OpenNotification(notification) => {
                if let Some(client) = &self.client {
                    if let Some(url) = get_notification_url(&notification) {
                        let _ = open::that_detached(url);
                    }

                    // Mark as read if it was unread
                    if notification.unread {
                        let notification_id = notification.id.clone();
                        let client = client.clone();
                        return Task::perform(
                            async move {
                                client
                                    .activity()
                                    .notifications()
                                    .mark_as_read(notification_id.into())
                                    .await
                                    .map_err(|e| e.to_string())?;
                                Ok(Some(notification_id))
                            },
                            |result| cosmic::Action::App(Message::NotificationMarkedAsRead(result)),
                        );
                    }
                }
            }
            Message::MarkAsRead(notification_id) => {
                if let Some(client) = &self.client {
                    let client = client.clone();
                    return Task::perform(
                        async move {
                            client
                                .activity()
                                .notifications()
                                .mark_as_read(notification_id.into())
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok(Some(notification_id))
                        },
                        |result| cosmic::Action::App(Message::NotificationMarkedAsRead(result)),
                    );
                }
            }
            Message::MarkAllAsRead => {
                if let Some(client) = &self.client {
                    let client = client.clone();
                    return Task::perform(
                        async move {
                            client
                                .activity()
                                .notifications()
                                .mark_all_as_read(None)
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok(None)
                        },
                        |result| cosmic::Action::App(Message::NotificationMarkedAsRead(result)),
                    );
                }
            }
            Message::NotificationMarkedAsRead(result) => match result {
                Ok(None) => {
                    tasks.push(cosmic::task::message(Message::RefreshNotifications));
                }
                Ok(Some(notification_id)) => {
                    self.notifications
                        .iter_mut()
                        .find(|n| n.id == notification_id)
                        .map(|n| n.unread = !n.unread);
                }
                Err(error) => {
                    self.error_message = Some(format!("{}: {}", fl!("error-mark-read"), error));
                }
            },
            Message::ToggleShowAll(show_all) => {
                self.config.show_all = show_all;
                if let Some(handler) = &self.config_handler {
                    let _ = self.config.write_entry(handler);
                }
                tasks.push(cosmic::task::message(Message::RefreshNotifications));
            }
            Message::TokenLoaded(result) => match result {
                Ok(token) => match octocrab::OctocrabBuilder::new()
                    .personal_token(token)
                    .build()
                {
                    Ok(client) => {
                        self.client = Some(client);
                        tasks.push(cosmic::task::message(Message::RefreshNotifications));
                    }
                    Err(e) => {
                        self.is_loading = false;
                        self.error_message = Some(e.to_string());
                    }
                },
                Err(_) => {
                    // No token in keyring — show the setup UI
                    self.is_loading = false;
                }
            },
            Message::TokenInputChanged(value) => {
                self.token_input = value;
            }
            Message::SaveToken => {
                let token = self.token_input.clone();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            keyring::Entry::new("git-work", "github-token")
                                .and_then(|e| e.set_password(&token))
                                .map_err(|e| e.to_string())
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |result| cosmic::Action::App(Message::TokenSaved(result)),
                );
            }
            Message::TokenSaved(result) => match result {
                Ok(()) => {
                    let token = self.token_input.clone();
                    match octocrab::OctocrabBuilder::new()
                        .personal_token(token)
                        .build()
                    {
                        Ok(client) => {
                            self.client = Some(client);
                            self.error_message = None;
                            self.is_loading = true;
                            tasks.push(cosmic::task::message(Message::RefreshNotifications));
                        }
                        Err(e) => {
                            self.error_message = Some(e.to_string());
                        }
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to save token: {}", e));
                }
            },
            Message::Tick => {
                if self.client.is_some() && !self.is_loading {
                    tasks.push(cosmic::task::message(Message::RefreshNotifications));
                }
            }
        }
        Task::batch(tasks)
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

impl AppModel {
    fn notification_item<'a>(&self, notification: &'a Notification) -> Element<'a, Message> {
        let reason = format_reason(&notification.reason);

        let header = widget::row()
            .push_maybe(
                notification.unread.then_some(
                    widget::button::icon(cosmic::widget::icon::from_name(
                        "mail-mark-read-symbolic",
                    ))
                    .padding(spacing().space_xxxs)
                    .on_press(Message::MarkAsRead(notification.id.clone()))
                    .class(cosmic::theme::Button::Text),
                ),
            )
            .push(
                widget::column()
                    .push(
                        widget::button::link(notification.subject.title.clone())
                            .padding(spacing().space_none)
                            .on_press(Message::OpenNotification(notification.clone())),
                    )
                    .push_maybe(
                        notification
                            .repository
                            .full_name
                            .as_ref()
                            .map(|name| widget::text(name).size(spacing().space_xs)),
                    )
                    .spacing(spacing().space_xxxs),
            )
            .spacing(spacing().space_xxs)
            .align_y(Alignment::Center);

        let time_ago = format_time_ago(&notification.updated_at);

        let footer = widget::row()
            .push(widget::text(time_ago).size(11))
            .push(widget::horizontal_space().width(Length::Fill))
            .push(widget::text(reason).size(11));

        widget::container(
            widget::column()
                .push(header)
                .push(footer)
                .spacing(spacing().space_xxxs),
        )
        .class(cosmic::style::Container::Card)
        .padding(spacing().space_xxs)
        .width(Length::Fill)
        .into()
    }
}

fn format_time_ago(datetime: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*datetime);

    if duration.num_days() > 0 {
        fl!("time-days-ago", days = duration.num_days())
    } else if duration.num_hours() > 0 {
        fl!("time-hours-ago", hours = duration.num_hours())
    } else if duration.num_minutes() > 0 {
        fl!("time-minutes-ago", minutes = duration.num_minutes())
    } else {
        fl!("time-just-now")
    }
}
