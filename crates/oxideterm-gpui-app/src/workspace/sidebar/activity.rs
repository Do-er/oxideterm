use super::*;
use gpui::StatefulInteractiveElement;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

impl WorkspaceApp {

    pub(in crate::workspace) fn render_context_activity_bar(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let panel_visible = self.context_sidebar_visible();
        let toggle_label = self.i18n.t(if panel_visible {
            "sidebar.actions.collapse"
        } else {
            "sidebar.actions.expand"
        });
        div()
            .id("context-activity-bar")
            .flex_none()
            .w(px(self.tokens.metrics.activity_bar_width))
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .when(self.window_background_preferences().is_none(), |bar| {
                bar.border_l_1()
                    .border_color(self.workspace_divider_color(theme.border, 0x40))
            })
            .child(
                div()
                    .flex_none()
                    .w_full()
                    .h(px(self.tokens.metrics.tabbar_height))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(self.workspace_chrome_background(theme.bg))
                    .border_b_1()
                    .border_color(self.workspace_chrome_divider())
                    .child(
                        oxideterm_gpui_ui::button::icon_button(
                            &self.tokens,
                            Self::render_lucide_icon(
                                if panel_visible {
                                    LucideIcon::PanelRightClose
                                } else {
                                    LucideIcon::PanelRight
                                },
                                self.tokens.metrics.sidebar_collapse_icon_size,
                                rgb(theme.text_muted),
                            ),
                            oxideterm_gpui_ui::button::IconButtonOptions {
                                radius: oxideterm_gpui_ui::button::ButtonRadius::Md,
                                hover_background: Some(rgb(theme.bg_hover)),
                                idle_opacity: 1.0,
                                ..oxideterm_gpui_ui::button::IconButtonOptions::compact(
                                    self.tokens.metrics.activity_icon_size,
                                )
                            },
                        )
                        .id("context-sidebar-toggle")
                        .on_mouse_move(cx.listener(
                            move |this, event: &MouseMoveEvent, _window, cx| {
                                this.queue_workspace_tooltip(
                                    "context-sidebar-toggle",
                                    toggle_label.clone(),
                                    f32::from(event.position.x) + 12.0,
                                    f32::from(event.position.y) + 16.0,
                                    cx,
                                );
                            },
                        ))
                        .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                            if !*hovered {
                                this.clear_workspace_tooltip("context-sidebar-toggle", cx);
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _event, _window, cx| {
                                this.clear_workspace_tooltip("context-sidebar-toggle", cx);
                                let _ = this.toggle_context_sidebar_panel(
                                    this.active_context_sidebar_panel,
                                    cx,
                                );
                                cx.stop_propagation();
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .bg(self.workspace_sidebar_background(theme.bg))
                    .child(self.render_context_tool_buttons(window, cx))
                    .when(
                        self.settings_store.settings().sidebar_ui.show_app_lock_icon,
                        |body| {
                            body.child(
                                div()
                                    .flex_none()
                                    .pt(px(self.tokens.spacing.two))
                                    .pb(px(self.tokens.spacing.two))
                                    .mb(px(self.tokens.metrics.activity_icon_gap))
                                    .child(self.render_app_lock_button(
                                        self.tokens.metrics.activity_icon_size,
                                        cx,
                                    )),
                            )
                        },
                    ),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn render_activity_icon(
        &self,
        section: SidebarSection,
        icon: LucideIcon,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let active = match section {
            SidebarSection::Terminal => false,
            SidebarSection::Network => {
                self.active_content_tab(cx)
                    .is_some_and(|tab| tab.kind == TabKind::Runtime)
                    && self.host_tools.read(cx).active_runtime_section
                        == ConnectionRuntimeSection::Topology
            }
            SidebarSection::Assistant => self.ai_sidebar_visible(),
            SidebarSection::HostTools => {
                self.context_sidebar_visible()
                    && self.active_context_sidebar_panel == ContextSidebarPanel::HostTools
            }
            _ => self.active_sidebar_section == section,
        };
        let tooltip = self.activity_icon_tooltip(section);
        let tooltip_id = format!("activity-icon-{}", section.as_settings_key());
        let tooltip_id_for_move = tooltip_id.clone();
        let badge_count = if section == SidebarSection::Workspace {
            self.visible_local_terminal_session_count(cx)
                .saturating_add(self.detached_local_terminals.len())
                .min(u32::MAX as usize) as u32
        } else {
            0
        };
        let badge_color =
            if section == SidebarSection::Workspace && !self.detached_local_terminals.is_empty() {
                theme.warning
            } else {
                theme.accent
            };
        let badge_text_color = if badge_color == theme.accent {
            theme.accent_text
        } else {
            theme.bg
        };

        // Activity entries use the same static selected-card treatment as the
        // settings navigation while retaining the shared icon-button states.
        let button = oxideterm_gpui_ui::button::icon_button(
            &self.tokens,
            Self::render_lucide_icon(
                icon,
                self.tokens.metrics.activity_icon_glyph_size,
                rgb(if active { theme.accent } else { theme.text }),
            ),
            oxideterm_gpui_ui::button::IconButtonOptions {
                size: self.tokens.metrics.activity_icon_size,
                radius: oxideterm_gpui_ui::button::ButtonRadius::Md,
                has_background: active,
                background: active.then(|| self.settings_panel_background(theme.bg_panel)),
                border: active.then(|| rgb(theme.border)),
                hover_background: Some(rgb(theme.bg_hover)),
                idle_opacity: 1.0,
                ..oxideterm_gpui_ui::button::IconButtonOptions::compact(
                    self.tokens.metrics.activity_icon_size,
                )
            },
        );
        let button = if active {
            oxideterm_gpui_ui::theme_card_surface_shadow(button, &self.tokens)
        } else {
            button
        };

        button
            .id(("activity-icon", section as u64))
            .relative()
            .mb(px(self.tokens.metrics.activity_icon_gap))
            .when(badge_count > 0, |icon_el| {
                icon_el.child(
                    div()
                        .absolute()
                        // Tauri badges sit outside the `h-9 w-9` Button
                        // (`-top-1 -right-1`) so the active button chrome and
                        // numeric pill do not visually collide.
                        .right(px(-4.0))
                        .top(px(-4.0))
                        .min_w(px(14.0))
                        .h(px(14.0))
                        .px(px(3.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(rgb(badge_color))
                        .text_color(rgb(badge_text_color))
                        .text_size(px(9.0))
                        .child(if badge_count > 99 {
                            "99+".to_string()
                        } else {
                            badge_count.to_string()
                        }),
                )
            })
            .on_mouse_move(cx.listener({
                move |this, event: &MouseMoveEvent, _window, cx| {
                    this.queue_workspace_tooltip(
                        tooltip_id_for_move.clone(),
                        tooltip.clone(),
                        f32::from(event.position.x) + 12.0,
                        f32::from(event.position.y) + 16.0,
                        cx,
                    );
                }
            }))
            .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                if !*hovered {
                    this.clear_workspace_tooltip(&tooltip_id, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event, window, cx| {
                    if section == SidebarSection::Terminal {
                        this.open_connection_runtime_tab(
                            ConnectionRuntimeSection::Overview,
                            window,
                            cx,
                        );
                    } else if section == SidebarSection::Network {
                        this.open_topology_tab(window, cx);
                    } else if section == SidebarSection::Workspace {
                        // Tauri treats the bottom square as a local-terminal action.
                        if !this.detached_local_terminals.is_empty() {
                            this.detached_local_terminals_popover_open =
                                !this.detached_local_terminals_popover_open;
                            cx.notify();
                        } else {
                            let _ = this.create_local_terminal_tab(window, cx);
                        }
                    } else if section == SidebarSection::Assistant {
                        let _ = this.toggle_ai_sidebar(cx);
                    } else if section == SidebarSection::HostTools {
                        let _ =
                            this.toggle_context_sidebar_panel(ContextSidebarPanel::HostTools, cx);
                    } else {
                        this.active_surface = ActiveSurface::Terminal;
                        this.toggle_sidebar_section(section, cx);
                    }
                }),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn activity_icon_tooltip(&self, section: SidebarSection) -> String {
        match section {
            SidebarSection::Forwards => self.i18n.t("forwards.table.title"),
            SidebarSection::Terminal => self.i18n.t("sidebar.panels.runtime_overview"),
            SidebarSection::Network => self.i18n.t("sidebar.panels.connection_matrix"),
            SidebarSection::Assistant => self.i18n.t("sidebar.panels.ai"),
            SidebarSection::HostTools => self.i18n.t("sidebar.panels.host_tools"),
            SidebarSection::Automation => self.i18n.t("sidebar.panels.activity"),
            SidebarSection::Workspace => self.i18n.t("sidebar.actions.new_local_terminal"),
        }
    }

    /// A plugin panel is active only while the context drawer shows it, so the
    /// rail highlight and the panel body always describe the same surface.
    pub(in crate::workspace) fn plugin_sidebar_panel_visible(
        &self,
        plugin_id: &str,
        panel_id: &str,
        cx: &App,
    ) -> bool {
        self.context_sidebar_visible()
            && self.active_context_sidebar_panel == ContextSidebarPanel::PluginPanel
            && self
                .plugin_manager_state(cx)
                .active_sidebar_panel
                .as_ref()
                .is_some_and(|selection| {
                    selection.plugin_id == plugin_id && selection.panel_id == panel_id
                })
    }

    pub(in crate::workspace) fn render_plugin_sidebar_activity_icon(
        &self,
        panel: plugin_host::NativePluginRuntimeSidebarPanelContribution,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let selection = plugin_ui::NativePluginSidebarPanelSelection {
            plugin_id: panel.plugin_id.clone(),
            panel_id: panel.panel_id.clone(),
        };
        let active = self.plugin_sidebar_panel_visible(&panel.plugin_id, &panel.panel_id, cx);
        let tooltip = panel.title.clone();
        let tooltip_id = format!("activity-plugin-{}-{}", panel.plugin_id, panel.panel_id);
        let tooltip_id_for_move = tooltip_id.clone();
        let icon = LucideIcon::from_plugin_name(&panel.icon);

        let button = oxideterm_gpui_ui::button::icon_button(
            &self.tokens,
            Self::render_lucide_icon(
                icon,
                self.tokens.metrics.activity_icon_glyph_size,
                rgb(if active { theme.accent } else { theme.text }),
            ),
            oxideterm_gpui_ui::button::IconButtonOptions {
                size: self.tokens.metrics.activity_icon_size,
                radius: oxideterm_gpui_ui::button::ButtonRadius::Md,
                has_background: active,
                background: active.then(|| self.settings_panel_background(theme.bg_panel)),
                border: active.then(|| rgb(theme.border)),
                hover_background: Some(rgb(theme.bg_hover)),
                idle_opacity: 1.0,
                ..oxideterm_gpui_ui::button::IconButtonOptions::compact(
                    self.tokens.metrics.activity_icon_size,
                )
            },
        );
        let button = if active {
            oxideterm_gpui_ui::theme_card_surface_shadow(button, &self.tokens)
        } else {
            button
        };

        button
            .id((
                "activity-plugin-icon",
                native_plugin_sidebar_activity_id(&panel),
            ))
            .relative()
            .mb(px(self.tokens.metrics.activity_icon_gap))
            .on_mouse_move(cx.listener({
                move |this, event: &MouseMoveEvent, _window, cx| {
                    this.queue_workspace_tooltip(
                        tooltip_id_for_move.clone(),
                        tooltip.clone(),
                        f32::from(event.position.x) + 12.0,
                        f32::from(event.position.y) + 16.0,
                        cx,
                    );
                }
            }))
            .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                if !*hovered {
                    this.clear_workspace_tooltip(&tooltip_id, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    let requested_panel_is_visible = this.plugin_sidebar_panel_visible(
                        &selection.plugin_id,
                        &selection.panel_id,
                        cx,
                    );
                    if requested_panel_is_visible {
                        this.collapse_context_sidebar(cx);
                        cx.stop_propagation();
                        return;
                    }
                    // Choosing a plugin panel switches only the context drawer
                    // content; Plugin Manager remains a separate workspace tab.
                    // Selecting through the entity also drops the previous
                    // panel's UI state, including sensitive input drafts.
                    this.plugin_entity.update(cx, |plugins, _cx| {
                        plugins.select_sidebar_panel(selection.clone());
                    });
                    this.open_context_sidebar_panel(ContextSidebarPanel::PluginPanel, cx);
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn render_plugin_activity_action_icon(
        &self,
        item: plugin_host::NativePluginRuntimeActivityBarItemContribution,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let tooltip_id = format!("activity-plugin-action-{}-{}", item.plugin_id, item.item_id);
        let tooltip_id_for_move = tooltip_id.clone();
        let tooltip = item.title.clone();
        let plugin_id = item.plugin_id.clone();
        let command = item.command.clone();
        let icon = LucideIcon::from_plugin_name(&item.icon);
        let button = oxideterm_gpui_ui::button::icon_button(
            &self.tokens,
            Self::render_lucide_icon(
                icon,
                self.tokens.metrics.activity_icon_glyph_size,
                rgb(theme.text),
            ),
            oxideterm_gpui_ui::button::IconButtonOptions {
                size: self.tokens.metrics.activity_icon_size,
                radius: oxideterm_gpui_ui::button::ButtonRadius::Md,
                hover_background: Some(rgb(theme.bg_hover)),
                idle_opacity: 1.0,
                ..oxideterm_gpui_ui::button::IconButtonOptions::compact(
                    self.tokens.metrics.activity_icon_size,
                )
            },
        );

        button
            .id((
                "activity-plugin-action",
                native_plugin_activity_bar_item_id(&item),
            ))
            .relative()
            .mb(px(self.tokens.metrics.activity_icon_gap))
            .on_mouse_move(
                cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                    this.queue_workspace_tooltip(
                        tooltip_id_for_move.clone(),
                        tooltip.clone(),
                        f32::from(event.position.x) + 12.0,
                        f32::from(event.position.y) + 16.0,
                        cx,
                    );
                }),
            )
            .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                if !*hovered {
                    this.clear_workspace_tooltip(&tooltip_id, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    // Action items invoke only the manifest-declared command
                    // associated with their host-owned runtime registration.
                    this.dispatch_native_plugin_command(plugin_id.clone(), command.clone(), cx);
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn render_lucide_icon(
        icon: LucideIcon,
        size: f32,
        color: Rgba,
    ) -> AnyElement {
        svg()
            .path(icon.path())
            .size(px(size))
            .text_color(color)
            .into_any_element()
    }

    pub(in crate::workspace) fn render_animated_chevron(
        &self,
        id: impl Into<gpui::ElementId>,
        expanded: bool,
        size: f32,
        color: Rgba,
    ) -> AnyElement {
        // A single right-facing asset keeps state changes continuous instead
        // of swapping two unrelated SVG trees between renders.
        oxideterm_gpui_ui::motion::animated_chevron(
            &self.tokens,
            id,
            svg()
                .path(LucideIcon::ChevronRight.path())
                .size(px(size))
                .text_color(color),
            expanded,
        )
    }

    pub(in crate::workspace) fn render_loading_icon(
        &self,
        id: impl Into<gpui::ElementId>,
        size: f32,
        color: Rgba,
    ) -> AnyElement {
        // Spinner frames are requested only while callers render a genuine
        // loading state; static status icons continue using render_lucide_icon.
        oxideterm_gpui_ui::motion::animated_spinner(
            &self.tokens,
            id,
            svg()
                .path(LucideIcon::LoaderCircle.path())
                .size(px(size))
                .text_color(color),
        )
    }
}

pub(in crate::workspace) fn native_plugin_sidebar_activity_id(
    panel: &plugin_host::NativePluginRuntimeSidebarPanelContribution,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    panel.registration_id.hash(&mut hasher);
    panel.plugin_id.hash(&mut hasher);
    panel.panel_id.hash(&mut hasher);
    hasher.finish()
}

pub(in crate::workspace) fn native_plugin_activity_bar_item_id(
    item: &plugin_host::NativePluginRuntimeActivityBarItemContribution,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    item.registration_id.hash(&mut hasher);
    item.plugin_id.hash(&mut hasher);
    item.item_id.hash(&mut hasher);
    hasher.finish()
}
