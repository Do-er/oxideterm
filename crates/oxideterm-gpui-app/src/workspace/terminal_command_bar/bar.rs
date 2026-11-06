// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use super::*;
use crate::workspace::terminal_entity::TerminalBroadcastGroupEditKind;
use gpui::StatefulInteractiveElement;
use oxideterm_gpui_ui::dropdown_menu::{
    DropdownMenuItemKind, dropdown_menu_content, dropdown_menu_item, dropdown_menu_separator,
};

const TERMINAL_RECORDING_MENU_WIDTH: f32 = 220.0;
const TERMINAL_RECORDING_MENU_BOTTOM: f32 = 30.0;

impl WorkspaceApp {
    pub(super) fn render_terminal_command_bar(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        const COMMAND_BAR_BG_ALPHA: u32 = 0xf2; // Tauri bg-theme-bg/95
        const COMMAND_BAR_BORDER_ALPHA: u32 = 0xb3; // Tauri border-theme-border/70

        let theme = self.tokens.ui;
        let command_bar_background = if self.window_background_preferences().is_some() {
            self.workspace_chrome_background(theme.bg)
        } else {
            rgba((theme.bg << 8) | COMMAND_BAR_BG_ALPHA)
        };
        let workspace = cx.entity();
        let active_pane_id = self.active_pane_id(cx);
        let split_controls_visible = matches!(
            self.active_terminal_kind(cx),
            Some(
                oxideterm_terminal::TerminalSessionKind::LocalPty
                    | oxideterm_terminal::TerminalSessionKind::SshPty
            )
        );
        let can_configure_remote_integration = self.active_ssh_terminal_node_id(cx).is_some();
        let remote_integration_pending = self.remote_shell_integration_pending(cx);
        let remote_integration_tooltip_id = "terminal-command-configure-directory-tracking";
        let remote_integration_tooltip_title = self
            .i18n
            .t("settings_view.connections.shell_integration.toolbar_action");
        let can_split = self.can_split_active_pane(cx);
        let broadcast_targets =
            self.terminal_broadcast_target_panes(active_pane_id.unwrap_or(PaneId(0)), cx);
        let broadcast_enabled = active_pane_id
            .and_then(|pane| self.terminal.read(cx).sync_groups().member(pane))
            .is_some_and(|member| {
                !member.isolated && self.terminal.read(cx).sync_groups().enabled(member.group)
            });
        let broadcast_label = if broadcast_enabled {
            broadcast_targets.len().to_string()
        } else {
            "0".to_string()
        };
        let broadcast_tooltip_title = if broadcast_enabled {
            format!(
                "{} ({})",
                self.i18n.t("terminal.broadcast.select_targets"),
                broadcast_targets.len()
            )
        } else {
            self.i18n.t("terminal.broadcast.select_targets")
        };
        let quick_commands_open = self.terminal.read(cx).quick_commands.is_open();
        let (composer_expanded, composer_running_count) = {
            let composer = self.terminal_command_composer.read(cx);
            (composer.is_expanded(), composer.running_count())
        };
        let recording_status = self.active_terminal_recording_status(cx);
        let recording_active = recording_status.state != TerminalRecordingState::Idle;
        let session_log_active =
            self.active_terminal_session_log_status(cx).state != TerminalSessionLogState::Idle;
        let timestamps_active = self.active_terminal_timestamps_enabled(cx);
        let highlight_override_active = self.active_terminal_highlight_override(cx);
        let timestamps_tooltip_title = self.i18n.t("terminal.recording.timestamps");
        let recording_toggle_tooltip_title = match recording_status.state {
            TerminalRecordingState::Idle => self.i18n.t("terminal.recording.title"),
            TerminalRecordingState::Recording => self.i18n.t("terminal.recording.pause"),
            TerminalRecordingState::Paused => self.i18n.t("terminal.recording.resume"),
        };
        let capture_menu_tooltip_title = if session_log_active {
            self.i18n.t("terminal.session_log.title")
        } else {
            self.i18n.t("terminal.recording.title")
        };
        let bar = div()
            .relative()
            .flex_none()
            .border_t_1()
            .border_color(self.workspace_divider_color(theme.border, COMMAND_BAR_BORDER_ALPHA))
            .bg(command_bar_background)
            .px(px(12.0))
            .py(px(4.0))
            .shadow_lg()
            .when(self.privilege_credential_picker_is_current(cx), |bar| {
                bar.child(self.render_privilege_credential_picker(cx))
            })
            .when(self.terminal_highlight_popover_open, |bar| {
                bar.child(self.render_terminal_highlight_popover(cx))
            })
            .child(
                div()
                    .w_full()
                    .min_w(px(0.0))
                    .min_h(px(24.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .flex_none()
                            .child(
                                div()
                                    .relative()
                                    .child(self.terminal_command_action_button(
                                        LucideIcon::ListChecks,
                                        if composer_expanded {
                                            rgb(theme.accent)
                                        } else if composer_running_count > 0 {
                                            rgb(theme.warning)
                                        } else {
                                            rgb(theme.text_muted)
                                        },
                                        false,
                                        Some(if composer_expanded {
                                            rgba((theme.accent << 8) | 0x26)
                                        } else {
                                            rgba(0x00000000)
                                        }),
                                        "terminal-command-composer-toggle",
                                        if composer_running_count > 0 {
                                            format!(
                                                "{} ({})",
                                                self.i18n.t("terminal.composer.title"),
                                                composer_running_count
                                            )
                                        } else {
                                            self.i18n.t("terminal.composer.title")
                                        },
                                        move |this, _event, window, cx| {
                                            let expanding = !this
                                                .terminal_command_composer
                                                .read(cx)
                                                .is_expanded();
                                            if expanding {
                                                this.close_terminal_command_overlays(cx);
                                                this.ime_marked_text = None;
                                            }
                                            this.terminal_command_composer.update(
                                                cx,
                                                |composer, cx| {
                                                    composer.toggle_expanded(cx);
                                                },
                                            );
                                            let document_id = this
                                                .terminal_command_composer
                                                .read(cx)
                                                .active_document_id();
                                            if expanding {
                                                this.focus_terminal_command_composer_editor(
                                                    document_id,
                                                    window,
                                                    cx,
                                                );
                                            } else {
                                                this.focus_active_pane(window, cx);
                                            }
                                            cx.stop_propagation();
                                        },
                                        cx,
                                    ))
                                    .when(composer_running_count > 0, |container| {
                                        container.child(
                                            div()
                                                .absolute()
                                                .top(px(-3.0))
                                                .right(px(-3.0))
                                                .h(px(14.0))
                                                .min_w(px(14.0))
                                                .px(px(3.0))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded_full()
                                                .bg(rgb(theme.warning))
                                                .text_size(px(9.0))
                                                .font_weight(gpui::FontWeight::BOLD)
                                                .text_color(rgb(theme.bg))
                                                .child(composer_running_count.to_string()),
                                        )
                                    }),
                            )
                            .child(self.terminal_command_action_button(
                                LucideIcon::Send,
                                if quick_commands_open {
                                    rgb(theme.accent)
                                } else {
                                    rgb(theme.text_muted)
                                },
                                false,
                                Some(if quick_commands_open {
                                    rgba((theme.accent << 8) | 0x26)
                                } else {
                                    rgba(0x00000000)
                                }),
                                "terminal-command-quick-commands",
                                self.i18n.t("terminal.quick_commands.title"),
                                |this, _event, window, cx| {
                                    this.toggle_terminal_quick_commands_panel(window, cx);
                                    cx.stop_propagation();
                                },
                                cx,
                            ))
                            .child(select_anchor_probe(
                                SelectAnchorId::TerminalBroadcastMenu,
                                div()
                                    .id("terminal-command-broadcast")
                                    .h(px(22.0))
                                    .px(px(6.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .rounded(px(self.tokens.radii.md))
                                    .border_1()
                                    .border_color(if broadcast_enabled {
                                        rgba((theme.accent << 8) | 0x4d)
                                    } else {
                                        rgba((theme.border << 8) | 0x80)
                                    })
                                    .bg(if broadcast_enabled {
                                        rgba((theme.accent << 8) | 0x1a)
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .hover(|style| {
                                        style.bg(if broadcast_enabled {
                                            rgba((theme.accent << 8) | 0x2e)
                                        } else {
                                            rgba((theme.bg_hover << 8) | 0x80)
                                        })
                                    })
                                    .cursor_pointer()
                                    .text_size(px(11.0))
                                    .text_color(if broadcast_enabled {
                                        rgb(theme.accent)
                                    } else {
                                        rgb(theme.text_muted)
                                    })
                                    .child(Self::render_lucide_icon(
                                        LucideIcon::Radio,
                                        12.0,
                                        if broadcast_enabled {
                                            rgb(theme.accent)
                                        } else {
                                            rgb(theme.text_muted)
                                        },
                                    ))
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .child(broadcast_label),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                            this.terminal_highlight_popover_open = false;
                                            this.toggle_terminal_broadcast_menu(cx);
                                            cx.stop_propagation();
                                            cx.notify();
                                        }),
                                    )
                                    .on_mouse_move(
                                        cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                                            this.queue_workspace_tooltip(
                                                "terminal-command-broadcast",
                                                broadcast_tooltip_title.clone(),
                                                f32::from(event.position.x) + 12.0,
                                                f32::from(event.position.y) + 16.0,
                                                cx,
                                            );
                                        }),
                                    )
                                    .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                                        if !*hovered {
                                            this.clear_workspace_tooltip("terminal-command-broadcast", cx);
                                        }
                                    })),
                                {
                                    let workspace = workspace.clone();
                                    move |anchor, _window, cx| {
                                        let _ = workspace.update(cx, |this, cx| {
                                            this.update_select_anchor(anchor, cx);
                                        });
                                    }
                                },
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .flex()
                            .items_center()
                            .justify_start()
                            .when(composer_expanded || quick_commands_open, |mid| {
                                mid.child(self.render_terminal_composer_bar_targets(cx))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap(px(4.0))
                            .children(self.render_terminal_ssh_latency_chip(window, cx))
                            .when(split_controls_visible, |actions| {
                                // Transport readiness controls the disabled state; keeping SSH
                                // actions visible makes their placement match local terminals.
                                actions
                                    .child(self.terminal_command_action_button(
                                        LucideIcon::SplitSquareHorizontal,
                                        rgb(theme.text_muted),
                                        !can_split,
                                        None,
                                        "terminal-command-split-horizontal",
                                        self.i18n.t("command_palette.cmd_split_horizontal"),
                                        |this, _event, window, cx| {
                                            this.split_active_pane(
                                                SplitDirection::Horizontal,
                                                window,
                                                cx,
                                            );
                                            cx.stop_propagation();
                                        },
                                        cx,
                                    ))
                                    .child(self.terminal_command_action_button(
                                        LucideIcon::SplitSquareVertical,
                                        rgb(theme.text_muted),
                                        !can_split,
                                        None,
                                        "terminal-command-split-vertical",
                                        self.i18n.t("command_palette.cmd_split_vertical"),
                                        |this, _event, window, cx| {
                                            this.split_active_pane(
                                                SplitDirection::Vertical,
                                                window,
                                                cx,
                                            );
                                            cx.stop_propagation();
                                        },
                                        cx,
                                    ))
                            })
                            .when(can_configure_remote_integration, |actions| {
                                actions.child(self.terminal_command_action_button(
                                    LucideIcon::FolderSync,
                                    rgb(theme.text_muted),
                                    remote_integration_pending,
                                    None,
                                    remote_integration_tooltip_id,
                                    remote_integration_tooltip_title,
                                    |this, _event, _window, cx| {
                                        this.open_remote_shell_integration_confirm(cx);
                                        cx.stop_propagation();
                                    },
                                    cx,
                                ))
                            })
                            .child(select_anchor_probe(
                                SelectAnchorId::TerminalHighlightRuleSet,
                                self.terminal_command_action_button(
                                    LucideIcon::Hash,
                                    if highlight_override_active {
                                        rgb(theme.accent)
                                    } else {
                                        rgb(theme.text_muted)
                                    },
                                    false,
                                    Some(if highlight_override_active {
                                        rgba((theme.accent << 8) | 0x26)
                                    } else {
                                        rgba(0x00000000)
                                    }),
                                    "terminal-command-highlight-rules",
                                    self.i18n.t("terminal.highlight_override.title"),
                                    |this, _event, _window, cx| {
                                        this.toggle_terminal_highlight_popover(cx);
                                        cx.stop_propagation();
                                    },
                                    cx,
                                )
                                .relative(),
                                {
                                    let workspace = workspace.clone();
                                    move |anchor, _window, cx| {
                                        let _ = workspace.update(cx, |this, cx| {
                                            this.update_select_anchor(anchor, cx);
                                        });
                                    }
                                },
                            ))
                            .child(self.terminal_command_action_button(
                                LucideIcon::Clock,
                                if timestamps_active {
                                    rgb(theme.accent)
                                } else {
                                    rgb(theme.text_muted)
                                },
                                false,
                                Some(if timestamps_active {
                                    rgba((theme.accent << 8) | 0x26)
                                } else {
                                    rgba(0x00000000)
                                }),
                                "terminal-command-timestamps",
                                timestamps_tooltip_title,
                                |this, _event, _window, cx| {
                                    this.toggle_active_terminal_timestamps(cx);
                                    cx.stop_propagation();
                                },
                                cx,
                            ))
                            .when(recording_active, |actions| {
                                actions.child(
                                    div()
                                        .h(px(20.0))
                                        .px(px(6.0))
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .rounded(px(self.tokens.radii.md))
                                        .border_1()
                                        .border_color(rgba((theme.error << 8) | 0x4d))
                                        .bg(rgba((theme.error << 8) | 0x1a))
                                        .text_size(px(11.0))
                                        .text_color(rgb(theme.error))
                                        .child(Self::render_lucide_icon(
                                            LucideIcon::Circle,
                                            10.0,
                                            rgb(theme.error),
                                        ))
                                        .child(format_recording_elapsed(recording_status.elapsed)),
                                )
                            })
                            .when(!recording_active, |actions| {
                                actions.child(
                                    div()
                                        .relative()
                                        .flex_none()
                                        .child(self.terminal_command_action_button(
                                            if session_log_active {
                                                LucideIcon::FileText
                                            } else {
                                                LucideIcon::FileVideo
                                            },
                                            if session_log_active {
                                                rgb(theme.accent)
                                            } else {
                                                rgb(theme.text_muted)
                                            },
                                            false,
                                            Some(if self.terminal_recording_menu_open {
                                                rgba((theme.accent << 8) | 0x26)
                                            } else {
                                                rgba(0x00000000)
                                            }),
                                            "terminal-command-recording-menu",
                                            capture_menu_tooltip_title.clone(),
                                            |this, _event, _window, cx| {
                                                this.toggle_terminal_recording_menu(cx);
                                                cx.stop_propagation();
                                            },
                                            cx,
                                        ))
                                        .when(self.terminal_recording_menu_open, |anchor| {
                                            // Keep the menu in the toolbar button's local coordinate
                                            // owner so its anchor is current in the same draw pass.
                                            anchor.child(self.render_terminal_recording_menu(cx))
                                        }),
                                )
                            })
                            .when(recording_active, |actions| {
                                actions.child(
                                    div()
                                        .relative()
                                        .flex_none()
                                        .child(self.terminal_command_action_button(
                                            LucideIcon::FileText,
                                            if session_log_active {
                                                rgb(theme.accent)
                                            } else {
                                                rgb(theme.text_muted)
                                            },
                                            false,
                                            Some(if self.terminal_recording_menu_open {
                                                rgba((theme.accent << 8) | 0x26)
                                            } else {
                                                rgba(0x00000000)
                                            }),
                                            "terminal-command-session-log-menu",
                                            self.i18n.t("terminal.session_log.title"),
                                            |this, _event, _window, cx| {
                                                this.toggle_terminal_recording_menu(cx);
                                                cx.stop_propagation();
                                            },
                                            cx,
                                        ))
                                        .when(self.terminal_recording_menu_open, |anchor| {
                                            anchor.child(self.render_terminal_recording_menu(cx))
                                        }),
                                )
                            })
                            .when(recording_active, |actions| {
                                actions.child(self.terminal_command_action_button(
                                    if recording_status.state == TerminalRecordingState::Paused {
                                        LucideIcon::Play
                                    } else {
                                        LucideIcon::Circle
                                    },
                                    rgb(theme.error),
                                    false,
                                    Some(rgba((theme.error << 8) | 0x26)),
                                    "terminal-command-recording-toggle",
                                    recording_toggle_tooltip_title.clone(),
                                    move |this, _event, _window, cx| {
                                        match recording_status.state {
                                            TerminalRecordingState::Recording => {
                                                this.pause_active_terminal_recording(cx)
                                            }
                                            TerminalRecordingState::Paused => {
                                                this.resume_active_terminal_recording(cx)
                                            }
                                            TerminalRecordingState::Idle => {}
                                        }
                                        cx.stop_propagation();
                                    },
                                    cx,
                                ))
                            })
                            .when(recording_active, |actions| {
                                actions
                                    .child(self.terminal_command_action_button(
                                        LucideIcon::Square,
                                        rgb(theme.error),
                                        false,
                                        None,
                                        "terminal-command-recording-stop",
                                        self.i18n.t("terminal.recording.stop"),
                                        |this, _event, _window, cx| {
                                            this.stop_active_terminal_recording(cx);
                                            cx.stop_propagation();
                                        },
                                        cx,
                                    ))
                                    .child(self.terminal_command_action_button(
                                        LucideIcon::Trash2,
                                        rgb(theme.error),
                                        false,
                                        None,
                                        "terminal-command-recording-discard",
                                        self.i18n.t("terminal.recording.discard"),
                                        |this, _event, _window, cx| {
                                            this.discard_active_terminal_recording(cx);
                                            cx.stop_propagation();
                                        },
                                        cx,
                                    ))
                            }),
                    ),
            );
        select_anchor_probe(
            SelectAnchorId::TerminalCommandBar,
            bar,
            move |anchor, _window, cx| {
                let _ = workspace.update(cx, |this, cx| {
                    this.update_select_anchor(anchor, cx);
                });
            },
        )
        .into_any_element()
    }

    fn toggle_terminal_recording_menu(&mut self, cx: &mut Context<Self>) {
        let should_open = !self.terminal_recording_menu_open;
        self.terminal_recording_menu_open = should_open;
        if should_open {
            self.blur_terminal_quick_commands_input(cx);
            self.dismiss_terminal_broadcast_menu(cx);
            self.dismiss_terminal_highlight_popover();
            self.dismiss_terminal_target_popover();
        }
        cx.notify();
    }

    pub(in crate::workspace) fn dismiss_terminal_recording_menu(&mut self) -> bool {
        std::mem::take(&mut self.terminal_recording_menu_open)
    }

    fn render_terminal_recording_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let session_log_status = self.active_terminal_session_log_status(cx);
        let session_log_available = self.active_terminal_session_log_available(cx);
        let menu = context_menu_event_boundary(
            dropdown_menu_content(&self.tokens)
                .absolute()
                .bottom(px(TERMINAL_RECORDING_MENU_BOTTOM))
                .right(px(0.0))
                .w(px(TERMINAL_RECORDING_MENU_WIDTH))
                .occlude(),
        );
        let start_item = dropdown_menu_item(
            &self.tokens,
            self.i18n.t("terminal.recording.start"),
            DropdownMenuItemKind::Plain,
            false,
            false,
        );
        let open_item = dropdown_menu_item(
            &self.tokens,
            self.i18n.t("terminal.recording.open_cast"),
            DropdownMenuItemKind::Plain,
            false,
            false,
        );

        let menu = menu
            .child(self.workspace_context_menu_styled_action(
                start_item,
                false,
                false,
                ContextMenuActionableStyle::default(),
                |this| {
                    this.terminal_recording_menu_open = false;
                },
                |this, _event, _window, cx| {
                    this.start_active_terminal_recording(cx);
                },
                cx,
            ))
            .child(self.workspace_context_menu_styled_action(
                open_item,
                false,
                false,
                ContextMenuActionableStyle::default(),
                |this| {
                    this.terminal_recording_menu_open = false;
                },
                |this, _event, window, cx| {
                    this.open_terminal_cast_file(window, cx);
                },
                cx,
            ))
            .child(dropdown_menu_separator(&self.tokens));

        let menu = match session_log_status.state {
            TerminalSessionLogState::Idle => {
                let item = dropdown_menu_item(
                    &self.tokens,
                    self.i18n.t("terminal.session_log.start"),
                    DropdownMenuItemKind::Plain,
                    false,
                    !session_log_available,
                );
                menu.child(self.workspace_context_menu_styled_action(
                    item,
                    !session_log_available,
                    false,
                    ContextMenuActionableStyle::default(),
                    |this| this.terminal_recording_menu_open = false,
                    |this, _event, _window, cx| this.start_active_terminal_session_log(cx),
                    cx,
                ))
            }
            TerminalSessionLogState::Logging => {
                let pause_item = dropdown_menu_item(
                    &self.tokens,
                    self.i18n.t("terminal.session_log.pause"),
                    DropdownMenuItemKind::Plain,
                    false,
                    false,
                );
                let stop_item = dropdown_menu_item(
                    &self.tokens,
                    self.i18n.t("terminal.session_log.stop"),
                    DropdownMenuItemKind::Plain,
                    false,
                    false,
                );
                menu.child(self.workspace_context_menu_styled_action(
                    pause_item,
                    false,
                    false,
                    ContextMenuActionableStyle::default(),
                    |this| this.terminal_recording_menu_open = false,
                    |this, _event, _window, cx| this.pause_active_terminal_session_log(cx),
                    cx,
                ))
                .child(self.workspace_context_menu_styled_action(
                    stop_item,
                    false,
                    false,
                    ContextMenuActionableStyle::default(),
                    |this| this.terminal_recording_menu_open = false,
                    |this, _event, _window, cx| this.stop_active_terminal_session_log(cx),
                    cx,
                ))
            }
            TerminalSessionLogState::Paused => {
                let resume_item = dropdown_menu_item(
                    &self.tokens,
                    self.i18n.t("terminal.session_log.resume"),
                    DropdownMenuItemKind::Plain,
                    false,
                    false,
                );
                let stop_item = dropdown_menu_item(
                    &self.tokens,
                    self.i18n.t("terminal.session_log.stop"),
                    DropdownMenuItemKind::Plain,
                    false,
                    false,
                );
                menu.child(self.workspace_context_menu_styled_action(
                    resume_item,
                    false,
                    false,
                    ContextMenuActionableStyle::default(),
                    |this| this.terminal_recording_menu_open = false,
                    |this, _event, _window, cx| this.resume_active_terminal_session_log(cx),
                    cx,
                ))
                .child(self.workspace_context_menu_styled_action(
                    stop_item,
                    false,
                    false,
                    ContextMenuActionableStyle::default(),
                    |this| this.terminal_recording_menu_open = false,
                    |this, _event, _window, cx| this.stop_active_terminal_session_log(cx),
                    cx,
                ))
            }
        };

        let menu = menu.when(session_log_status.path.is_some(), |menu| {
            let item = dropdown_menu_item(
                &self.tokens,
                self.i18n.t("terminal.session_log.open_file"),
                DropdownMenuItemKind::Plain,
                false,
                false,
            );
            menu.child(self.workspace_context_menu_styled_action(
                item,
                false,
                false,
                ContextMenuActionableStyle::default(),
                |this| this.terminal_recording_menu_open = false,
                |this, _event, _window, cx| this.open_active_terminal_session_log(cx),
                cx,
            ))
        });
        let directory_item = dropdown_menu_item(
            &self.tokens,
            self.i18n.t("terminal.session_log.open_directory"),
            DropdownMenuItemKind::Plain,
            false,
            false,
        );
        menu.child(self.workspace_context_menu_styled_action(
            directory_item,
            false,
            false,
            ContextMenuActionableStyle::default(),
            |this| this.terminal_recording_menu_open = false,
            |this, _event, _window, cx| this.open_terminal_session_log_directory(cx),
            cx,
        ))
        .into_any_element()
    }

    fn render_terminal_broadcast_group_edit_row(
        &self,
        group_id: Option<uuid::Uuid>,
        value: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let target = WorkspaceImeTarget::TerminalBroadcastGroupName;
        let edit_kind = match group_id {
            Some(group_id) => TerminalBroadcastGroupEditKind::Rename(group_id),
            None => TerminalBroadcastGroupEditKind::Create,
        };
        let valid = self.terminal_broadcast_group_name_valid(edit_kind, value);
        let input = self
            .workspace_text_input_borderless(
                target,
                TextInputView {
                    value,
                    placeholder: self.i18n.t("terminal.broadcast.group_name_placeholder"),
                    focused: true,
                    caret_visible: self.input_caret.visible(),
                    secret: false,
                    selected_all: false,
                    selected_range: self.ime_selected_range_for_target(target, cx),
                    marked_text: self.marked_text_for_target(target, cx),
                },
            )
            .w_full()
            .text_size(px(12.0));

        // Rename edits the group row in place; create uses the same row shape at
        // the end of the list so both flows share one inline editor.
        div()
            .h(px(30.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(8.0))
            .rounded(px(self.tokens.radii.md))
            .bg(rgba((theme.accent << 8) | 0x14))
            .child(Self::render_lucide_icon(
                if group_id.is_some() {
                    LucideIcon::Pencil
                } else {
                    LucideIcon::Plus
                },
                12.0,
                rgb(theme.accent),
            ))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(self.text_input_with_workspace_ime(
                        target,
                        input,
                        |_this, _cx| {},
                        cx,
                    )),
            )
            .child(
                div()
                    .size(px(20.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(self.tokens.radii.sm))
                    .when(valid, |button| {
                        button
                            .cursor_pointer()
                            .hover(|button| button.bg(rgb(theme.bg_hover)))
                    })
                    .child(Self::render_lucide_icon(
                        LucideIcon::Check,
                        12.0,
                        rgb(if valid {
                            theme.accent
                        } else {
                            theme.text_muted
                        }),
                    ))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.commit_terminal_broadcast_group_edit(cx);
                            cx.stop_propagation();
                        }),
                    ),
            )
            .child(
                div()
                    .size(px(20.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(self.tokens.radii.sm))
                    .cursor_pointer()
                    .hover(|button| button.bg(rgb(theme.bg_hover)))
                    .child(Self::render_lucide_icon(
                        LucideIcon::X,
                        12.0,
                        rgb(theme.text_muted),
                    ))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.terminal.update(cx, |terminal, _cx| {
                                terminal.cancel_broadcast_group_edit();
                            });
                            this.clear_ime_selection();
                            cx.notify();
                            cx.stop_propagation();
                        }),
                    ),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn render_terminal_broadcast_menu(
        &self,
        placement: TerminalBroadcastMenuPlacement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let entries = self.terminal_broadcast_entries(cx);
        let groups = self.terminal_broadcast_groups().to_vec();
        let selected_group_id = self.terminal.read(cx).selected_broadcast_group_id();
        let group_editor = self
            .terminal
            .read(cx)
            .broadcast_group_editor()
            .map(|(kind, value)| (kind, value.to_string()));
        let anchor_left = self
            .select_anchors
            .get(&SelectAnchorId::TerminalBroadcastMenu)
            .map(|anchor| f32::from(anchor.bounds.origin.x).max(12.0));

        let mut menu = context_menu_event_boundary({
            let menu = div()
                .absolute()
                .w(px(TERMINAL_BROADCAST_MENU_WIDTH))
                .max_h(px(TERMINAL_BROADCAST_MENU_MAX_HEIGHT))
                .rounded(px(self.tokens.radii.lg))
                .border_1()
                .border_color(rgb(theme.border))
                .bg(rgba((theme.bg_elevated << 8) | 0xf2))
                .shadow_lg()
                .p(px(6.0))
                .text_size(px(12.0));
            if let Some(left) = anchor_left {
                menu.left(px(left))
            } else {
                menu.right(px(12.0))
            }
        })
        .id("terminal-broadcast-menu-scroll")
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(6.0))
                .py(px(4.0))
                .text_size(px(11.0))
                .text_color(rgb(theme.text_muted))
                .child(self.i18n.t("terminal.broadcast.groups"))
                .child(
                    div()
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .px(px(6.0))
                        .rounded(px(self.tokens.radii.sm))
                        .cursor_pointer()
                        .hover(|button| button.bg(rgb(theme.bg_hover)))
                        .child(Self::render_lucide_icon(
                            LucideIcon::Plus,
                            12.0,
                            rgb(theme.accent),
                        ))
                        .child(self.i18n.t("terminal.broadcast.create_group"))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _event, window, cx| {
                                this.begin_terminal_broadcast_group_create(window, cx);
                                cx.stop_propagation();
                            }),
                        ),
                ),
        );
        menu = match placement {
            TerminalBroadcastMenuPlacement::Bottom(offset) => menu.bottom(px(offset)),
            TerminalBroadcastMenuPlacement::Top(offset) => menu.top(px(offset)),
        };

        let temp_selected = selected_group_id.is_none();
        let temp_member_count = self.terminal.read(cx).sync_groups().panes(None).len();
        let temp_enabled = self.terminal.read(cx).sync_groups().enabled(None);

        menu = menu.child(
            div()
                .h(px(30.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(8.0))
                .rounded(px(self.tokens.radii.md))
                .cursor_pointer()
                .when(temp_selected, |row| {
                    row.bg(rgba((theme.accent << 8) | 0x1f))
                })
                .hover(|row| row.bg(rgb(theme.bg_hover)))
                .child(if temp_selected {
                    Self::render_lucide_icon(LucideIcon::Check, 12.0, rgb(theme.accent))
                } else {
                    div().size(px(12.0)).into_any_element()
                })
                .child(
                    div()
                        .flex_1()
                        .truncate()
                        .font_weight(if temp_selected {
                            gpui::FontWeight::MEDIUM
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .child(self.i18n.t("terminal.broadcast.temporary_group")),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(if temp_enabled {
                            theme.accent
                        } else {
                            theme.text_muted
                        }))
                        .child(self.i18n.t(if temp_enabled {
                            "terminal.broadcast.sync_enabled"
                        } else {
                            "terminal.broadcast.sync_disabled"
                        })),
                )
                .child(
                    div()
                        .min_w(px(18.0))
                        .px(px(4.0))
                        .py(px(1.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(self.tokens.radii.sm))
                        .bg(rgb(theme.bg_panel))
                        .text_size(px(10.0))
                        .text_color(rgb(theme.text_muted))
                        .child(temp_member_count.to_string()),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _event, _window, cx| {
                        this.terminal.update(cx, |terminal, _cx| {
                            terminal.clear_selected_broadcast_group();
                        });
                        cx.notify();
                        cx.stop_propagation();
                    }),
                ),
        );

        for group in &groups {
            let group_id = group.id;
            if matches!(
                &group_editor,
                Some((TerminalBroadcastGroupEditKind::Rename(editing_id), _))
                    if *editing_id == group_id
            ) {
                let value = group_editor
                    .as_ref()
                    .map(|(_, value)| value.as_str())
                    .unwrap_or_default();
                menu = menu.child(
                    self.render_terminal_broadcast_group_edit_row(Some(group_id), value, cx),
                );
                continue;
            }
            let selected = selected_group_id == Some(group_id);
            let member_count = self
                .terminal
                .read(cx)
                .sync_groups()
                .panes(Some(group_id))
                .len();
            let group_enabled = self
                .terminal
                .read(cx)
                .sync_groups()
                .enabled(Some(group_id));
            let name = group.name.clone();
            menu = menu.child(
                div()
                    .h(px(30.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(8.0))
                    .rounded(px(self.tokens.radii.md))
                    .cursor_pointer()
                    .when(selected, |row| row.bg(rgba((theme.accent << 8) | 0x1f)))
                    .hover(|row| row.bg(rgb(theme.bg_hover)))
                    .child(if selected {
                        Self::render_lucide_icon(LucideIcon::Check, 12.0, rgb(theme.accent))
                    } else {
                        div().size(px(12.0)).into_any_element()
                    })
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .font_weight(if selected {
                                gpui::FontWeight::MEDIUM
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .child(name),
                    )
                    .child(
                        div()
                            .size(px(20.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(self.tokens.radii.sm))
                            .hover(|button| button.bg(rgb(theme.bg_panel)))
                            .child(Self::render_lucide_icon(
                                LucideIcon::Pencil,
                                11.0,
                                rgb(theme.text_muted),
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, window, cx| {
                                    this.begin_terminal_broadcast_group_rename(
                                        group_id, window, cx,
                                    );
                                    cx.stop_propagation();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .size(px(20.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(self.tokens.radii.sm))
                            .hover(|button| button.bg(rgba((theme.error << 8) | 0x22)))
                            .child(Self::render_lucide_icon(
                                LucideIcon::Trash2,
                                11.0,
                                rgb(theme.error),
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, _window, cx| {
                                    this.delete_terminal_broadcast_group(group_id, cx);
                                    cx.stop_propagation();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(if group_enabled {
                                theme.accent
                            } else {
                                theme.text_muted
                            }))
                            .child(self.i18n.t(if group_enabled {
                                "terminal.broadcast.sync_enabled"
                            } else {
                                "terminal.broadcast.sync_disabled"
                            })),
                    )
                    .child(
                        div()
                            .min_w(px(18.0))
                            .px(px(4.0))
                            .py(px(1.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(self.tokens.radii.sm))
                            .bg(rgb(theme.bg_panel))
                            .text_size(px(10.0))
                            .text_color(rgb(theme.text_muted))
                            .child(member_count.to_string()),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, _window, cx| {
                            this.select_terminal_broadcast_group(group_id, cx);
                            cx.stop_propagation();
                        }),
                    ),
            );
        }

        if let Some((TerminalBroadcastGroupEditKind::Create, value)) = &group_editor {
            // A new group is appended to the list, so its inline editor sits at
            // the end of the group section instead of in a detached bottom box.
            menu = menu.child(self.render_terminal_broadcast_group_edit_row(None, value, cx));
        }

        let tiled_panes = self.selected_terminal_tile_panes(cx);
        let can_tile = self.tab_host.read(cx).can_tile_terminal_panes(&tiled_panes);
        menu = menu.child(
            div()
                .px(px(6.0))
                .py(px(4.0))
                .child(self.terminal_sync_action_button(
                    self.i18n.t("terminal.broadcast.tile_selected"),
                    can_tile,
                    |this, _, window, cx| {
                        this.tile_selected_terminal_panes(window, cx);
                        cx.stop_propagation();
                    },
                    cx,
                ))
                .when(!can_tile, |row| {
                    row.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(theme.text_muted))
                            .child(
                                self.i18n
                                    .t("terminal.broadcast.tile_selection_hint")
                                    .replace("{{max}}", &MAX_PANES_PER_TAB.to_string()),
                            ),
                    )
                }),
        );
        let enabled = self.terminal.read(cx).broadcast_enabled();
        let selected_count = self
            .terminal
            .read(cx)
            .sync_groups()
            .panes(selected_group_id)
            .len();
        let selectable_count = entries
            .iter()
            .filter(|entry| {
                let member = self.terminal.read(cx).sync_groups().member(entry.pane_id);
                member.is_none() || member.map(|m| m.group) == Some(selected_group_id)
            })
            .count();
        let all_selected = selectable_count > 0 && selected_count >= selectable_count;

        menu = menu.child(
            div()
                .border_t_1()
                .border_color(self.workspace_chrome_divider())
                .my(px(4.0)),
        );

        menu = menu.child(
            div()
                .px(px(6.0))
                .py(px(4.0))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(rgb(theme.text_muted))
                                .child(self.i18n.t("terminal.broadcast.group_members")),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(rgb(theme.text_muted))
                                .child(format!("({selected_count})")),
                        )
                        .when(selectable_count > 1, |row| {
                            row.child(
                                div()
                                    .cursor_pointer()
                                    .px(px(4.0))
                                    .py(px(1.0))
                                    .rounded(px(self.tokens.radii.sm))
                                    .text_size(px(10.0))
                                    .text_color(rgb(theme.accent))
                                    .hover(|btn| btn.bg(rgb(theme.bg_hover)))
                                    .child(self.i18n.t(if all_selected {
                                        "terminal.broadcast.deselect_all"
                                    } else {
                                        "terminal.broadcast.select_all"
                                    }))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _event, _window, cx| {
                                            this.set_all_terminal_broadcast_group_members(
                                                !all_selected,
                                                cx,
                                            );
                                            cx.stop_propagation();
                                        }),
                                    ),
                            )
                        }),
                )
                .child(
                    div()
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .px(px(8.0))
                        .rounded(px(self.tokens.radii.sm))
                        .when(selected_count > 0, |button| {
                            if enabled {
                                button
                                    .bg(rgba((theme.accent << 8) | 0x22))
                                    .text_color(rgb(theme.accent))
                                    .hover(|btn| btn.bg(rgba((theme.accent << 8) | 0x38)))
                            } else {
                                button
                                    .bg(rgb(theme.bg_hover))
                                    .text_color(rgb(theme.text))
                                    .hover(|btn| btn.bg(rgb(theme.bg_panel)))
                            }
                            .cursor_pointer()
                        })
                        .when(selected_count == 0, |button| {
                            button.text_color(rgb(theme.text_muted)).cursor_not_allowed()
                        })
                        .text_size(px(11.0))
                        .child(Self::render_lucide_icon(
                            if enabled {
                                LucideIcon::Radio
                            } else {
                                LucideIcon::Play
                            },
                            11.0,
                            if selected_count > 0 {
                                rgb(theme.accent)
                            } else {
                                rgb(theme.text_muted)
                            },
                        ))
                        .child(self.i18n.t(if enabled {
                            "terminal.broadcast.pause_group"
                        } else {
                            "terminal.broadcast.start_group"
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _event, _window, cx| {
                                if selected_count > 0 {
                                    this.terminal
                                        .update(cx, |terminal, _| terminal.toggle_broadcast());
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                            }),
                        ),
                ),
        );

        if entries.is_empty() {
            menu = menu.child(
                div()
                    .py(px(16.0))
                    .px(px(8.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .text_color(rgb(theme.text_muted))
                    .child(Self::render_lucide_icon(
                        LucideIcon::Terminal,
                        18.0,
                        rgb(theme.text_muted),
                    ))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .child(self.i18n.t("terminal.broadcast.no_targets")),
                    ),
            );
        }

        for entry in entries {
            let pane_id = entry.pane_id;
            let member = self.terminal.read(cx).sync_groups().member(pane_id);
            let checked = member.is_some_and(|member| member.group == selected_group_id);
            let other_group = member.filter(|member| member.group != selected_group_id);
            let is_isolated = member.is_some_and(|member| member.isolated);
            let kind_icon = match entry.kind {
                TabKind::SshTerminal => LucideIcon::Server,
                _ => LucideIcon::Terminal,
            };

            let row = div()
                .h(px(30.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(8.0))
                .rounded(px(self.tokens.radii.md))
                .when(checked, |row| row.bg(rgba((theme.accent << 8) | 0x0f)))
                .child(if other_group.is_some() {
                    Self::render_lucide_icon(LucideIcon::Lock, 12.0, rgb(theme.text_muted))
                } else if checked {
                    Self::render_lucide_icon(LucideIcon::CheckSquare, 13.0, rgb(theme.accent))
                } else {
                    Self::render_lucide_icon(LucideIcon::Square, 13.0, rgb(theme.text_muted))
                })
                .child(Self::render_lucide_icon(
                    kind_icon,
                    12.0,
                    rgb(if checked {
                        theme.accent
                    } else {
                        theme.text_muted
                    }),
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_weight(if checked {
                            gpui::FontWeight::MEDIUM
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .child(entry.label),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.0))
                        .text_color(rgb(theme.text_muted))
                        .child(format!("#{}", pane_id.0)),
                )
                .when_some(other_group, |row, member| {
                    row.child(
                        div()
                            .px(px(4.0))
                            .py(px(1.0))
                            .rounded(px(self.tokens.radii.sm))
                            .bg(rgb(theme.bg_panel))
                            .text_size(px(10.0))
                            .text_color(rgb(theme.text_muted))
                            .child(self.terminal_sync_group_name(member.group)),
                    )
                })
                .when(checked, |row| {
                    row.when(is_isolated, |r| {
                        r.child(
                            div()
                                .px(px(4.0))
                                .py(px(1.0))
                                .rounded(px(self.tokens.radii.sm))
                                .bg(rgba((theme.warning << 8) | 0x28))
                                .text_size(px(10.0))
                                .text_color(rgb(theme.warning))
                                .child(self.i18n.t("terminal.broadcast.sync_isolated")),
                        )
                    })
                    .child(
                        div()
                            .cursor_pointer()
                            .px(px(5.0))
                            .py(px(2.0))
                            .rounded(px(self.tokens.radii.sm))
                            .hover(|btn| btn.bg(rgb(theme.bg_hover)))
                            .text_size(px(10.0))
                            .text_color(rgb(if is_isolated {
                                theme.accent
                            } else {
                                theme.text_muted
                            }))
                            .child(self.i18n.t(if is_isolated {
                                "terminal.broadcast.resume_member"
                            } else {
                                "terminal.broadcast.isolate_member"
                            }))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, _window, cx| {
                                    this.toggle_terminal_sync_isolation(pane_id, cx);
                                    cx.stop_propagation();
                                }),
                            ),
                    )
                });
            menu = menu.child(self.render_terminal_broadcast_menu_action(
                row,
                other_group.is_some(),
                false,
                Some(rgb(theme.bg_hover)),
                move |this, _, _, cx| {
                    this.toggle_terminal_broadcast_group_member(pane_id, cx);
                },
                cx,
            ));
        }

        menu.into_any_element()
    }

    pub(in crate::workspace) fn terminal_sync_action_button(
        &self,
        label: String,
        enabled: bool,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        self.workspace_toolbar_action_button(
            label,
            None,
            oxideterm_gpui_ui::button::ToolbarButtonOptions {
                button: oxideterm_gpui_ui::button::ButtonOptions {
                    variant: oxideterm_gpui_ui::button::ButtonVariant::Ghost,
                    disabled: !enabled,
                    ..Default::default()
                },
                height: Some(22.0),
                padding_x: Some(6.0),
                ..Default::default()
            },
            cx.listener(listener),
        )
    }

    pub(in crate::workspace) fn render_terminal_sync_member_header(
        &self,
        pane_id: PaneId,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let member = self.terminal.read(cx).sync_groups().member(pane_id)?;
        let enabled = self.terminal.read(cx).sync_groups().enabled(member.group);
        let theme = self.tokens.ui;
        let state = if member.isolated {
            "terminal.broadcast.sync_isolated"
        } else if enabled {
            "terminal.broadcast.sync_enabled"
        } else {
            "terminal.broadcast.sync_disabled"
        };
        Some(
            div()
                .w_full()
                .flex_none()
                .h(px(TERMINAL_SYNC_HEADER_HEIGHT))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .bg(self.workspace_chrome_background(theme.bg))
                .border_b_1()
                .border_color(self.workspace_chrome_divider())
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .text_color(rgb(if member.isolated {
                    theme.warning
                } else if enabled {
                    theme.accent
                } else {
                    theme.text_muted
                }))
                .child(Self::render_lucide_icon(
                    LucideIcon::Radio,
                    12.0,
                    rgb(theme.accent),
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child(self.terminal_sync_group_name(member.group)),
                )
                .child(div().flex_none().child(self.i18n.t(state)))
                .child(self.terminal_sync_action_button(
                    self.i18n.t(if member.isolated {
                        "terminal.broadcast.resume_member"
                    } else {
                        "terminal.broadcast.isolate_member"
                    }),
                    true,
                    move |this, _, _, cx| {
                        this.toggle_terminal_sync_isolation(pane_id, cx);
                        cx.stop_propagation();
                    },
                    cx,
                ))
                .into_any_element(),
        )
    }

    pub(super) fn render_terminal_broadcast_menu_action(
        &self,
        item: gpui::Div,
        disabled: bool,
        loading: bool,
        hover_bg: Option<gpui::Rgba>,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        // A pane owned by another group stays disabled until explicitly removed there.
        // Persistent menu rows still use one shared cx.listener wrapper so
        // toggling targets cannot re-enter WorkspaceApp during the click.
        self.workspace_context_menu_persistent_styled_action(
            item,
            disabled,
            loading,
            ContextMenuActionableStyle {
                hover_background: hover_bg,
                hover_text_color: None,
            },
            listener,
            cx,
        )
    }
}
