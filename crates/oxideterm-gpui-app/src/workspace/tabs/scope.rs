use super::*;

const fn tab_cleanup_menu_should_open(trigger_hovered: bool, menu_hovered: bool) -> bool {
    trigger_hovered || menu_hovered
}

impl WorkspaceApp {
    pub(in crate::workspace) fn set_tab_cleanup_trigger_hovered(
        &mut self,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        if self.main_window_tabs.cleanup_trigger_hovered == hovered {
            return;
        }
        self.main_window_tabs.cleanup_trigger_hovered = hovered;
        self.sync_tab_cleanup_menu_hover_state(cx);
    }

    pub(in crate::workspace) fn set_tab_cleanup_menu_hovered(
        &mut self,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        if self.main_window_tabs.cleanup_menu_hovered == hovered {
            return;
        }
        self.main_window_tabs.cleanup_menu_hovered = hovered;
        self.sync_tab_cleanup_menu_hover_state(cx);
    }

    pub(in crate::workspace) fn open_tab_cleanup_menu(&mut self, cx: &mut Context<Self>) {
        self.main_window_tabs.cleanup_trigger_hovered = true;
        if !self.main_window_tabs.cleanup_menu_open {
            self.main_window_tabs.cleanup_menu_open = true;
            cx.notify();
        }
    }

    pub(in crate::workspace) fn close_tab_cleanup_menu(&mut self) -> bool {
        let was_open = self.main_window_tabs.cleanup_menu_open
            || self.main_window_tabs.cleanup_trigger_hovered
            || self.main_window_tabs.cleanup_menu_hovered;
        self.main_window_tabs.cleanup_menu_open = false;
        self.main_window_tabs.cleanup_trigger_hovered = false;
        self.main_window_tabs.cleanup_menu_hovered = false;
        was_open
    }

    fn sync_tab_cleanup_menu_hover_state(&mut self, cx: &mut Context<Self>) {
        let should_open = tab_cleanup_menu_should_open(
            self.main_window_tabs.cleanup_trigger_hovered,
            self.main_window_tabs.cleanup_menu_hovered,
        );
        if self.main_window_tabs.cleanup_menu_open != should_open {
            self.main_window_tabs.cleanup_menu_open = should_open;
        }
        cx.notify();
    }

    pub(in crate::workspace) fn tab_cleanup_label(&self, target: TabCleanupTarget) -> String {
        match target {
            TabCleanupTarget::Global => self.i18n.t("sidebar.tab_scope.global"),
            TabCleanupTarget::LocalTerminal => self.i18n.t("sidebar.tab_scope.local"),
            TabCleanupTarget::All => self.i18n.t("sidebar.tab_scope.all"),
        }
    }

    const fn tab_cleanup_icon(target: TabCleanupTarget) -> LucideIcon {
        match target {
            TabCleanupTarget::Global => LucideIcon::AppWindow,
            TabCleanupTarget::LocalTerminal => LucideIcon::Terminal,
            TabCleanupTarget::All => LucideIcon::Layers,
        }
    }

    pub(in crate::workspace) fn tab_cleanup_count(
        &self,
        target: TabCleanupTarget,
        cx: &App,
    ) -> usize {
        let host = self.tab_host.read(cx);
        match target {
            TabCleanupTarget::Global => host
                .unfiltered_main_tabs()
                .filter(|(_, tab)| {
                    self.tab_affinities(tab.id, cx)
                        .contains(&TabAffinity::Global)
                })
                .count(),
            TabCleanupTarget::LocalTerminal => host
                .unfiltered_main_tabs()
                .filter(|(_, tab)| {
                    self.tab_affinities(tab.id, cx)
                        .contains(&TabAffinity::LocalTerminal)
                })
                .count(),
            TabCleanupTarget::All => host.unfiltered_main_tabs().count(),
        }
    }

    pub(in crate::workspace) fn cleanup_tabs(
        &mut self,
        target: TabCleanupTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_tab_cleanup_menu();
        self.main_window_tabs.drag = None;

        match target {
            TabCleanupTarget::All => {
                let tab_ids = self
                    .tab_host
                    .read(cx)
                    .unfiltered_main_tabs()
                    .map(|(_, tab)| tab.id)
                    .collect::<Vec<_>>();
                for tab_id in tab_ids {
                    self.force_close_tab_by_id(tab_id, window, cx);
                }
            }
            TabCleanupTarget::Global => {
                let mut page_ids = Vec::new();
                for (_, tab) in self.tab_host.read(cx).unfiltered_main_tabs() {
                    for content in self.tab_host.read(cx).tab_contents(tab.id) {
                        if let TabContent::Page(page_id) = content
                            && self
                                .page_affinities(page_id, cx)
                                .contains(&TabAffinity::Global)
                            && !page_ids.contains(&page_id)
                        {
                            page_ids.push(page_id);
                        }
                    }
                }
                for page_id in page_ids {
                    self.force_close_tab_by_id(page_id, window, cx);
                }
            }
            TabCleanupTarget::LocalTerminal => {
                let mut full_tabs = Vec::new();
                let mut split_panes = Vec::new();
                for (_, tab) in self.tab_host.read(cx).unfiltered_main_tabs() {
                    let contents = self.tab_host.read(cx).tab_contents(tab.id);
                    if contents.is_empty() {
                        continue;
                    }
                    let all_local = contents.iter().all(|content| {
                        matches!(
                            content,
                            TabContent::Terminal(session_id)
                                if self
                                    .terminal_affinities(*session_id, cx)
                                    .contains(&TabAffinity::LocalTerminal)
                        )
                    });
                    if all_local {
                        full_tabs.push(tab.id);
                    } else if let Some(root) = &tab.root_pane {
                        let mut pane_ids = Vec::new();
                        root.collect_pane_ids(&mut pane_ids);
                        for pane_id in pane_ids {
                            if root.session_id_for_pane(pane_id).is_some_and(|session_id| {
                                self.terminal_affinities(session_id, cx)
                                    .contains(&TabAffinity::LocalTerminal)
                            }) {
                                split_panes.push((tab.id, pane_id));
                            }
                        }
                    }
                }
                for tab_id in full_tabs {
                    self.force_close_tab_by_id(tab_id, window, cx);
                }
                for (tab_id, pane_id) in split_panes {
                    self.force_close_terminal_pane_in_tab(tab_id, pane_id, window, cx);
                }
            }
        }
        cx.notify();
    }

    fn tab_cleanup_trigger_metrics(
        &self,
        window: &Window,
        available_width: f32,
    ) -> (String, f32) {
        let label = self.i18n.t("sidebar.tab_scope.clean_tabs");
        let width = (self.tab_text_width(&label, self.tokens.metrics.ui_text_sm, window)
            + self.tokens.metrics.ui_control_padding_x * 4.0)
            .clamp(96.0, 180.0)
            .min(available_width.max(0.0));
        (label, width)
    }

    pub(in crate::workspace) fn render_tab_scope_trigger(
        &self,
        window: &Window,
        available_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (label, width) = self.tab_cleanup_trigger_metrics(window, available_width);
        let menu_open = self.main_window_tabs.cleanup_menu_open;
        let hover_bg = rgb(self.tokens.ui.bg_hover);
        let workspace = cx.entity().downgrade();
        div()
            .id("titlebar-tab-scope")
            .w(px(width))
            .flex_none()
            .h(px(self.tokens.metrics.titlebar_height - 6.0))
            .rounded(px(self.tokens.radii.md))
            .flex()
            .items_center()
            .justify_center()
            .px(px(self.tokens.metrics.ui_control_padding_x * 1.5))
            .text_size(px(self.tokens.metrics.ui_text_sm))
            .text_color(rgb(self.tokens.ui.text))
            .bg(if menu_open {
                hover_bg
            } else {
                rgba(0x00000000)
            })
            .role(gpui::Role::Button)
            .aria_label(label.clone())
            .child(
                div()
                    .truncate()
                    .text_align(gpui::TextAlign::Center)
                    .child(label),
            )
            .hover(move |trigger| trigger.bg(hover_bg))
            .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                this.set_tab_cleanup_trigger_hovered(*hovered, cx);
            }))
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, _window, cx| {
                let _ = workspace.update(cx, |this, cx| {
                    this.open_tab_cleanup_menu(cx);
                });
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _window, cx| {
                    this.open_tab_cleanup_menu(cx);
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn render_tab_cleanup_menu(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.main_window_tabs.cleanup_menu_open || !self.window_titlebar_visible(window) {
            self.close_tab_cleanup_menu();
            return None;
        }
        let viewport_width = f32::from(window.viewport_size().width);
        let menu_width = 156.0;
        let left = ((viewport_width - menu_width) * 0.5).max(8.0);
        let top = (self.window_titlebar_height(window) - 4.0).max(0.0);

        let mut panel = oxideterm_gpui_ui::select::select_event_boundary(
            oxideterm_gpui_ui::menu::menu_content(&self.tokens)
                .w_full()
                .min_w(px(0.0)),
        );
        let hover_bg = rgb(self.tokens.ui.bg_hover);
        for target in TabCleanupTarget::ALL {
            let label = self.tab_cleanup_label(target);
            let count = self.tab_cleanup_count(target, cx);
            let icon = Self::tab_cleanup_icon(target);
            let workspace = cx.entity().downgrade();
            panel = panel.child(
                div()
                    .id(("titlebar-tab-cleanup-option", target as usize))
                    .w_full()
                    .h(px(self.tokens.metrics.ui_control_height))
                    .px(px(self.tokens.metrics.ui_menu_item_padding_x))
                    .rounded(px(self.tokens.radii.xs))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(self.tokens.spacing.two))
                    .text_size(px(self.tokens.metrics.ui_text_sm))
                    .text_color(rgb(self.tokens.ui.text))
                    .opacity(if count == 0 { 0.5 } else { 1.0 })
                    .cursor_pointer()
                    .hover(move |row| row.bg(hover_bg))
                    .role(gpui::Role::MenuItem)
                    .aria_label(label.clone())
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .items_center()
                            .gap(px(self.tokens.spacing.two))
                            .child(Self::render_lucide_icon(
                                icon,
                                14.0,
                                rgb(self.tokens.ui.text_muted),
                            ))
                            .child(div().flex_1().min_w(px(0.0)).truncate().child(label)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(self.tokens.metrics.ui_text_xs))
                            .text_color(rgb(self.tokens.ui.text_muted))
                            .child(count.to_string()),
                    )
                    .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                        let _ = workspace.update(cx, |this, cx| {
                            this.cleanup_tabs(target, window, cx);
                        });
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.cleanup_tabs(target, window, cx);
                            cx.stop_propagation();
                        }),
                    ),
            );
        }

        Some(
            div()
                .id("titlebar-tab-cleanup-menu")
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(menu_width))
                .pt(px(4.0))
                .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                    this.set_tab_cleanup_menu_hovered(*hovered, cx);
                }))
                .child(panel)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_cleanup_targets_keep_global_first_and_close_when_leaving_both_regions() {
        assert_eq!(
            TabCleanupTarget::ALL,
            [
                TabCleanupTarget::Global,
                TabCleanupTarget::LocalTerminal,
                TabCleanupTarget::All,
            ]
        );
        assert!(tab_cleanup_menu_should_open(true, false));
        assert!(tab_cleanup_menu_should_open(false, true));
        assert!(tab_cleanup_menu_should_open(true, true));
        assert!(!tab_cleanup_menu_should_open(false, false));
    }
}
