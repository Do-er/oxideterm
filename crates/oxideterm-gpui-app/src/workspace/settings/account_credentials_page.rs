// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use super::*;
use gpui::ClipboardItem;
use oxideterm_connections::{AccountCredential, AccountCredentialError};
use zeroize::Zeroizing;

pub(super) struct AccountCredentialDraft {
    pub id: Option<String>,
    pub name: String,
    pub username: String,
    pub password: Zeroizing<String>,
}

impl AccountCredentialDraft {
    pub(super) fn new(credential: Option<&AccountCredential>) -> Self {
        Self {
            id: credential.map(|entry| entry.id.clone()),
            name: credential.map(|entry| entry.name.clone()).unwrap_or_default(),
            username: credential.map(|entry| entry.username.clone()).unwrap_or_default(),
            password: Zeroizing::new(String::new()),
        }
    }
}

impl WorkspaceApp {
    fn edit_account_credential(
        &mut self,
        credential: Option<AccountCredential>,
        cx: &mut Context<Self>,
    ) {
        let draft = AccountCredentialDraft::new(credential.as_ref());
        self.settings_workspace.update(cx, |settings, cx| {
            settings.account_credential_draft = Some(draft);
            settings.account_credential_error = None;
            settings.settings_focused_input = Some(SettingsInput::AccountCredentialName);
            cx.notify();
        });
        self.clear_ime_selection();
        cx.notify();
    }

    fn save_account_credential_draft(&mut self, cx: &mut Context<Self>) {
        let Some(mut draft) = self
            .settings_workspace
            .update(cx, |settings, _| settings.account_credential_draft.take())
        else {
            return;
        };
        let password_str = std::mem::take(&mut *draft.password);
        let secret = (!password_str.is_empty()).then(|| SecretString::from(password_str.clone()));
        let result = self.connection_store.save_account_credential(
            draft.id.as_deref(),
            &draft.name,
            &draft.username,
            secret.as_ref(),
        );
        if result.is_ok() {
            self.queue_cloud_sync_dirty_refresh(cx);
        }
        self.settings_workspace.update(cx, |settings, cx| {
            settings.settings_focused_input = None;
            settings.account_credential_error = result.err();
            if settings.account_credential_error.is_some() {
                draft.password = Zeroizing::new(password_str);
                settings.account_credential_draft = Some(draft);
            }
            cx.notify();
        });
        self.clear_ime_selection();
        cx.notify();
    }

    pub(in crate::workspace) fn settings_account_credentials_card(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tokens = self.tokens;
        let entries = self.connection_store.account_credentials().to_vec();
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.workspace_toolbar_action_button(
                self.i18n.t("settings_view.account_credentials.add"),
                None,
                ToolbarButtonOptions::default(),
                cx.listener(|this, _, _, cx| this.edit_account_credential(None, cx)),
            ));

        if entries.is_empty() && self.settings_workspace.read(cx).account_credential_draft.is_none() {
            body = body.child(
                div()
                    .py_2()
                    .text_size(px(tokens.metrics.ui_text_sm))
                    .text_color(rgb(tokens.ui.text_muted))
                    .child(self.i18n.t("settings_view.account_credentials.no_credentials")),
            );
        }

        for credential in entries {
            let edit = credential.clone();
            let id = credential.id.clone();
            let username = credential.username.clone();
            let ref_count = self.connection_store.count_account_credential_references(&id);

            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(tokens.ui.border))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .truncate()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(credential.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(tokens.metrics.ui_text_xs))
                                    .text_color(rgb(tokens.ui.text_muted))
                                    .child(format!(
                                        "{}: {}",
                                        self.i18n.t("settings_view.account_credentials.username"),
                                        credential.username
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(tokens.metrics.ui_text_xs))
                            .text_color(rgb(tokens.ui.text_muted))
                            .child(
                                self.i18n
                                    .t("settings_view.account_credentials.in_use")
                                    .replace("{{count}}", &ref_count.to_string()),
                            ),
                    )
                    .child(self.workspace_toolbar_action_button(
                        self.i18n.t("settings_view.account_credentials.copy_username"),
                        None,
                        ToolbarButtonOptions {
                            button: ButtonOptions {
                                size: ButtonSize::Sm,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        cx.listener(move |_, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(username.clone()));
                        }),
                    ))
                    .child({
                        let cred_id = id.clone();
                        self.workspace_toolbar_action_button(
                            self.i18n.t("settings_view.account_credentials.copy_password"),
                            None,
                            ToolbarButtonOptions {
                                button: ButtonOptions {
                                    size: ButtonSize::Sm,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            cx.listener(move |this, _, _, cx| {
                                if let Ok(secret) = this
                                    .connection_store
                                    .get_account_credential_password(&cred_id)
                                {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        secret.expose_secret().to_string(),
                                    ));
                                }
                            }),
                        )
                    })
                    .child(self.workspace_toolbar_action_button(
                        self.i18n.t("sessionManager.actions.edit"),
                        None,
                        ToolbarButtonOptions {
                            button: ButtonOptions {
                                size: ButtonSize::Sm,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        cx.listener(move |this, _, _, cx| {
                            this.edit_account_credential(Some(edit.clone()), cx)
                        }),
                    ))
                    .child(self.workspace_toolbar_action_button(
                        self.i18n.t("common.actions.delete"),
                        None,
                        ToolbarButtonOptions {
                            button: ButtonOptions {
                                size: ButtonSize::Sm,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        cx.listener(move |this, _, _, cx| {
                            let result = this.connection_store.delete_account_credential(&id);
                            if result.is_ok() {
                                this.queue_cloud_sync_dirty_refresh(cx);
                            }
                            this.settings_workspace.update(cx, |settings, cx| {
                                settings.account_credential_error = result.err();
                                cx.notify();
                            });
                            cx.notify();
                        }),
                    )),
            );
        }

        if let Some(draft) = self.settings_workspace.read(cx).account_credential_draft.as_ref() {
            let id = draft.id.clone();
            let name = draft.name.clone();
            let username = draft.username.clone();

            let editor = div()
                .flex()
                .flex_col()
                .gap_3()
                .p_3()
                .rounded(px(tokens.radii.md))
                .border_1()
                .border_color(rgb(tokens.ui.border))
                .bg(rgb(tokens.ui.bg_sunken))
                .child(self.settings_privilege_text_field(
                    "settings_view.account_credentials.name",
                    SettingsInput::AccountCredentialName,
                    name,
                    self.i18n.t("settings_view.account_credentials.name_placeholder"),
                    false,
                    cx,
                ))
                .child(self.settings_privilege_text_field(
                    "settings_view.account_credentials.username",
                    SettingsInput::AccountCredentialUsername,
                    username,
                    self.i18n.t("settings_view.account_credentials.username_placeholder"),
                    false,
                    cx,
                ))
                .child(self.settings_privilege_text_field(
                    "settings_view.account_credentials.password",
                    SettingsInput::AccountCredentialPassword,
                    "",
                    self.i18n.t(if id.is_some() {
                        "settings_view.account_credentials.password_edit_hint"
                    } else {
                        "settings_view.account_credentials.password_placeholder"
                    }),
                    true,
                    cx,
                ))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child(self.workspace_toolbar_action_button(
                            self.i18n.t("common.actions.cancel"),
                            None,
                            ToolbarButtonOptions::default(),
                            cx.listener(|this, _, _, cx| {
                                this.settings_workspace.update(cx, |settings, cx| {
                                    settings.account_credential_draft = None;
                                    settings.account_credential_error = None;
                                    settings.settings_focused_input = None;
                                    cx.notify();
                                });
                                this.clear_ime_selection();
                                cx.notify();
                            }),
                        ))
                        .child(self.workspace_toolbar_action_button(
                            self.i18n.t("sessionManager.privilege_credentials.save"),
                            None,
                            ToolbarButtonOptions {
                                button: ButtonOptions {
                                    variant: ButtonVariant::Default,
                                    size: ButtonSize::Sm,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            cx.listener(|this, _, _, cx| this.save_account_credential_draft(cx)),
                        )),
                );
            body = body.child(editor);
        }

        if let Some(error) = self.settings_workspace.read(cx).account_credential_error.as_ref() {
            let key = match error {
                AccountCredentialError::InvalidName => {
                    "settings_view.account_credentials.invalid_name"
                }
                AccountCredentialError::InvalidUsername => {
                    "settings_view.account_credentials.invalid_username"
                }
                AccountCredentialError::MissingPassword => {
                    "settings_view.account_credentials.missing_password"
                }
                AccountCredentialError::NotFound => "settings_view.account_credentials.not_found",
                AccountCredentialError::Storage(_) => "settings_view.account_credentials.save_failed",
            };
            body = body.child(
                div()
                    .text_color(rgb(tokens.ui.error))
                    .text_size(px(tokens.metrics.ui_text_sm))
                    .child(self.i18n.t(key)),
            );
        }

        self.settings_card(
            "settings_view.account_credentials.title",
            "settings_view.account_credentials.description",
            vec![body.into_any_element()],
        )
    }
}

