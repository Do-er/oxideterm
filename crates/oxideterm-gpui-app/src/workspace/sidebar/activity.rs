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
            .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                this.set_context_sidebar_activity_bar_hovered(*hovered, cx);
            }))
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
                if *hovered {
                    this.clear_all_workspace_tooltips(cx);
                }
                this.set_context_sidebar_activity_hovered(
                    (*hovered).then_some(ContextSidebarPanel::PluginPanel),
                    cx,
                );
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    this.plugin_entity.update(cx, |plugins, _cx| {
                        plugins.select_sidebar_panel(selection.clone());
                    });
                    this.toggle_context_sidebar_panel_pin(ContextSidebarPanel::PluginPanel, cx);
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
