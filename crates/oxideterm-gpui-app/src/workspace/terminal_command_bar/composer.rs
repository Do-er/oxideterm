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
use super::terminal_command_bar::{TerminalComposerEditField, TerminalComposerValueEdit};
use oxideterm_gpui_ui::text_input::TextInputView;
use oxideterm_terminal::{TerminalSendInputMode, TerminalSendPacing};
use zeroize::Zeroizing;

const TERMINAL_COMPOSER_PANEL_PADDING: f32 = 8.0;
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
            .child(self.render_terminal_command_composer_controls(&active, documents.len(), cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .child(active.editor.clone()),
            )
            .child(self.render_terminal_command_composer_tabs(&documents, active.id, window, cx))
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
                    .border_t_2()
                    .border_color(if active {
                        rgb(self.tokens.ui.accent)
                    } else {
                        rgba(0x00000000)
                    })
                    .bg(if active {
                        rgba((self.tokens.ui.accent << 8) | 0x14)
                    } else {
                        rgba(0x00000000)
                    })
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
        let tab_list = div()
            .h_full()
            .flex_1()
            .min_w(px(0.0))
            .overflow_x_scrollbar()
            .child(tab_row);

        div()
            .h(px(26.0))
            .w_full()
            .flex_none()
            .px(px(TERMINAL_COMPOSER_PANEL_PADDING))
            .border_t_1()
            .border_color(self.workspace_chrome_divider())
            .when(self.window_background_preferences().is_none(), |strip| {
                strip.bg(rgb(self.tokens.ui.bg))
            })
            .child(tab_list)
            .into_any_element()
    }

    fn render_terminal_command_composer_controls(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        documents_len: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let status_label = self.terminal_command_composer_status_label(snapshot);
        let failure_label = self.terminal_command_composer_failure_label(snapshot);
        let progress_percent = if snapshot.total_units == 0 {
            0.0
        } else {
            (snapshot.completed_units as f32 / snapshot.total_units as f32) * 100.0
        };
        let running = snapshot.status == TerminalCommandComposerStatus::Running;
        let status_color = match snapshot.status {
            TerminalCommandComposerStatus::Completed => rgb(self.tokens.ui.success),
            TerminalCommandComposerStatus::Failed => rgb(self.tokens.ui.error),
            TerminalCommandComposerStatus::Running => rgb(self.tokens.ui.accent),
            TerminalCommandComposerStatus::Stopped => rgb(self.tokens.ui.warning),
            TerminalCommandComposerStatus::Idle => rgb(self.tokens.ui.text_muted),
        };

        let left_controls = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(self.terminal_composer_run_button(document_id, running, cx))
            .child(self.terminal_command_capsule_button(
                LucideIcon::Plus,
                rgb(self.tokens.ui.text_muted),
                false,
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
            .child(self.terminal_command_capsule_button(
                LucideIcon::X,
                rgb(self.tokens.ui.text_muted),
                documents_len <= 1,
                "terminal-command-composer-remove",
                self.i18n.t("terminal.composer.remove_task"),
                move |this, _event, window, cx| {
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.remove_document(document_id, cx);
                    });
                    let next_id = this.terminal_command_composer.read(cx).active_document_id();
                    this.focus_terminal_command_composer_editor(next_id, window, cx);
                    cx.stop_propagation();
                },
                cx,
            ))
            .when(snapshot.status != TerminalCommandComposerStatus::Idle, |left| {
                // Run feedback is read-only, so it lives in one sunken text
                // field instead of separate chips that look interactive.
                let mut status_group = semantic_surface(
                    &self.tokens,
                    SurfaceOptions::new(SurfaceKind::InsetGroup)
                        .padding(SurfacePadding::None)
                        .has_background_image(self.window_background_preferences().is_some()),
                )
                .flex_none()
                .h(px(20.0))
                .px(px(7.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(status_color)
                        .child(status_label),
                );
                if snapshot.status != TerminalCommandComposerStatus::Failed {
                    status_group = status_group
                        .child(self.render_terminal_composer_text_counter(
                            self.i18n.t("terminal.composer.sent"),
                            snapshot.accepted_writes,
                            MonospaceDatumTone::Primary,
                        ))
                        .child(self.render_terminal_composer_text_counter(
                            self.i18n.t("terminal.composer.skipped"),
                            snapshot.skipped_writes,
                            if snapshot.skipped_writes > 0 {
                                MonospaceDatumTone::Warning
                            } else {
                                MonospaceDatumTone::Muted
                            },
                        ));
                }
                status_group = status_group.when_some(failure_label, |group, detail| {
                    group.child(
                        div()
                            .min_w(px(0.0))
                            .max_w(px(240.0))
                            .truncate()
                            .text_size(px(10.0))
                            .text_color(rgb(self.tokens.ui.text_muted))
                            .child(detail),
                    )
                });
                left.child(self.render_terminal_composer_control_divider())
                    .child(status_group)
            });

        let right_controls = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(self.render_terminal_composer_interval_capsule(snapshot, cx))
            .child(self.render_terminal_composer_control_divider())
            .child(self.render_terminal_composer_repeat_capsule(snapshot, cx))
            .child(self.render_terminal_composer_control_divider())
            .child(self.render_terminal_composer_mode_control(snapshot, cx))
            .child(self.render_terminal_composer_pacing_control(snapshot, cx));

        div()
            .flex_none()
            .border_b_1()
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
                    .justify_between()
                    .gap(px(6.0))
                    .child(left_controls)
                    .child(right_controls),
            )
            .into_any_element()
    }

    pub(super) fn open_terminal_target_popover(&mut self, cx: &mut Context<Self>) {
        self.terminal_target_popover_open = true;
        self.commit_terminal_composer_value_edit(cx);
        self.dismiss_terminal_highlight_popover();
        self.dismiss_terminal_broadcast_menu(cx);
        self.dismiss_terminal_recording_menu();
        cx.notify();
    }

    pub(super) fn toggle_terminal_target_popover(&mut self, cx: &mut Context<Self>) {
        if self.terminal_target_popover_open {
            self.terminal_target_popover_open = false;
        } else {
            self.open_terminal_target_popover(cx);
        }
        cx.notify();
    }

    pub(in crate::workspace) fn dismiss_terminal_target_popover(&mut self) -> bool {
        std::mem::take(&mut self.terminal_target_popover_open)
    }

    pub(in crate::workspace) fn begin_terminal_composer_value_edit(
        &mut self,
        field: TerminalComposerEditField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.terminal_command_composer.read(cx).active_document_snapshot() else {
            return;
        };
        let draft = match field {
            TerminalComposerEditField::Interval => snapshot.interval_ms.to_string(),
            TerminalComposerEditField::Repeat => snapshot.repeat_count.to_string(),
        };
        let draft_len = draft.encode_utf16().count();
        self.terminal_composer_value_edit = Some(TerminalComposerValueEdit {
            field,
            draft: draft.clone(),
        });
        self.ime_marked_text = None;
        self.selected_ime_target = Some(WorkspaceImeTarget::TerminalComposerValue);
        self.set_ime_selection_from_anchor(WorkspaceImeTarget::TerminalComposerValue, 0, draft_len);
        window.focus(&self.focus_handle, cx);
        self.show_active_input_caret(cx);
        cx.notify();
    }

    pub(in crate::workspace) fn commit_terminal_composer_value_edit(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(edit) = self.terminal_composer_value_edit.take() else {
            return false;
        };
        let Some(snapshot) = self.terminal_command_composer.read(cx).active_document_snapshot() else {
            self.clear_ime_selection();
            self.selected_ime_target = None;
            cx.notify();
            return true;
        };
        let document_id = snapshot.id;
        match edit.field {
            TerminalComposerEditField::Interval => {
                if let Ok(ms) = edit.draft.trim().parse::<u64>() {
                    self.terminal_command_composer.update(cx, |composer, cx| {
                        composer.set_interval(document_id, ms, cx);
                    });
                }
            }
            TerminalComposerEditField::Repeat => {
                let trimmed = edit.draft.trim();
                if trimmed == "0" || trimmed.is_empty() {
                    self.terminal_command_composer.update(cx, |composer, cx| {
                        composer.set_repeat_count(document_id, 0, cx);
                    });
                } else if let Ok(val) = trimmed.parse::<u32>() {
                    self.terminal_command_composer.update(cx, |composer, cx| {
                        composer.set_repeat_count(document_id, val, cx);
                    });
                }
            }
        }
        self.clear_ime_selection();
        self.selected_ime_target = None;
        cx.notify();
        true
    }

    pub(in crate::workspace) fn cancel_terminal_composer_value_edit(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.terminal_composer_value_edit.take().is_some() {
            self.clear_ime_selection();
            self.selected_ime_target = None;
            cx.notify();
            true
        } else {
            false
        }
    }

    pub(super) fn render_terminal_composer_bar_targets(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(snapshot) = self.terminal_command_composer.read(cx).active_document_snapshot() else {
            return div().into_any_element();
        };
        let document_id = snapshot.id;
        let targets = self.terminal_command_composer_target_entries(cx);
        let selected_target_count = targets
            .iter()
            .filter(|(target, _, _)| snapshot.selected_targets.contains(&target.pane_id))
            .count();
        let theme = self.tokens.ui;
        let is_current = snapshot.target_scope == TerminalCommandComposerTargetScope::Current;
        let is_selected = !is_current;
        let is_popover_open = self.terminal_target_popover_open;

        let selected_label = if selected_target_count > 0 {
            format!("{} ({})", self.i18n.t("terminal.composer.selected"), selected_target_count)
        } else {
            self.i18n.t("terminal.composer.selected")
        };

        let current_tab = div()
            .h(px(20.0))
            .px(px(7.0))
            .flex()
            .items_center()
            .gap(px(3.0))
            .rounded(px(self.tokens.radii.sm))
            .cursor_pointer()
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .font_weight(if is_current {
                gpui::FontWeight::MEDIUM
            } else {
                gpui::FontWeight::NORMAL
            })
            .bg(if is_current {
                rgba((theme.accent << 8) | 0x2e)
            } else {
                rgba(0x00000000)
            })
            .text_color(if is_current {
                rgb(theme.accent)
            } else {
                rgb(theme.text_muted)
            })
            .hover(move |s| {
                s.bg(if is_current {
                    rgba((theme.accent << 8) | 0x3d)
                } else {
                    rgba((theme.bg_hover << 8) | 0x80)
                })
                .text_color(if is_current {
                    rgb(theme.accent)
                } else {
                    rgb(theme.text)
                })
            })
            .child(Self::render_lucide_icon(
                LucideIcon::Terminal,
                11.0,
                if is_current {
                    rgb(theme.accent)
                } else {
                    rgb(theme.text_muted)
                },
            ))
            .child(self.i18n.t("terminal.composer.current"))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.set_target_scope(
                            document_id,
                            TerminalCommandComposerTargetScope::Current,
                            cx,
                        );
                    });
                    this.dismiss_terminal_target_popover();
                    cx.stop_propagation();
                }),
            );

        let selected_tab = select_anchor_probe(
            SelectAnchorId::TerminalTargetMenu,
            div()
                .id("terminal-target-selected-tab")
                .h(px(20.0))
                .px(px(7.0))
                .flex()
                .items_center()
                .gap(px(3.0))
                .rounded(px(self.tokens.radii.sm))
                .cursor_pointer()
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .font_weight(if is_selected {
                    gpui::FontWeight::MEDIUM
                } else {
                    gpui::FontWeight::NORMAL
                })
                .bg(if is_selected || is_popover_open {
                    rgba((theme.accent << 8) | 0x2e)
                } else {
                    rgba(0x00000000)
                })
                .text_color(if is_selected || is_popover_open {
                    rgb(theme.accent)
                } else {
                    rgb(theme.text_muted)
                })
                .hover(move |s| {
                    s.bg(if is_selected || is_popover_open {
                        rgba((theme.accent << 8) | 0x3d)
                    } else {
                        rgba((theme.bg_hover << 8) | 0x80)
                    })
                    .text_color(if is_selected || is_popover_open {
                        rgb(theme.accent)
                    } else {
                        rgb(theme.text)
                    })
                })
                .child(Self::render_lucide_icon(
                    LucideIcon::CheckSquare,
                    11.0,
                    if is_selected || is_popover_open {
                        rgb(theme.accent)
                    } else {
                        rgb(theme.text_muted)
                    },
                ))
                .child(selected_label)
                .child(Self::render_lucide_icon(
                    LucideIcon::ChevronDown,
                    10.0,
                    if is_selected || is_popover_open {
                        rgb(theme.accent)
                    } else {
                        rgb(theme.text_muted)
                    },
                ))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                        if !is_selected {
                            this.terminal_command_composer.update(cx, |composer, cx| {
                                composer.set_target_scope(
                                    document_id,
                                    TerminalCommandComposerTargetScope::Selected,
                                    cx,
                                );
                            });
                            this.open_terminal_target_popover(cx);
                        } else {
                            this.toggle_terminal_target_popover(cx);
                        }
                        cx.stop_propagation();
                    }),
                ),
            {
                let workspace = cx.entity().clone();
                move |anchor, _window, cx| {
                    let _ = workspace.update(cx, |this, cx| {
                        this.update_select_anchor(anchor, cx);
                    });
                }
            },
        );

        let segmented_container = div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .p(px(2.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x80))
            .bg(rgba((theme.bg << 8) | 0x60))
            .child(current_tab)
            .child(selected_tab);

        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .min_w(px(0.0))
            .child(
                div()
                    .flex_none()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(theme.text_muted))
                    .child(self.i18n.t("terminal.composer.targets")),
            )
            .child(segmented_container)
            .into_any_element()
    }

    fn render_terminal_composer_mode_control(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let theme = self.tokens.ui;
        let can_change = snapshot.status != TerminalCommandComposerStatus::Running;
        let is_hex = snapshot.input_mode == TerminalSendInputMode::Hex;

        let make_tab = |mode: TerminalSendInputMode, label: &'static str, active: bool| {
            div()
                .h(px(20.0))
                .px(px(7.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(self.tokens.radii.sm))
                .cursor_pointer()
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .font_weight(if active {
                    gpui::FontWeight::MEDIUM
                } else {
                    gpui::FontWeight::NORMAL
                })
                .bg(if active {
                    rgba((theme.accent << 8) | 0x2e)
                } else {
                    rgba(0x00000000)
                })
                .text_color(if active {
                    rgb(theme.accent)
                } else {
                    rgb(theme.text_muted)
                })
                .hover(move |s| {
                    s.bg(if active {
                        rgba((theme.accent << 8) | 0x3d)
                    } else {
                        rgba((theme.bg_hover << 8) | 0x80)
                    })
                    .text_color(if active {
                        rgb(theme.accent)
                    } else {
                        rgb(theme.text)
                    })
                })
                .child(label)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        if can_change && !active {
                            this.terminal_command_composer.update(cx, |composer, cx| {
                                composer.set_input_mode(document_id, mode, cx);
                            });
                        }
                        cx.stop_propagation();
                    }),
                )
        };

        div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .p(px(2.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x80))
            .bg(rgba((theme.bg << 8) | 0x60))
            .child(make_tab(TerminalSendInputMode::Text, "Text", !is_hex))
            .child(make_tab(TerminalSendInputMode::Hex, "Hex", is_hex))
            .into_any_element()
    }

    fn render_terminal_composer_pacing_control(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let theme = self.tokens.ui;
        let can_change = snapshot.status != TerminalCommandComposerStatus::Running;
        let is_character = snapshot.pacing == TerminalSendPacing::Character;

        let line_label = self.i18n.t("terminal.composer.by_line");
        let char_label = self.i18n.t("terminal.composer.by_character");

        let make_tab = |pacing: TerminalSendPacing, label: String, active: bool| {
            div()
                .h(px(20.0))
                .px(px(7.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(self.tokens.radii.sm))
                .cursor_pointer()
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .font_weight(if active {
                    gpui::FontWeight::MEDIUM
                } else {
                    gpui::FontWeight::NORMAL
                })
                .bg(if active {
                    rgba((theme.accent << 8) | 0x2e)
                } else {
                    rgba(0x00000000)
                })
                .text_color(if active {
                    rgb(theme.accent)
                } else {
                    rgb(theme.text_muted)
                })
                .hover(move |s| {
                    s.bg(if active {
                        rgba((theme.accent << 8) | 0x3d)
                    } else {
                        rgba((theme.bg_hover << 8) | 0x80)
                    })
                    .text_color(if active {
                        rgb(theme.accent)
                    } else {
                        rgb(theme.text)
                    })
                })
                .child(label)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        if can_change && !active {
                            this.terminal_command_composer.update(cx, |composer, cx| {
                                composer.set_pacing(document_id, pacing, cx);
                            });
                        }
                        cx.stop_propagation();
                    }),
                )
        };

        div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .p(px(2.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x80))
            .bg(rgba((theme.bg << 8) | 0x60))
            .child(make_tab(TerminalSendPacing::Line, line_label, !is_character))
            .child(make_tab(TerminalSendPacing::Character, char_label, is_character))
            .into_any_element()
    }

    pub(in crate::workspace) fn render_terminal_target_popover(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(snapshot) = self.terminal_command_composer.read(cx).active_document_snapshot() else {
            return div().into_any_element();
        };
        let document_id = snapshot.id;
        let theme = self.tokens.ui;
        let targets = self.terminal_command_composer_target_entries(cx);
        let anchor_left = self
            .select_anchors
            .get(&SelectAnchorId::TerminalTargetMenu)
            .map(|anchor| f32::from(anchor.bounds.origin.x).max(12.0));

        let all_pane_ids: Vec<PaneId> = targets.iter().map(|(t, _, _)| t.pane_id).collect();
        let header_row = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .px(px(4.0))
            .py(px(2.0))
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .child(
                div()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(theme.text_muted))
                    .child(format!(
                        "{}/{}",
                        snapshot.selected_targets.len(),
                        targets.len()
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .cursor_pointer()
                            .text_color(rgb(theme.accent))
                            .hover(|s| s.underline())
                            .child(self.i18n.t("terminal.composer.select_all"))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener({
                                    let all_pane_ids = all_pane_ids.clone();
                                    move |this, _event, _window, cx| {
                                        this.terminal_command_composer.update(cx, |composer, cx| {
                                            composer.set_selected_targets(document_id, &all_pane_ids, cx);
                                        });
                                        cx.stop_propagation();
                                    }
                                }),
                            ),
                    )
                    .child(
                        div()
                            .cursor_pointer()
                            .text_color(rgb(theme.text_muted))
                            .hover(|s| s.underline())
                            .child(self.i18n.t("terminal.composer.deselect_all"))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, _window, cx| {
                                    this.terminal_command_composer.update(cx, |composer, cx| {
                                        composer.set_selected_targets(document_id, &[], cx);
                                    });
                                    cx.stop_propagation();
                                }),
                            ),
                    ),
            );

        let mut pane_list = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .max_h(px(180.0))
            .overflow_y_scrollbar();

        for (target, label, _kind) in &targets {
            let pane_id = target.pane_id;
            let is_included = snapshot.selected_targets.contains(&pane_id);

            pane_list = pane_list.child(
                div()
                    .h(px(24.0))
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(self.tokens.radii.sm))
                    .cursor_pointer()
                    .bg(if is_included {
                        rgba((theme.accent << 8) | 0x14)
                    } else {
                        rgba(0x00000000)
                    })
                    .hover(move |s| s.bg(rgba((theme.bg_hover << 8) | 0x80)))
                    .child(Self::render_lucide_icon(
                        if is_included {
                            LucideIcon::CheckSquare
                        } else {
                            LucideIcon::Square
                        },
                        12.0,
                        if is_included {
                            rgb(theme.accent)
                        } else {
                            rgb(theme.text_muted)
                        },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_size(px(self.tokens.metrics.ui_text_xs))
                            .text_color(if is_included {
                                rgb(theme.text)
                            } else {
                                rgb(theme.text_muted)
                            })
                            .child(label.clone()),
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

        let mut popover = div()
            .absolute()
            .bottom(px(38.0))
            .min_w(px(160.0))
            .max_w(px(240.0))
            .max_h(px(260.0))
            .rounded(px(self.tokens.radii.lg))
            .border_1()
            .border_color(rgb(theme.border))
            .bg(rgba((theme.bg_elevated << 8) | 0xfc))
            .shadow_lg()
            .p(px(6.0))
            .flex()
            .flex_col()
            .gap(px(4.0));

        if let Some(left) = anchor_left {
            popover = popover.left(px(left));
        } else {
            popover = popover.left(px(80.0));
        }

        context_menu_event_boundary(popover)
            .child(header_row)
            .child(
                div()
                    .h(px(1.0))
                    .w_full()
                    .bg(rgba((theme.border << 8) | 0x60)),
            )
            .child(pane_list)
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

    fn render_terminal_composer_text_counter(
        &self,
        label: String,
        value: u64,
        tone: MonospaceDatumTone,
    ) -> AnyElement {
        // Read-only counters stay plain text inside the sunken status field
        // so they never read as separate interactive chips.
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(3.0))
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(self.tokens.ui.text_muted))
                    .child(label),
            )
            .child(
                monospace_datum(
                    &self.tokens,
                    value.to_string(),
                    Some(settings_mono_font_family(self.settings_store.settings())),
                    MonospaceDatumOptions::new(tone).text_size(10.0).strong(),
                )
                .whitespace_nowrap(),
            )
            .into_any_element()
    }

    fn render_terminal_composer_interval_capsule(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let theme = self.tokens.ui;
        let is_editing = self
            .terminal_composer_value_edit
            .as_ref()
            .is_some_and(|edit| edit.field == TerminalComposerEditField::Interval);

        let content = if is_editing {
            let target = WorkspaceImeTarget::TerminalComposerValue;
            let edit_draft = self
                .terminal_composer_value_edit
                .as_ref()
                .map(|e| e.draft.clone())
                .unwrap_or_default();

            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(Self::render_lucide_icon(
                    LucideIcon::Clock,
                    11.0,
                    rgb(theme.accent),
                ))
                .child(
                    div()
                        .w(px(36.0))
                        .child(self.text_input_with_workspace_ime(
                            target,
                            self.workspace_text_input_borderless(
                                target,
                                TextInputView {
                                    value: &edit_draft,
                                    placeholder: String::new(),
                                    focused: true,
                                    caret_visible: self.input_caret.visible(),
                                    secret: false,
                                    selected_all: false,
                                    selected_range: self.ime_selected_range_for_target(target, cx),
                                    marked_text: self.marked_text_for_target(target, cx),
                                },
                            ),
                            |_this, _cx| {},
                            cx,
                        )),
                )
                .child(
                    div()
                        .text_size(px(self.tokens.metrics.ui_text_xs))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(theme.accent))
                        .child("ms"),
                )
        } else {
            let interval_label = format!("{} ms", snapshot.interval_ms);
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(Self::render_lucide_icon(
                    LucideIcon::Clock,
                    11.0,
                    rgb(theme.text_muted),
                ))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(theme.text))
                        .child(interval_label),
                )
        };

        let pacing = snapshot.pacing;
        div()
            .id("terminal-composer-interval-capsule")
            .h(px(24.0))
            .px(px(7.0))
            .flex()
            .items_center()
            .rounded(px(self.tokens.radii.sm))
            .border_1()
            .border_color(if is_editing {
                rgb(theme.accent)
            } else {
                rgba((theme.border << 8) | 0x80)
            })
            .bg(if is_editing {
                rgba((theme.accent << 8) | 0x14)
            } else {
                rgba((theme.bg << 8) | 0x60)
            })
            .cursor_pointer()
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .when(!is_editing, |capsule| {
                capsule.hover(|s| s.bg(rgba((theme.bg_hover << 8) | 0x80)))
            })
            .child(content)
            .on_scroll_wheel(cx.listener(move |this, event: &gpui::ScrollWheelEvent, _window, cx| {
                let vertical = f32::from(event.delta.pixel_delta(px(16.0)).y);
                if vertical.abs() > 0.5 {
                    let base_step = match pacing {
                        TerminalSendPacing::Line => 100,
                        TerminalSendPacing::Character => 10,
                    };
                    let step = if event.modifiers.shift {
                        base_step * 10
                    } else {
                        base_step
                    };
                    // Positive wheel delta scrolls up, which should grow the interval.
                    let delta_ms = if vertical > 0.0 { step } else { -step };
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.adjust_interval(document_id, delta_ms, cx);
                    });
                    if let Some(edit) = this.terminal_composer_value_edit.as_mut() {
                        if edit.field == TerminalComposerEditField::Interval {
                            if let Some(snap) = this
                                .terminal_command_composer
                                .read(cx)
                                .active_document_snapshot()
                            {
                                edit.draft = snap.interval_ms.to_string();
                            }
                        }
                    }
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event, window, cx| {
                    if !is_editing {
                        this.begin_terminal_composer_value_edit(
                            TerminalComposerEditField::Interval,
                            window,
                            cx,
                        );
                    }
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    fn render_terminal_composer_repeat_capsule(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_id = snapshot.id;
        let theme = self.tokens.ui;
        let is_editing = self
            .terminal_composer_value_edit
            .as_ref()
            .is_some_and(|edit| edit.field == TerminalComposerEditField::Repeat);

        let content = if is_editing {
            let target = WorkspaceImeTarget::TerminalComposerValue;
            let edit_draft = self
                .terminal_composer_value_edit
                .as_ref()
                .map(|e| e.draft.clone())
                .unwrap_or_default();

            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(Self::render_lucide_icon(
                    LucideIcon::RefreshCw,
                    10.0,
                    rgb(theme.accent),
                ))
                .child(
                    div()
                        .w(px(30.0))
                        .child(self.text_input_with_workspace_ime(
                            target,
                            self.workspace_text_input_borderless(
                                target,
                                TextInputView {
                                    value: &edit_draft,
                                    placeholder: String::new(),
                                    focused: true,
                                    caret_visible: self.input_caret.visible(),
                                    secret: false,
                                    selected_all: false,
                                    selected_range: self.ime_selected_range_for_target(target, cx),
                                    marked_text: self.marked_text_for_target(target, cx),
                                },
                            ),
                            |_this, _cx| {},
                            cx,
                        )),
                )
                .child(
                    div()
                        .text_size(px(self.tokens.metrics.ui_text_xs))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(theme.accent))
                        .child(self.i18n.t("terminal.composer.repeat_unit")),
                )
        } else {
            let repeat_label = if snapshot.repeat_count == 0 {
                "♾️".to_string()
            } else {
                self.i18n
                    .t("terminal.composer.repeat_count_format")
                    .replace("{{count}}", &snapshot.repeat_count.to_string())
            };

            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(Self::render_lucide_icon(
                    LucideIcon::RefreshCw,
                    10.0,
                    rgb(theme.text_muted),
                ))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(theme.text))
                        .child(repeat_label),
                )
        };

        div()
            .id("terminal-composer-repeat-capsule")
            .h(px(24.0))
            .px(px(7.0))
            .flex()
            .items_center()
            .rounded(px(self.tokens.radii.sm))
            .border_1()
            .border_color(if is_editing {
                rgb(theme.accent)
            } else {
                rgba((theme.border << 8) | 0x80)
            })
            .bg(if is_editing {
                rgba((theme.accent << 8) | 0x14)
            } else {
                rgba((theme.bg << 8) | 0x60)
            })
            .cursor_pointer()
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .when(!is_editing, |capsule| {
                capsule.hover(|s| s.bg(rgba((theme.bg_hover << 8) | 0x80)))
            })
            .child(content)
            .on_scroll_wheel(cx.listener(move |this, event: &gpui::ScrollWheelEvent, _window, cx| {
                let vertical = f32::from(event.delta.pixel_delta(px(16.0)).y);
                if vertical.abs() > 0.5 {
                    let step = if event.modifiers.shift { 5 } else { 1 };
                    // Positive wheel delta scrolls up, which should grow the repeat count.
                    let delta_step = if vertical > 0.0 { step } else { -step };
                    this.terminal_command_composer.update(cx, |composer, cx| {
                        composer.adjust_repeat_count(document_id, delta_step, cx);
                    });
                    if let Some(edit) = this.terminal_composer_value_edit.as_mut() {
                        if edit.field == TerminalComposerEditField::Repeat {
                            if let Some(snap) = this
                                .terminal_command_composer
                                .read(cx)
                                .active_document_snapshot()
                            {
                                edit.draft = snap.repeat_count.to_string();
                            }
                        }
                    }
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event, window, cx| {
                    if !is_editing {
                        this.begin_terminal_composer_value_edit(
                            TerminalComposerEditField::Repeat,
                            window,
                            cx,
                        );
                    }
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    fn terminal_composer_run_button(
        &self,
        document_id: TerminalCommandComposerId,
        running: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
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
        let (border, background, hover_background, text_color) = if running {
            (
                rgba((theme.error << 8) | 0x80),
                rgba((theme.error << 8) | 0x14),
                rgba((theme.error << 8) | 0x24),
                rgb(theme.error),
            )
        } else {
            (
                rgba((theme.border << 8) | 0x80),
                rgba((theme.bg << 8) | 0x60),
                rgba((theme.bg_hover << 8) | 0x80),
                rgb(theme.text),
            )
        };
        // The run control shares the 24px capsule chrome with the +/x task
        // buttons. Its width matches the + and x capsules plus the 4px gap so
        // the whole task-action group keeps a stable footprint.
        div()
            .h(px(24.0))
            .w(px(52.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(self.tokens.radii.sm))
            .border_1()
            .border_color(border)
            .bg(background)
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(text_color)
            .cursor_pointer()
            .hover(move |button| button.bg(hover_background))
            .child(div().min_w_0().truncate().child(label))
            .id("terminal-composer-run-button")
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
        self.i18n.t(match snapshot.status {
            TerminalCommandComposerStatus::Idle => "terminal.composer.idle",
            TerminalCommandComposerStatus::Running => "terminal.composer.running",
            TerminalCommandComposerStatus::Completed => "terminal.composer.completed",
            TerminalCommandComposerStatus::Stopped => "terminal.composer.stopped",
            TerminalCommandComposerStatus::Failed => "terminal.composer.failed",
        })
    }

    fn terminal_command_composer_failure_label(
        &self,
        snapshot: &TerminalCommandComposerDocumentSnapshot,
    ) -> Option<String> {
        snapshot.failure.map(|failure| {
            self.i18n.t(match failure {
                TerminalCommandComposerFailure::EmptyInput => "terminal.composer.error_empty",
                TerminalCommandComposerFailure::InvalidHex => "terminal.composer.error_hex",
                TerminalCommandComposerFailure::NoTargets => "terminal.composer.error_targets",
                TerminalCommandComposerFailure::TargetBusy => "terminal.composer.error_busy",
            })
        })
    }

    pub(in crate::workspace) fn terminal_command_composer_target_entries(
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

    pub(in crate::workspace) fn terminal_command_target_panes(&self, cx: &App) -> Vec<PaneId> {
        let Some(active_pane_id) = self.active_pane_id(cx) else {
            return Vec::new();
        };
        let Some(snapshot) = self
            .terminal_command_composer
            .read(cx)
            .active_document_snapshot()
        else {
            return vec![active_pane_id];
        };
        let entries = self.terminal_command_composer_target_entries(cx);
        let live_panes: Vec<PaneId> = entries.into_iter().map(|(t, _, _)| t.pane_id).collect();

        match snapshot.target_scope {
            TerminalCommandComposerTargetScope::Current => vec![active_pane_id],
            TerminalCommandComposerTargetScope::Selected => {
                let selected: Vec<PaneId> = live_panes
                    .into_iter()
                    .filter(|pane_id| snapshot.selected_targets.contains(pane_id))
                    .collect();
                if selected.is_empty() {
                    vec![active_pane_id]
                } else {
                    selected
                }
            }
        }
    }

    pub(in crate::workspace) fn start_terminal_command_composer(
        &mut self,
        document_id: TerminalCommandComposerId,
        cx: &mut Context<Self>,
    ) -> bool {
        let current_pane_id = self.active_pane_id(cx);
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

