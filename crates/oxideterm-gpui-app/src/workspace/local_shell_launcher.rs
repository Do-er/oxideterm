use super::*;

use oxideterm_gpui_ui::modal::{dismissible_dialog_backdrop, overlay_content_boundary};

const LOCAL_SHELL_LAUNCHER_WIDTH: f32 = 480.0; // Match the Tauri shell launcher dialog width.
const LOCAL_SHELL_LAUNCHER_LIST_MAX_HEIGHT: f32 = 300.0; // Keep long shell lists inside the modal.

impl WorkspaceApp {
    pub(in crate::workspace) fn open_local_shell_launcher(&mut self, cx: &mut Context<Self>) {
        let settings = self.settings_store.settings();
        let shells = self.effective_local_shells_for_settings(settings);
        self.local_shell_launcher_selected_id = settings
            .local_terminal
            .default_shell_id
            .as_ref()
            .filter(|id| shells.iter().any(|shell| &shell.id == *id))
            .cloned()
            .or_else(|| shells.first().map(|shell| shell.id.clone()));
        self.local_shell_launcher_open = true;
        cx.notify();
    }

    fn close_local_shell_launcher(&mut self, cx: &mut Context<Self>) {
        self.local_shell_launcher_open = false;
        self.local_shell_launcher_selected_id = None;
        cx.notify();
    }

    pub(in crate::workspace) fn handle_local_shell_launcher_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.local_shell_launcher_open {
            return false;
        }
        let settings = self.settings_store.settings();
        let shells = self.effective_local_shells_for_settings(settings);
        match event.keystroke.key.as_str() {
            "escape" => self.close_local_shell_launcher(cx),
            "enter" => self.launch_selected_local_shell(window, cx),
            "up" | "arrowup" => {
                if !shells.is_empty() {
                    let current_idx = self
                        .local_shell_launcher_selected_id
                        .as_deref()
                        .and_then(|id| shells.iter().position(|s| s.id == id))
                        .unwrap_or(0);
                    let new_idx = if current_idx == 0 {
                        shells.len() - 1
                    } else {
                        current_idx - 1
                    };
                    self.local_shell_launcher_selected_id = Some(shells[new_idx].id.clone());
                    cx.notify();
                }
            }
            "down" | "arrowdown" => {
                if !shells.is_empty() {
                    let current_idx = self
                        .local_shell_launcher_selected_id
                        .as_deref()
                        .and_then(|id| shells.iter().position(|s| s.id == id))
                        .unwrap_or(0);
                    let new_idx = (current_idx + 1) % shells.len();
                    self.local_shell_launcher_selected_id = Some(shells[new_idx].id.clone());
                    cx.notify();
                }
            }
            _ => {}
        }
        true
    }

    fn launch_local_shell(
        &mut self,
        shell_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settings = self.settings_store.settings();
        let Some(shell) = self
            .effective_local_shells_for_settings(settings)
            .into_iter()
            .find(|shell| shell.id == shell_id)
        else {
            return;
        };

        let mut terminal_config = self.local_terminal_config();
        terminal_config.shell = Some(shell.clone());
        self.edit_settings(
            |settings| {
                let recent = &mut settings.local_terminal.recent_shell_ids;
                recent.retain(|id| id != &shell.id);
                recent.insert(0, shell.id.clone());
                recent.truncate(5);
            },
            cx,
        );
        self.local_shell_launcher_open = false;
        self.local_shell_launcher_selected_id = None;
        let _ =
            self.create_local_terminal_tab_with_config(terminal_config, shell.label, window, cx);
    }

    fn launch_selected_local_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_id) = self.local_shell_launcher_selected_id.clone() {
            self.launch_local_shell(&selected_id, window, cx);
        }
    }

    pub(in crate::workspace) fn render_local_shell_launcher(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let settings = self.settings_store.settings();
        let default_shell_id = settings.local_terminal.default_shell_id.clone();
        let selected_shell_id = self.local_shell_launcher_selected_id.clone();
        let shells = self.effective_local_shells_for_settings(settings);

        let mut shell_list = div()
            .w_full()
            .max_h(px(LOCAL_SHELL_LAUNCHER_LIST_MAX_HEIGHT))
            .overflow_y_scrollbar()
            .flex()
            .flex_col()
            .gap(px(4.0));
        for shell in shells {
            let shell_id = shell.id.clone();
            let selected = selected_shell_id.as_deref() == Some(shell.id.as_str());
            let is_default = default_shell_id.as_deref() == Some(shell.id.as_str());
            let shell_id_for_launch = shell_id.clone();
            let shell_id_for_default = shell_id.clone();
            let tokens = self.tokens.clone();
            let star_tooltip = if is_default {
                self.i18n.t("settings_view.local_terminal.default")
            } else {
                self.i18n.t("settings_view.ai.profile_set_default")
            };
            shell_list = shell_list.child(
                div()
                    .w_full()
                    .min_w_0()
                    .rounded(px(self.tokens.radii.md))
                    .border_1()
                    .border_color(if selected {
                        rgb(theme.accent)
                    } else {
                        rgba(theme.border << 8)
                    })
                    .bg(if selected {
                        rgba((theme.accent << 8) | 0x1a)
                    } else {
                        rgba(theme.bg_hover << 8)
                    })
                    .hover(move |style| style.bg(rgb(theme.bg_hover)))
                    .px(px(12.0))
                    .py(px(10.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .cursor_pointer()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .child(Self::render_lucide_icon(
                                LucideIcon::Terminal,
                                18.0,
                                rgb(if selected {
                                    theme.accent
                                } else {
                                    theme.text_muted
                                }),
                            ))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .text_size(px(self.tokens.metrics.ui_text_sm))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(theme.text))
                                            .child(shell.label)
                                            .when(is_default, |label| {
                                                label.child(self.text_badge(
                                                    self.i18n.t("settings_view.local_terminal.default"),
                                                    theme.warning,
                                                ))
                                            }),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(self.tokens.metrics.ui_text_xs))
                                            .text_color(rgb(theme.text_muted))
                                            .child(shell.path.display().to_string()),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!("shell-star-{}", shell_id)))
                            .w(px(28.0))
                            .h(px(28.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(self.tokens.radii.sm))
                            .cursor_pointer()
                            .when(is_default, |this| {
                                this.text_color(rgb(theme.warning))
                            })
                            .when(!is_default, |this| {
                                this.text_color(rgba((theme.text_muted << 8) | 0x80))
                                    .hover(move |style| {
                                        style
                                            .bg(rgb(theme.bg_hover))
                                            .text_color(rgb(theme.warning))
                                    })
                            })
                            .tooltip(move |_, cx| {
                                oxideterm_gpui_ui::tooltip::tooltip_view(
                                    tokens.clone(),
                                    star_tooltip.clone(),
                                    None,
                                    cx,
                                )
                            })
                            .child(Self::render_lucide_icon(
                                LucideIcon::Star,
                                16.0,
                                rgb(if is_default {
                                    theme.warning
                                } else {
                                    theme.text_muted
                                }),
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, _window, cx| {
                                    let id = shell_id_for_default.clone();
                                    this.edit_settings(
                                        |settings| {
                                            settings.local_terminal.default_shell_id = Some(id);
                                        },
                                        cx,
                                    );
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            ),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, window, cx| {
                            this.launch_local_shell(&shell_id_for_launch, window, cx);
                            cx.stop_propagation();
                        }),
                    ),
            );
        }

        dismissible_dialog_backdrop()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.close_local_shell_launcher(cx);
                    cx.stop_propagation();
                }),
            )
            .child(overlay_content_boundary(
                div()
                    .w(px(LOCAL_SHELL_LAUNCHER_WIDTH))
                    .max_w_full()
                    .rounded(px(self.tokens.radii.lg))
                    .border_1()
                    .border_color(rgb(theme.border))
                    .bg(rgb(theme.bg_panel))
                    .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&self.tokens))
                    .p(px(16.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(self.tokens.metrics.ui_text_base))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(theme.text_heading))
                            .child(self.i18n.t("settings_view.local_terminal.select_shell")),
                    )
                    .child(shell_list),
            ))
            .into_any_element()
    }
}
