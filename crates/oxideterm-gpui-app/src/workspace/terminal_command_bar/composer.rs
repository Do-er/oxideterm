// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

// Hallmark · pre-emit critique: P4 H5 E4 S5 R5 V4
// Hallmark · macrostructure: Workbench · genre: modern-minimal · tone: technical and restrained

use super::*;
use crate::workspace::terminal_command_composer::{
    TERMINAL_COMPOSER_RESIZE_HOTZONE_HEIGHT, TerminalCommandComposerDocumentSnapshot,
    TerminalCommandComposerFailure, TerminalCommandComposerId, TerminalCommandComposerStatus,
    TerminalCommandComposerTarget, TerminalCommandComposerTargetScope,
};
use oxideterm_terminal::{TerminalSendInputMode, TerminalSendPacing};
use zeroize::Zeroizing;

const TERMINAL_COMPOSER_CONTROL_HEIGHT: f32 = 24.0;
const TERMINAL_COMPOSER_PANEL_PADDING: f32 = 8.0;
const TERMINAL_COMPOSER_INTERVAL_STEP_LINE_MS: i64 = 100;
const TERMINAL_COMPOSER_INTERVAL_STEP_CHARACTER_MS: i64 = 20;
const TERMINAL_COMPOSER_MODE_CONTROL_WIDTH: f32 = 116.0;
const TERMINAL_COMPOSER_PACING_CONTROL_WIDTH: f32 = 140.0;
const TERMINAL_COMPOSER_SCOPE_CONTROL_WIDTH: f32 = 240.0;
const TERMINAL_COMPOSER_TARGET_LIST_MIN_WIDTH: f32 = 140.0;
const TERMINAL_COMPOSER_TARGET_CONTEXT_MAX_WIDTH: f32 = 200.0;
const TERMINAL_COMPOSER_PROGRESS_HEIGHT: f32 = 2.0;

impl WorkspaceApp {
    pub(in crate::workspace) fn render_terminal_command_composer_panel(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let composer = self.terminal_command_composer.read(cx);
        let target_height = if !composer.is_visible() || !composer.is_expanded() {
            0.0
        } else {
            composer.panel_height_for_viewport(f32::from(window.viewport_size().height))
        };
        let mut tokens = self.tokens;
        if composer.is_resizing() {
            tokens.motion.spatial_enabled = false;
        }
        oxideterm_gpui_ui::motion::auto_height(
            &tokens,
            "terminal-command-composer-height",
            if composer.is_expanded() {
                Some(self.render_terminal_command_composer_content(window, cx))
            } else {
                None
            },
        )
        .target_height(target_height)
        .overflow_when_settled()
        .into_any_element()
    }

    fn render_terminal_command_composer_content(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let composer_expanded = self.terminal_command_composer.read(cx).is_expanded();
        let Some(active) = self
            .terminal_command_composer
            .read(cx)
            .active_document_snapshot()
        else {
            return div().into_any_element();
        };
        if !composer_expanded {
            return div().into_any_element();
        }
        let viewport_height = f32::from(window.viewport_size().height);
        let panel_height = self
            .terminal_command_composer
            .read(cx)
            .panel_height_for_viewport(viewport_height);
        let documents = self.terminal_command_composer.read(cx).document_snapshots();
        let targets = self.terminal_command_composer_target_entries(cx);
        let composer_is_resizing = self.terminal_command_composer.read(cx).is_resizing();
        let has_background = self.window_background_preferences().is_some();

        let panel = div()
            .relative()
            .flex_none()
            .h(px(panel_height))
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(self.workspace_chrome_divider())
            .bg(if has_background {
                rgba(
                    (theme.bg << 8)
                        | oxideterm_gpui_ui::surface::theme_glass_card_background_alpha(
                            &self.tokens,
                        ),
                )
            } else {
                rgb(theme.bg)
            })
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                this.update_terminal_command_composer_resize(event, window, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.finish_terminal_command_composer_resize(cx);
                }),
            )
            .child(
                div()
                    .id("terminal-command-composer-resize")
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(TERMINAL_COMPOSER_RESIZE_HOTZONE_HEIGHT))
                    .cursor(CursorStyle::ResizeRow)
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            if event.click_count >= 2 {
                                this.terminal_command_composer.update(cx, |composer, cx| {
                                    composer
                                        .reset_height(f32::from(window.viewport_size().height), cx);
                                });
                            } else {
                                this.terminal_command_composer.update(cx, |composer, cx| {
                                    composer.start_resize(
                                        event.position.y,
                                        f32::from(window.viewport_size().height),
                                        cx,
                                    );
                                });
                            }
                            window.prevent_default();
                            cx.stop_propagation();
                        }),
                    ),
            )
            .when(composer_is_resizing, |panel| {
                // Keep pointer capture inside the composer surface while its divider moves.
                panel.cursor(CursorStyle::ResizeRow)
            })
            .child(self.render_terminal_command_composer_tabs(&documents, active.id, window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .child(active.editor.clone()),
            )
            .child(self.render_terminal_command_composer_controls(&active, &targets, cx))
            .into_any_element();
        if has_background {
            oxideterm_gpui_ui::surface::tab_surface(&self.tokens, panel)
        } else {
            panel
        }
    }

    fn terminal_command_composer_compact_ghost_text(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        draft: &str,
        cx: &App,
    ) -> Option<String> {
        if snapshot.input_mode != TerminalSendInputMode::Text
            || snapshot.target_scope != TerminalCommandComposerTargetScope::Current
            || snapshot.status == TerminalCommandComposerStatus::Running
            || draft.is_empty()
            || draft
                .chars()
                .any(|character| matches!(character, '\n' | '\r'))
        {
            return None;
        }
        self.active_pane(cx)
            .and_then(|pane| pane.read(cx).history_ghost_text_for_input(draft))
            .filter(|suffix| {
                !suffix
                    .chars()
                    .any(|character| matches!(character, '\n' | '\r'))
            })
    }

    fn terminal_command_composer_compact_history_suggestions(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        cx: &mut Context<Self>,
    ) -> Vec<TerminalCommandSuggestion> {
        if snapshot.input_mode != TerminalSendInputMode::Text
            || snapshot.target_scope != TerminalCommandComposerTargetScope::Current
            || snapshot.status == TerminalCommandComposerStatus::Running
        {
            return Vec::new();
        }
        let draft = Zeroizing::new(
            self.terminal_command_composer
                .read(cx)
                .active_compact_draft()
                .unwrap_or_default()
                .to_string(),
        );
        self.terminal_command_composer_visible_history_suggestions(&draft, cx)
    }

    pub(in crate::workspace) fn accept_terminal_command_composer_suggestion(
        &mut self,
        suggestion: &TerminalCommandSuggestion,
        cx: &mut Context<Self>,
    ) -> TerminalCommandComposerId {
        let mut draft = Zeroizing::new(
            self.terminal_command_composer
                .read(cx)
                .active_compact_draft()
                .unwrap_or_default()
                .to_string(),
        );
        let start = suggestion.replacement.start.min(draft.len());
        let end = suggestion.replacement.end.min(draft.len()).max(start);
        draft.replace_range(start..end, &suggestion.insert_text);
        let caret = draft.encode_utf16().count();
        let document_id = self.terminal_command_composer.update(cx, |composer, cx| {
            composer.replace_active_compact_text(std::mem::take(&mut *draft), cx)
        });
        let target = WorkspaceImeTarget::TerminalCommandComposerCompact;
        self.set_ime_selection_from_anchor(target, caret, caret);
        self.ime_marked_text = None;
        self.show_active_input_caret(cx);
        cx.notify();
        document_id
    }

    fn render_terminal_command_composer_tabs(
        &self,
        documents: &[TerminalCommandComposerDocumentSnapshot],
        active_id: TerminalCommandComposerId,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut tab_row = div().h_full().flex_none().flex().flex_row().items_stretch();
        for document in documents {
            let document_id = document.id;
            let active = document_id == active_id;
            let label = format!(
                "{} {}",
                self.i18n.t("terminal.composer.task"),
                document_id.0
            );
            let status_icon = match document.status {
                TerminalCommandComposerStatus::Running => LucideIcon::LoaderCircle,
                TerminalCommandComposerStatus::Completed => LucideIcon::Check,
                TerminalCommandComposerStatus::Stopped => LucideIcon::Square,
                TerminalCommandComposerStatus::Failed => LucideIcon::AlertCircle,
                TerminalCommandComposerStatus::Idle => LucideIcon::FileText,
            };
            let foreground = if active {
                rgb(self.tokens.ui.accent)
            } else {
                rgb(self.tokens.ui.text_muted)
            };
            tab_row = tab_row.child(
                div()
                    .flex_none()
                    .h_full()
                    .max_w(px(160.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .border_b_1()
                    .border_color(if active {
                        rgb(self.tokens.ui.accent)
                    } else {
                        rgba(0x00000000)
                    })
                    .bg(rgba(0x00000000))
                    .cursor_pointer()
                    .text_size(px(11.0))
                    .text_color(foreground)
                    .hover(|tab| tab.bg(rgb(self.tokens.ui.bg_hover)))
                    .child(Self::render_lucide_icon(status_icon, 12.0, foreground))
                    .child(div().truncate().child(label))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, window, cx| {
                            this.terminal_command_composer.update(cx, |composer, cx| {
                                composer.set_active_document(document_id, cx);
                            });
                            this.focus_terminal_command_composer_editor(document_id, window, cx);
                            cx.stop_propagation();
                        }),
                    ),
            );
        }
        // Scrollable moves the viewport's own style onto an outer wrapper.
        // Keep the tab flex row as a child so its horizontal layout survives
        // that transfer and additional tasks cannot stack behind the editor.
        let tab_list = div()
            .h_full()
            .flex_1()
            .min_w(px(0.0))
            .overflow_x_scrollbar()
            .child(tab_row);

        let task_actions = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(self.terminal_command_action_button(
                LucideIcon::Plus,
                rgb(self.tokens.ui.text_muted),
                false,
                None,
                "terminal-command-composer-add",
                self.i18n.t("terminal.composer.add_task"),
                |this, _event, window, cx| {
                    let document_id = this
                        .terminal_command_composer
                        .update(cx, |composer, cx| composer.add_document(cx));
                    this.focus_terminal_command_composer_editor(document_id, window, cx);
                    cx.stop_propagation();
                },
                cx,
            ))
            .when(documents.len() > 1, |actions| {
                actions.child(self.terminal_command_action_button(
                    LucideIcon::X,
                    rgb(self.tokens.ui.text_muted),
                    false,
                    None,
                    "terminal-command-composer-remove",
                    self.i18n.t("terminal.composer.remove_task"),
                    move |this, _event, window, cx| {
                        this.terminal_command_composer.update(cx, |composer, cx| {
                            composer.remove_document(active_id, cx);
                        });
                        let next_id = this.terminal_command_composer.read(cx).active_document_id();
                        this.focus_terminal_command_composer_editor(next_id, window, cx);
                        cx.stop_propagation();
                    },
                    cx,
                ))
            });

        div()
            .h(px(30.0))
            .flex_none()
            .px(px(TERMINAL_COMPOSER_PANEL_PADDING))
            .flex()
            .items_center()
            .gap(px(6.0))
            .border_b_1()
            .border_color(self.workspace_chrome_divider())
            .when(self.window_background_preferences().is_none(), |strip| {
                strip.bg(rgb(self.tokens.ui.bg))
            })
            .child(tab_list)
            .child(task_actions)
            .into_any_element()
    }

    fn render_terminal_command_composer_controls(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        targets: &[(TerminalCommandComposerTarget, String, TabKind)],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let interval_step = match snapshot.pacing {
            TerminalSendPacing::Line => TERMINAL_COMPOSER_INTERVAL_STEP_LINE_MS,
            TerminalSendPacing::Character => TERMINAL_COMPOSER_INTERVAL_STEP_CHARACTER_MS,
        };
        let status_label = self.terminal_command_composer_status_label(snapshot);
        let progress_percent = if snapshot.total_units == 0 {
            0.0
        } else {
            (snapshot.completed_units as f32 / snapshot.total_units as f32) * 100.0
        };
        let running = snapshot.status == TerminalCommandComposerStatus::Running;
        let selected_target_count = targets
            .iter()
            .filter(|(target, _, _)| snapshot.selected_targets.contains(&target.pane_id))
            .count();
        let current_target_label = self.terminal_command_active_target_label(cx);
        let status_color = match snapshot.status {
            TerminalCommandComposerStatus::Completed => self.tokens.ui.success,
            TerminalCommandComposerStatus::Failed => self.tokens.ui.error,
            TerminalCommandComposerStatus::Running => self.tokens.ui.accent,
            TerminalCommandComposerStatus::Stopped => self.tokens.ui.warning,
            TerminalCommandComposerStatus::Idle => self.tokens.ui.text_muted,
        };
        let left_format = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(self.render_terminal_composer_mode_control(snapshot, cx))
            .child(self.render_terminal_composer_control_divider())
            .child(self.render_terminal_composer_pacing_control(snapshot, cx));

        let right_format = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(self.render_terminal_composer_stepper(
                self.i18n.t("terminal.composer.interval"),
                format!("{} ms", snapshot.interval_ms),
                move |this, _event, cx| {
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.adjust_interval(document_id, -interval_step, cx);
                    });
                },
                move |this, _event, cx| {
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.adjust_interval(document_id, interval_step, cx);
                    });
                },
                None,
                cx,
            ))
            .child(self.render_terminal_composer_control_divider())
            .child(self.render_terminal_composer_stepper(
                self.i18n.t("terminal.composer.repeat"),
                if snapshot.repeat_count == 0 {
                    "♾️".to_string()
                } else {
                    format!("{}×", snapshot.repeat_count)
                },
                move |this, event, cx| {
                    let step = Self::composer_step_from_modifiers(&event.modifiers);
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.adjust_repeat_count(document_id, -step, cx);
                    });
                },
                move |this, event, cx| {
                    let step = Self::composer_step_from_modifiers(&event.modifiers);
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.adjust_repeat_count(document_id, step, cx);
                    });
                },
                Some(Box::new(move |this, cx| {
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.cycle_repeat_preset(document_id, cx);
                    });
                })),
                cx,
            ));

        let format_controls = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .flex_wrap()
            .child(left_format)
            .child(right_format);

        let target_context = (snapshot.target_scope == TerminalCommandComposerTargetScope::Current)
            .then(|| {
                div()
                    .flex_none()
                    .max_w(px(TERMINAL_COMPOSER_TARGET_CONTEXT_MAX_WIDTH))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .text_color(rgb(self.tokens.ui.text_muted))
                    .child(Self::render_lucide_icon(
                        LucideIcon::Terminal,
                        12.0,
                        rgb(self.tokens.ui.text_muted),
                    ))
                    .child(div().truncate().child(current_target_label))
                    .into_any_element()
            });
        let broadcast_context =
            (snapshot.target_scope == TerminalCommandComposerTargetScope::Broadcast).then(|| {
                let active_pane = self.active_pane_id(cx);
                let broadcast_targets = active_pane
                    .map(|pane| self.terminal_broadcast_target_panes(pane, cx))
                    .unwrap_or_default();
                let broadcast_enabled = active_pane
                    .and_then(|pane| self.terminal.read(cx).sync_groups().member(pane))
                    .is_some_and(|member| {
                        !member.isolated
                            && self.terminal.read(cx).sync_groups().enabled(member.group)
                    });
                let (icon_color, label) = if broadcast_enabled && !broadcast_targets.is_empty() {
                    (
                        self.tokens.ui.accent,
                        format!(
                            "{} ({})",
                            self.i18n.t("terminal.composer.broadcast_active"),
                            broadcast_targets.len() + 1
                        ),
                    )
                } else {
                    (
                        self.tokens.ui.text_muted,
                        self.i18n.t("terminal.composer.fallback_current"),
                    )
                };
                div()
                    .flex_none()
                    .max_w(px(TERMINAL_COMPOSER_TARGET_CONTEXT_MAX_WIDTH + 100.0))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .text_color(rgb(icon_color))
                    .child(Self::render_lucide_icon(
                        LucideIcon::Radio,
                        12.0,
                        rgb(icon_color),
                    ))
                    .child(div().truncate().child(label))
                    .into_any_element()
            });
        let selected_targets = (snapshot.target_scope
            == TerminalCommandComposerTargetScope::Selected)
            .then(|| self.render_terminal_composer_target_list(snapshot, targets, cx));
        let target_controls = div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(6.0))
            .flex_wrap()
            .child(
                div()
                    .flex_none()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(self.tokens.ui.text_muted))
                    .child(self.i18n.t("terminal.composer.targets")),
            )
            .child(self.render_terminal_composer_scope_control(snapshot, selected_target_count, cx))
            .when_some(target_context, |row, context| row.child(context))
            .when_some(broadcast_context, |row, context| row.child(context))
            .when_some(selected_targets, |row, target_list| {
                row.child(
                    div()
                        .min_w(px(TERMINAL_COMPOSER_TARGET_LIST_MIN_WIDTH))
                        .flex_1()
                        .child(target_list),
                )
            })
            .when(
                snapshot.target_scope != TerminalCommandComposerTargetScope::Selected,
                |row| row.child(div().flex_1().min_w(px(8.0))),
            )
            .when(
                snapshot.status != TerminalCommandComposerStatus::Idle,
                |row| {
                    row.child(
                        div()
                            .flex_none()
                            .text_size(px(11.0))
                            .text_color(rgb(status_color))
                            .child(status_label),
                    )
                    .child(
                        monospace_datum(
                            &self.tokens,
                            format!(
                                "{}/{} · {} {} · {} {}",
                                snapshot.completed_units,
                                if snapshot.total_units == 0 {
                                    "♾️".to_string()
                                } else {
                                    snapshot.total_units.to_string()
                                },
                                self.i18n.t("terminal.composer.accepted"),
                                snapshot.accepted_writes,
                                self.i18n.t("terminal.composer.skipped"),
                                snapshot.skipped_writes
                            ),
                            Some(settings_mono_font_family(self.settings_store.settings())),
                            MonospaceDatumOptions::new(MonospaceDatumTone::Muted).text_size(10.0),
                        )
                        .whitespace_nowrap(),
                    )
                },
            )
            .child(self.terminal_composer_run_button(document_id, running, cx));

        div()
            .flex_none()
            .border_t_1()
            .border_color(self.workspace_chrome_divider())
            .when(self.window_background_preferences().is_none(), |controls| {
                controls.bg(rgb(self.tokens.ui.bg))
            })
            .when(snapshot.total_units > 0, |panel| {
                panel.child(
                    div()
                        .h(px(TERMINAL_COMPOSER_PROGRESS_HEIGHT))
                        .w_full()
                        .bg(self.workspace_chrome_divider())
                        .child(
                            div()
                                .h_full()
                                .w(relative((progress_percent / 100.0).clamp(0.0, 1.0)))
                                .bg(rgb(self.tokens.ui.accent)),
                        ),
                )
            })
            .child(
                div()
                    .min_h(px(32.0))
                    .px(px(TERMINAL_COMPOSER_PANEL_PADDING))
                    .py(px(2.0))
                    .flex()
                    .items_center()
                    .child(format_controls),
            )
            .child(
                div()
                    .min_h(px(32.0))
                    .px(px(TERMINAL_COMPOSER_PANEL_PADDING))
                    .py(px(2.0))
                    .border_t_1()
                    .border_color(self.workspace_chrome_divider())
                    .flex()
                    .items_center()
                    .child(target_controls),
            )
            .into_any_element()
    }

    fn render_terminal_composer_mode_control(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let active_index = usize::from(snapshot.input_mode == TerminalSendInputMode::Hex);
        let control_id = "terminal-composer-mode";
        let previous_index = self
            .segmented_control_user_previous_index(control_id, active_index)
            .unwrap_or(active_index);
        let transition_active =
            self.segmented_control_user_transition_active(control_id, active_index);
        let can_change = snapshot.status != TerminalCommandComposerStatus::Running;
        let items = [TerminalSendInputMode::Text, TerminalSendInputMode::Hex]
            .into_iter()
            .enumerate()
            .map(|(index, mode)| {
                let label = self.i18n.t(match mode {
                    TerminalSendInputMode::Text => "terminal.composer.text",
                    TerminalSendInputMode::Hex => "terminal.composer.hex",
                });
                oxideterm_gpui_ui::segmented_control_item(
                    &self.tokens,
                    label,
                    index == active_index,
                )
                .rounded_none()
                .px(px(6.0))
                .py(px(2.0))
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        if can_change && index != active_index {
                            this.begin_user_segmented_control_transition_from(
                                control_id,
                                active_index,
                                index,
                                cx,
                            );
                        }
                        this.terminal_command_composer.update(cx, |composer, cx| {
                            composer.set_input_mode(document_id, mode, cx);
                        });
                        cx.stop_propagation();
                    }),
                )
                .into_any_element()
            })
            .collect();
        oxideterm_gpui_ui::segmented_control(
            &self.tokens,
            ("terminal-composer-mode", document_id.0),
            oxideterm_gpui_ui::SegmentedControlOptions::new(active_index, previous_index, 2)
                .user_transition_active(transition_active)
                .underline(TERMINAL_COMPOSER_MODE_CONTROL_WIDTH),
            items,
        )
        .into_any_element()
    }

    fn render_terminal_composer_pacing_control(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let active_index = usize::from(snapshot.pacing == TerminalSendPacing::Character);
        let control_id = "terminal-composer-pacing";
        let previous_index = self
            .segmented_control_user_previous_index(control_id, active_index)
            .unwrap_or(active_index);
        let transition_active =
            self.segmented_control_user_transition_active(control_id, active_index);
        let can_change = snapshot.status != TerminalCommandComposerStatus::Running;
        let items = [TerminalSendPacing::Line, TerminalSendPacing::Character]
            .into_iter()
            .enumerate()
            .map(|(index, pacing)| {
                let label = self.i18n.t(match pacing {
                    TerminalSendPacing::Line => "terminal.composer.by_line",
                    TerminalSendPacing::Character
                        if snapshot.input_mode == TerminalSendInputMode::Hex =>
                    {
                        "terminal.composer.by_byte"
                    }
                    TerminalSendPacing::Character => "terminal.composer.by_character",
                });
                oxideterm_gpui_ui::segmented_control_item(
                    &self.tokens,
                    label,
                    index == active_index,
                )
                .rounded_none()
                .px(px(6.0))
                .py(px(2.0))
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        if can_change && index != active_index {
                            this.begin_user_segmented_control_transition_from(
                                control_id,
                                active_index,
                                index,
                                cx,
                            );
                        }
                        this.terminal_command_composer.update(cx, |composer, cx| {
                            composer.set_pacing(document_id, pacing, cx);
                        });
                        cx.stop_propagation();
                    }),
                )
                .into_any_element()
            })
            .collect();
        oxideterm_gpui_ui::segmented_control(
            &self.tokens,
            ("terminal-composer-pacing", document_id.0),
            oxideterm_gpui_ui::SegmentedControlOptions::new(active_index, previous_index, 2)
                .user_transition_active(transition_active)
                .underline(TERMINAL_COMPOSER_PACING_CONTROL_WIDTH),
            items,
        )
        .into_any_element()
    }

    fn render_terminal_composer_scope_control(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        selected_target_count: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let active_index = match snapshot.target_scope {
            TerminalCommandComposerTargetScope::Current => 0,
            TerminalCommandComposerTargetScope::All => 1,
            TerminalCommandComposerTargetScope::Broadcast => 2,
            TerminalCommandComposerTargetScope::Selected => 3,
        };
        let control_id = "terminal-composer-scope";
        let previous_index = self
            .segmented_control_user_previous_index(control_id, active_index)
            .unwrap_or(active_index);
        let transition_active =
            self.segmented_control_user_transition_active(control_id, active_index);
        let can_change = snapshot.status != TerminalCommandComposerStatus::Running;
        let items = [
            TerminalCommandComposerTargetScope::Current,
            TerminalCommandComposerTargetScope::All,
            TerminalCommandComposerTargetScope::Broadcast,
            TerminalCommandComposerTargetScope::Selected,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, scope)| {
            let label = match scope {
                TerminalCommandComposerTargetScope::Current => {
                    self.i18n.t("terminal.composer.current")
                }
                TerminalCommandComposerTargetScope::All => self.i18n.t("terminal.composer.all"),
                TerminalCommandComposerTargetScope::Broadcast => {
                    self.i18n.t("terminal.composer.broadcast")
                }
                TerminalCommandComposerTargetScope::Selected => format!(
                    "{} ({selected_target_count})",
                    self.i18n.t("terminal.composer.selected")
                ),
            };
            oxideterm_gpui_ui::segmented_control_item(&self.tokens, label, index == active_index)
                .rounded_none()
                .px(px(6.0))
                .py(px(2.0))
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        if can_change && index != active_index {
                            this.begin_user_segmented_control_transition_from(
                                control_id,
                                active_index,
                                index,
                                cx,
                            );
                        }
                        this.terminal_command_composer.update(cx, |composer, cx| {
                            composer.set_target_scope(document_id, scope, cx);
                        });
                        cx.stop_propagation();
                    }),
                )
                .into_any_element()
        })
        .collect();
        oxideterm_gpui_ui::segmented_control(
            &self.tokens,
            ("terminal-composer-scope", document_id.0),
            oxideterm_gpui_ui::SegmentedControlOptions::new(active_index, previous_index, 4)
                .user_transition_active(transition_active)
                .underline(TERMINAL_COMPOSER_SCOPE_CONTROL_WIDTH),
            items,
        )
        .into_any_element()
    }

    fn render_terminal_composer_target_list(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        targets: &[(TerminalCommandComposerTarget, String, TabKind)],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let mut target_row = div().flex_none().flex().items_center().gap(px(4.0));
        for (target, label, _kind) in targets {
            let pane_id = target.pane_id;
            let selected = snapshot.selected_targets.contains(&pane_id);
            let options = ActionChipOptions::new()
                .active(selected)
                .height(24.0)
                .radius(ButtonRadius::Sm)
                .idle_text_tone(ActionChipTextTone::Muted);
            target_row = target_row.child(
                action_chip(
                    &self.tokens,
                    label.clone(),
                    Some(Self::render_lucide_icon(
                        if selected {
                            LucideIcon::CheckSquare
                        } else {
                            LucideIcon::Square
                        },
                        12.0,
                        action_chip_foreground(&self.tokens, options),
                    )),
                    options,
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        this.terminal_command_composer.update(cx, |composer, cx| {
                            composer.toggle_selected_target(document_id, pane_id, cx);
                        });
                        cx.stop_propagation();
                    }),
                ),
            );
        }
        // The inner row retains content-sized target chips while the outer
        // viewport owns horizontal overflow at narrow terminal widths.
        div()
            .w_full()
            .overflow_x_scrollbar()
            .child(target_row)
            .into_any_element()
    }

    fn render_terminal_composer_control_divider(&self) -> AnyElement {
        div()
            .w(px(1.0))
            .h(px(14.0))
            .flex_none()
            .bg(self.workspace_chrome_divider())
            .into_any_element()
    }

    fn composer_step_from_modifiers(modifiers: &gpui::Modifiers) -> i32 {
        if modifiers.alt {
            1000
        } else if modifiers.control || modifiers.platform {
            100
        } else if modifiers.shift {
            10
        } else {
            1
        }
    }

    fn render_terminal_composer_stepper(
        &self,
        label: String,
        value: String,
        decrement: impl Fn(&mut Self, &MouseDownEvent, &mut Context<Self>) + 'static,
        increment: impl Fn(&mut Self, &MouseDownEvent, &mut Context<Self>) + 'static,
        value_click: Option<Box<dyn Fn(&mut Self, &mut Context<Self>) + 'static>>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut value_container = div()
            .h_full()
            .min_w(px(46.0))
            .px(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(11.0))
            .font_family(settings_mono_font_family(self.settings_store.settings()))
            .child(value);

        if let Some(on_click) = value_click {
            value_container = value_container
                .cursor_pointer()
                .hover(|button| button.bg(rgb(self.tokens.ui.bg_hover)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                        on_click(this, cx);
                        cx.stop_propagation();
                    }),
                );
        }

        div()
            .h(px(TERMINAL_COMPOSER_CONTROL_HEIGHT))
            .flex()
            .items_center()
            .overflow_hidden()
            .child(
                div()
                    .h_full()
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .text_size(px(11.0))
                    .text_color(rgb(self.tokens.ui.text_muted))
                    .child(label),
            )
            .child(
                div()
                    .h_full()
                    .w(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|button| button.bg(rgb(self.tokens.ui.bg_hover)))
                    .child("−")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                            decrement(this, event, cx);
                            cx.stop_propagation();
                        }),
                    ),
            )
            .child(value_container)
            .child(
                div()
                    .h_full()
                    .w(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|button| button.bg(rgb(self.tokens.ui.bg_hover)))
                    .child("+")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                            increment(this, event, cx);
                            cx.stop_propagation();
                        }),
                    ),
            )
            .into_any_element()
    }

    fn terminal_composer_run_button(
        &self,
        document_id: TerminalCommandComposerId,
        running: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = if running {
            self.i18n.t("terminal.composer.stop")
        } else {
            self.i18n.t("terminal.composer.start")
        };
        let tokens = self.tokens;
        let tooltip_label = if running {
            self.i18n.t("terminal.composer.stop")
        } else {
            format!("{} (Ctrl+Enter)", self.i18n.t("terminal.composer.start"))
        };
        oxideterm_gpui_ui::button::button_with(
            &self.tokens,
            label,
            oxideterm_gpui_ui::button::ButtonOptions {
                variant: if running {
                    oxideterm_gpui_ui::button::ButtonVariant::Destructive
                } else {
                    oxideterm_gpui_ui::button::ButtonVariant::Default
                },
                size: oxideterm_gpui_ui::button::ButtonSize::Sm,
                radius: oxideterm_gpui_ui::button::ButtonRadius::Md,
                disabled: false,
            },
        )
        .id("terminal-composer-run-button")
        .min_w(px(68.0))
        .tooltip(move |_, cx| {
            oxideterm_gpui_ui::tooltip::tooltip_view(tokens, tooltip_label.clone(), None, cx)
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _event, _window, cx| {
                if running {
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.stop(document_id, cx);
                    });
                } else {
                    this.start_terminal_command_composer(document_id, cx);
                }
                cx.stop_propagation();
            }),
        )
        .into_any_element()
    }

    fn terminal_command_composer_status_label(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
    ) -> String {
        if let Some(failure) = snapshot.failure {
            return self.i18n.t(match failure {
                TerminalCommandComposerFailure::EmptyInput => "terminal.composer.error_empty",
                TerminalCommandComposerFailure::InvalidHex => "terminal.composer.error_hex",
                TerminalCommandComposerFailure::NoTargets => "terminal.composer.error_targets",
                TerminalCommandComposerFailure::TargetBusy => "terminal.composer.error_busy",
            });
        }
        self.i18n.t(match snapshot.status {
            TerminalCommandComposerStatus::Idle => "terminal.composer.idle",
            TerminalCommandComposerStatus::Running => "terminal.composer.running",
            TerminalCommandComposerStatus::Completed => "terminal.composer.completed",
            TerminalCommandComposerStatus::Stopped => "terminal.composer.stopped",
            TerminalCommandComposerStatus::Failed => "terminal.composer.failed",
        })
    }

    fn terminal_command_composer_target_entries(
        &self,
        cx: &App,
    ) -> Vec<(TerminalCommandComposerTarget, String, TabKind)> {
        let tab_host = self.tab_host.read(cx);
        self.terminal_broadcast_entries(cx)
            .into_iter()
            .filter_map(|entry| {
                tab_host.panes().get(&entry.pane_id).map(|pane| {
                    (
                        TerminalCommandComposerTarget {
                            pane_id: entry.pane_id,
                            pane: pane.downgrade(),
                        },
                        format!("{} · #{}", entry.label, entry.pane_id.0),
                        entry.kind,
                    )
                })
            })
            .collect()
    }

    pub(in crate::workspace) fn start_terminal_command_composer(
        &mut self,
        document_id: TerminalCommandComposerId,
        cx: &mut Context<Self>,
    ) -> bool {
        let target_scope = self
            .terminal_command_composer
            .read(cx)
            .document_snapshots()
            .into_iter()
            .find(|document| document.id == document_id)
            .map(|d| d.target_scope)
            .unwrap_or(TerminalCommandComposerTargetScope::Current);

        let current_pane_id = self.active_pane_id(cx);

        if target_scope == TerminalCommandComposerTargetScope::Broadcast {
            let broadcast_enabled = current_pane_id
                .and_then(|pane| self.terminal.read(cx).sync_groups().member(pane))
                .is_some_and(|member| {
                    !member.isolated && self.terminal.read(cx).sync_groups().enabled(member.group)
                });
            if let Some(active_pane) = current_pane_id {
                let targets = if broadcast_enabled {
                    let broadcast_targets = self.terminal_broadcast_target_panes(active_pane, cx);
                    if !broadcast_targets.is_empty() {
                        let mut all = vec![active_pane];
                        all.extend(broadcast_targets);
                        all
                    } else {
                        vec![active_pane]
                    }
                } else {
                    vec![active_pane]
                };
                self.terminal_command_composer.update(cx, |composer, cx| {
                    composer.set_selected_targets(document_id, &targets, cx);
                });
            }
        }
        let entries = self.terminal_command_composer_target_entries(cx);
        let live_panes = entries
            .iter()
            .map(|(target, _, _)| target.pane_id)
            .collect::<HashSet<_>>();
        let candidates = entries.into_iter().map(|(target, _, _)| target).collect();
        self.terminal_command_composer.update(cx, |composer, cx| {
            composer.retain_live_targets(&live_panes, cx);
            composer.start(document_id, current_pane_id, candidates, cx)
        })
    }

    pub(in crate::workspace) fn ensure_terminal_command_composer_visible(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.terminal_command_composer.update(cx, |composer, cx| {
            if !composer.is_visible() {
                composer.toggle_visible(cx);
            }
        });
    }

    pub(in crate::workspace) fn replace_terminal_command_composer_text(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) -> TerminalCommandComposerId {
        self.terminal_command_composer
            .update(cx, |composer, cx| composer.replace_active_text(text, cx))
    }

    pub(in crate::workspace) fn focus_terminal_command_composer_editor(
        &mut self,
        document_id: TerminalCommandComposerId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for control_id in [
            "terminal-composer-mode",
            "terminal-composer-pacing",
            "terminal-composer-scope",
        ] {
            self.segmented_control_user_motion.clear(control_id);
        }
        let editor = self
            .terminal_command_composer
            .read(cx)
            .document_snapshots()
            .into_iter()
            .find(|document| document.id == document_id)
            .map(|document| document.editor);
        if let Some(editor) = editor {
            let focus_handle = editor.read(cx).focus_handle(cx);
            window.focus(&focus_handle, cx);
        }
    }

    pub(in crate::workspace) fn focus_terminal_command_composer_input(
        &mut self,
        document_id: TerminalCommandComposerId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.blur_terminal_quick_commands_input(cx);
        let expanded = self.terminal_command_composer.read(cx).is_expanded();
        if expanded {
            self.focus_terminal_command_composer_editor(document_id, window, cx);
            return;
        }
        self.terminal_command_composer.update(cx, |composer, cx| {
            if !composer.is_visible() {
                composer.toggle_visible(cx);
            }
            composer.set_compact_focused(true, cx);
        });
        self.clear_ime_selection();
        window.focus(&self.focus_handle, cx);
    }

    pub(in crate::workspace) fn terminal_command_composer_editor_focused(
        &self,
        window: &Window,
        cx: &App,
    ) -> bool {
        let composer = self.terminal_command_composer.read(cx);
        composer.is_expanded()
            && composer.active_document_snapshot().is_some_and(|document| {
                document.editor.read(cx).focus_handle(cx).is_focused(window)
            })
    }

    pub(in crate::workspace) fn handle_compact_terminal_command_composer_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let composer = self.terminal_command_composer.read(cx);
        if !composer.is_visible() || composer.is_expanded() {
            return false;
        }
        let Some(document) = composer.active_document_snapshot() else {
            return false;
        };
        if !composer.compact_focused() {
            return false;
        }
        let suggestions_open = composer.compact_suggestions_open();
        let suggestion_highlighted = composer.compact_suggestion_highlighted();

        let modifiers = event.keystroke.modifiers;
        match event.keystroke.key.as_str() {
            "escape" if !modifiers.platform => {
                if suggestions_open {
                    self.terminal_command_composer.update(cx, |composer, cx| {
                        if composer.dismiss_compact_suggestions() {
                            cx.notify();
                        }
                    });
                    return true;
                }
                self.terminal_command_composer.update(cx, |composer, cx| {
                    composer.set_compact_focused(false, cx);
                });
                self.clear_ime_selection();
                self.focus_active_pane(window, cx);
                true
            }
            "enter" if !modifiers.platform && !modifiers.shift && !modifiers.alt => {
                if suggestions_open {
                    let suggestions =
                        self.terminal_command_composer_compact_history_suggestions(&document, cx);
                    if let Some(suggestion) =
                        suggestion_highlighted.and_then(|index| suggestions.get(index))
                    {
                        self.accept_terminal_command_composer_suggestion(suggestion, cx);
                        return true;
                    }
                }
                let document_id = document.id;
                if self.start_terminal_command_composer(document_id, cx) {
                    self.terminal_command_composer.update(cx, |composer, cx| {
                        composer.clear_compact_text(document_id, cx);
                    });
                    self.clear_ime_selection();
                }
                true
            }
            "up" | "arrowup" | "down" | "arrowdown"
                if !modifiers.platform
                    && !modifiers.control
                    && !modifiers.alt
                    && !modifiers.shift
                    && self
                        .marked_text_for_target(
                            WorkspaceImeTarget::TerminalCommandComposerCompact,
                            cx,
                        )
                        .is_none() =>
            {
                let suggestions =
                    self.terminal_command_composer_compact_history_suggestions(&document, cx);
                if suggestions.is_empty() {
                    return false;
                }
                let move_down = matches!(event.keystroke.key.as_str(), "down" | "arrowdown");
                self.terminal_command_composer.update(cx, |composer, cx| {
                    composer.move_compact_suggestion_selection(suggestions.len(), move_down, cx);
                });
                true
            }
            "tab"
                if !modifiers.platform
                    && !modifiers.control
                    && !modifiers.alt
                    && !modifiers.shift =>
            {
                if suggestions_open {
                    let suggestions =
                        self.terminal_command_composer_compact_history_suggestions(&document, cx);
                    if let Some(suggestion) =
                        suggestion_highlighted.and_then(|index| suggestions.get(index))
                    {
                        self.accept_terminal_command_composer_suggestion(suggestion, cx);
                        return true;
                    }
                }
                self.accept_terminal_command_composer_ghost_text(&document, cx)
            }
            "right" | "arrowright"
                if !modifiers.platform
                    && !modifiers.control
                    && !modifiers.alt
                    && !modifiers.shift =>
            {
                self.accept_terminal_command_composer_ghost_text(&document, cx)
            }
            _ => false,
        }
    }

    fn accept_terminal_command_composer_ghost_text(
        &mut self,
        document: &TerminalCommandComposerDocumentSnapshot,
        cx: &mut Context<Self>,
    ) -> bool {
        let target = WorkspaceImeTarget::TerminalCommandComposerCompact;
        let mut draft = Zeroizing::new(
            self.terminal_command_composer
                .read(cx)
                .active_compact_draft()
                .unwrap_or_default()
                .to_string(),
        );
        let draft_len = draft.encode_utf16().count();
        let caret_is_at_end = self
            .ime_selection_range_for_target(target, cx)
            .is_some_and(|range| range.start == draft_len && range.end == draft_len)
            && self.marked_text_for_target(target, cx).is_none();
        let Some(ghost_text) = caret_is_at_end
            .then(|| self.terminal_command_composer_compact_ghost_text(&document, &draft, cx))
            .flatten()
        else {
            return false;
        };
        draft.push_str(&ghost_text);
        let caret = draft.encode_utf16().count();
        self.terminal_command_composer.update(cx, |composer, cx| {
            composer.replace_active_compact_text(std::mem::take(&mut *draft), cx);
        });
        self.set_ime_selection_from_anchor(target, caret, caret);
        self.show_active_input_caret(cx);
        cx.notify();
        true
    }

    pub(in crate::workspace) fn update_terminal_command_composer_resize(
        &mut self,
        event: &MouseMoveEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_command_composer.update(cx, |composer, cx| {
            composer.update_resize(
                event.position.y,
                f32::from(window.viewport_size().height),
                event.dragging(),
                cx,
            );
        });
    }

    pub(in crate::workspace) fn finish_terminal_command_composer_resize(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.terminal_command_composer
            .update(cx, |composer, cx| composer.finish_resize(cx));
    }

    pub(in crate::workspace) fn sync_terminal_command_composer_appearance(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let terminal_settings = &self.settings_store.settings().terminal;
        let command_bar_enabled = terminal_settings.command_bar.enabled;
        let font_family = settings_mono_font_family(self.settings_store.settings()).to_string();
        let font_size = terminal_settings.font_size as f32;
        let line_height = terminal_settings.line_height as f32;
        let tokens = self.tokens;
        let background_active = self.window_background_preferences().is_some();
        self.terminal_command_composer.update(cx, |composer, cx| {
            composer.sync_editor_appearance(
                tokens,
                font_family,
                font_size,
                line_height,
                background_active,
                cx,
            );
            if !command_bar_enabled {
                // Disabling the owning surface must not leave hidden jobs with
                // no visible stop control.
                composer.set_expanded(false, cx);
                composer.stop_all(cx);
            }
        });
        if !command_bar_enabled {
            // Removing the dock's owning surface also releases any parameter draft and IME target.
            self.close_terminal_quick_commands_panel(cx);
        }
    }
}

#[cfg(test)]
mod quick_command_preview_tests {
    use super::*;

    struct ChipFixture;

    impl Render for ChipFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().flex().gap(px(5.0)).children(
                [
                    ("short", "Status".to_string()),
                    ("long", "Long command name ".repeat(30)),
                ]
                .into_iter()
                .map(|(id, label)| {
                    action_chip(
                        &oxideterm_theme::default_tokens(),
                        label,
                        Some(
                            div()
                                .size(px(11.0))
                                .debug_selector(move || format!("{id}-icon"))
                                .into_any_element(),
                        ),
                        ActionChipOptions::new().max_width(220.0),
                    )
                    .debug_selector(move || id.into())
                }),
            )
        }
    }

    #[gpui::test]
    fn long_names_keep_chips_bounded_without_shrinking_icons(cx: &mut gpui::TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| ChipFixture);
        cx.simulate_resize(gpui::size(px(800.0), px(100.0)));
        cx.update(|window, app| {
            window.draw(app).clear(app);
        });
        let short = cx.debug_bounds("short").unwrap();
        let long = cx.debug_bounds("long").unwrap();
        assert!(short.size.width < px(220.0));
        assert_eq!(long.size.width, px(220.0));
        assert_eq!(long.left(), short.right() + px(5.0));
        for id in ["short-icon", "long-icon"] {
            assert_eq!(
                cx.debug_bounds(id).unwrap().size,
                gpui::size(px(11.0), px(11.0))
            );
        }
    }
}
