use super::*;

impl WorkspaceApp {
    pub(super) fn render_session_manager_toolbar(
        &self,
        window: &Window,
        has_background: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let (selected_count, search_query, is_tree_mode) = {
            let manager = self.session_manager.read(cx);
            (
                manager.selected_items.len(),
                manager.search_query.clone(),
                manager.view_mode == SessionManagerViewMode::Tree,
            )
        };
        let workspace = cx.entity();
        let _ = window;
        div()
            .h(px(36.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .px_2()
            .border_b_1()
            .border_color(self.workspace_chrome_divider())
            .bg(theme_bg(theme.bg, has_background))
            // Search Input
            .child(
                div()
                    .flex_1()
                    .min_w(px(60.0))
                    .child(self.render_session_text_input(
                        SessionManagerInput::Search,
                        &search_query,
                        self.i18n.t("sessionManager.toolbar.search_placeholder"),
                        cx,
                    )),
            )
            // New Connection (Icon only)
            .child(self.workspace_tooltip_icon_button(
                LucideIcon::Plus,
                14.0,
                rgb(self.tokens.ui.text),
                IconButtonOptions {
                    radius: ButtonRadius::Md,
                    hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                    idle_opacity: 1.0,
                    ..IconButtonOptions::compact(24.0)
                },
                self.i18n.t("sessionManager.toolbar.new_connection"),
                "session-new-connection",
                true,
                cx.listener(|this, _event, window, cx| {
                    this.open_new_connection_form(window, cx);
                    cx.stop_propagation();
                }),
                cx.entity(),
            ))
            // New Group (Icon only)
            .child(self.workspace_tooltip_icon_button(
                LucideIcon::FolderPlus,
                14.0,
                rgb(self.tokens.ui.text),
                IconButtonOptions {
                    radius: ButtonRadius::Md,
                    hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                    idle_opacity: 1.0,
                    ..IconButtonOptions::compact(24.0)
                },
                self.i18n.t("sessionManager.folder_tree.new_group"),
                "session-new-group",
                true,
                cx.listener(|this, _event, _window, cx| {
                    this.open_session_group_creation(cx);
                    cx.stop_propagation();
                }),
                cx.entity(),
            ))
            // When in Tree mode: Expand All & Collapse All
            .when(is_tree_mode, |toolbar| {
                toolbar
                    .child(self.workspace_tooltip_icon_button(
                        LucideIcon::ChevronDown,
                        14.0,
                        rgb(self.tokens.ui.text),
                        IconButtonOptions {
                            radius: ButtonRadius::Md,
                            hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                            idle_opacity: 1.0,
                            ..IconButtonOptions::compact(24.0)
                        },
                        self.i18n.t("sessionManager.views.expand_all"),
                        "session-expand-all",
                        true,
                        cx.listener(|this, _event, _window, cx| {
                            let (roots, children) = this.session_group_tree();
                            let mut groups = HashSet::new();
                            collect_session_group_paths(&roots, &children, &mut groups);
                            this.session_manager.update(cx, |manager, cx| {
                                manager.expanded_groups = groups;
                                cx.notify();
                            });
                            cx.stop_propagation();
                        }),
                        cx.entity(),
                    ))
                    .child(self.workspace_tooltip_icon_button(
                        LucideIcon::ChevronRight,
                        14.0,
                        rgb(self.tokens.ui.text),
                        IconButtonOptions {
                            radius: ButtonRadius::Md,
                            hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                            idle_opacity: 1.0,
                            ..IconButtonOptions::compact(24.0)
                        },
                        self.i18n.t("sessionManager.views.collapse_all"),
                        "session-collapse-all",
                        true,
                        cx.listener(|this, _event, _window, cx| {
                            this.session_manager.update(cx, |manager, cx| {
                                manager.expanded_groups.clear();
                                cx.notify();
                            });
                            cx.stop_propagation();
                        }),
                        cx.entity(),
                    ))
            })
            // Toggle View Mode (Tree <-> List)
            .child(self.workspace_tooltip_icon_button(
                if is_tree_mode {
                    LucideIcon::LayoutList
                } else {
                    LucideIcon::ListTree
                },
                14.0,
                rgb(self.tokens.ui.text),
                IconButtonOptions {
                    radius: ButtonRadius::Md,
                    hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                    idle_opacity: 1.0,
                    ..IconButtonOptions::compact(24.0)
                },
                if is_tree_mode {
                    self.i18n.t("sessionManager.views.list")
                } else {
                    self.i18n.t("sessionManager.views.tree")
                },
                "session-toggle-view-mode",
                true,
                cx.listener(|this, _event, _window, cx| {
                    this.session_manager.update(cx, |manager, cx| {
                        manager.view_mode = match manager.view_mode {
                            SessionManagerViewMode::Tree => SessionManagerViewMode::List,
                            _ => SessionManagerViewMode::Tree,
                        };
                        cx.notify();
                    });
                    cx.stop_propagation();
                }),
                cx.entity(),
            ))
            // More actions (SSH Config import & Connection Importers)
            .child(self.render_session_manager_more_menu_trigger(has_background, cx))
            // Batch Actions (when items are selected)
            .when(selected_count > 0, |toolbar| {
                toolbar.child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .flex_none()
                                .px_1()
                                .text_size(px(self.tokens.metrics.ui_text_xs))
                                .text_color(rgb(theme.text_muted))
                                .child(selected_count_label(&self.i18n, selected_count)),
                        )
                        .child(select_anchor_probe(
                            SelectAnchorId::SessionManagerBatchMove,
                            self.workspace_tooltip_icon_button(
                                LucideIcon::FolderInput,
                                14.0,
                                rgb(self.tokens.ui.text),
                                IconButtonOptions {
                                    radius: ButtonRadius::Md,
                                    hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                                    idle_opacity: 1.0,
                                    ..IconButtonOptions::compact(24.0)
                                },
                                self.i18n.t("sessionManager.batch.move_to_group"),
                                "session-batch-move",
                                true,
                                cx.listener(|this, _event, _window, cx| {
                                    this.session_manager.update(cx, |manager, cx| {
                                        manager.show_batch_move = !manager.show_batch_move;
                                        cx.notify();
                                    });
                                    cx.stop_propagation();
                                }),
                                cx.entity(),
                            ),
                            move |anchor, _window, cx| {
                                let _ = workspace.update(cx, |this, cx| {
                                    this.update_select_anchor(anchor, cx);
                                });
                            },
                        ))
                        .child(self.workspace_tooltip_icon_button(
                            LucideIcon::Trash2,
                            14.0,
                            rgb(self.tokens.ui.error),
                            IconButtonOptions {
                                radius: ButtonRadius::Md,
                                hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                                idle_opacity: 1.0,
                                ..IconButtonOptions::compact(24.0)
                            },
                            self.i18n.t("sessionManager.batch.delete"),
                            "session-batch-delete",
                            true,
                            cx.listener(|this, _event, _window, cx| {
                                this.request_delete_selected_connections(cx);
                                cx.stop_propagation();
                            }),
                            cx.entity(),
                        )),
                )
            })
            .into_any_element()
    }

    pub(super) fn render_session_manager_more_menu_trigger(
        &self,
        _has_background: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let workspace = cx.entity();
        let trigger = self.workspace_tooltip_icon_button(
            LucideIcon::MoreVertical,
            14.0,
            rgb(self.tokens.ui.text),
            IconButtonOptions {
                radius: ButtonRadius::Md,
                hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                idle_opacity: 1.0,
                ..IconButtonOptions::compact(24.0)
            },
            self.i18n.t("sidebar.panels.more_tools"),
            "session-more-menu",
            true,
            cx.listener(move |this, _event, _window, cx| {
                this.session_manager.update(cx, |manager, cx| {
                    manager.sort_menu_open = !manager.sort_menu_open;
                    cx.notify();
                });
                cx.stop_propagation();
            }),
            cx.entity(),
        );

        select_anchor_probe(
            SelectAnchorId::SessionManagerSort,
            trigger,
            move |anchor, _window, cx| {
                let _ = workspace.update(cx, |this, cx| {
                    this.update_select_anchor(anchor, cx);
                });
            },
        )
        .into_any_element()
    }

    pub(super) fn render_session_manager_view_mode_menu(
        &self,
        _window: &Window,
        _has_background: bool,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        div().into_any_element()
    }

    pub(super) fn render_session_manager_sort_menu(
        &self,
        window: &Window,
        has_background: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(anchor) = self
            .select_anchors
            .get(&SelectAnchorId::SessionManagerSort)
            .copied()
        else {
            return div().into_any_element();
        };
        let viewport = window.viewport_size();
        let menu_width = 180.0;
        let menu_height = 80.0;
        let placement = browser_behavior::clamp_context_menu_position(
            f32::from(anchor.bounds.right()) - menu_width,
            f32::from(anchor.bounds.bottom()) + 4.0,
            f32::from(viewport.width),
            f32::from(viewport.height),
            menu_width,
            menu_height,
            8.0,
        );
        let mut menu = context_menu_event_boundary(
            dropdown_menu_content(&self.tokens).w(px(menu_width)),
        );
        let ssh_config_item = dropdown_menu_item(
            &self.tokens,
            self.i18n.t("settings_view.connections.ssh_config.title"),
            DropdownMenuItemKind::Plain,
            false,
            false,
        );
        menu = menu.child(self.render_session_manager_menu_action(
            ssh_config_item,
            false,
            false,
            has_background,
            |this, _event, _window, cx| {
                this.session_manager.update(cx, |m, cx| {
                    m.sort_menu_open = false;
                    cx.notify();
                });
                this.close_session_row_menus(cx);
                this.open_settings_ssh_config_import_dialog(cx);
                cx.stop_propagation();
            },
            cx,
        ));
        let importers_item = dropdown_menu_item(
            &self.tokens,
            self.i18n.t("settings_view.connections.importers.title"),
            DropdownMenuItemKind::Plain,
            false,
            false,
        );
        menu = menu.child(self.render_session_manager_menu_action(
            importers_item,
            false,
            false,
            has_background,
            |this, _event, window, cx| {
                this.session_manager.update(cx, |m, cx| {
                    m.sort_menu_open = false;
                    cx.notify();
                });
                this.close_session_row_menus(cx);
                this.open_connection_importers_settings(window, cx);
                cx.stop_propagation();
            },
            cx,
        ));
        deferred(
            anchored()
                .anchor(Corner::TopLeft)
                .position(gpui::point(px(placement.x), px(placement.y)))
                .position_mode(AnchoredPositionMode::Window)
                .child(overlay_content_boundary(menu)),
        )
        .with_priority(oxideterm_gpui_ui::modal::TAURI_POPOVER_LAYER_PRIORITY)
        .into_any_element()
    }
}
