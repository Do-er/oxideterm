use super::*;
use gpui::{StatefulInteractiveElement, point};
use oxideterm_gpui_ui::select::{
    select_option_action, select_option_highlighted, select_overlay_popup_with_max_height,
    select_separator,
};

const TOOLS: [ContextSidebarTool; 11] = [
    ContextSidebarTool::Monitor,
    ContextSidebarTool::Processes,
    ContextSidebarTool::Gpu,
    ContextSidebarTool::Docker,
    ContextSidebarTool::Services,
    ContextSidebarTool::Logs,
    ContextSidebarTool::Ports,
    ContextSidebarTool::Tmux,
    ContextSidebarTool::Filesystems,
    ContextSidebarTool::Schedules,
    ContextSidebarTool::Packages,
];

impl ContextSidebarTool {
    pub(in crate::workspace) const ALL: [Self; 11] = TOOLS;

    pub(in crate::workspace) fn settings_key(self) -> &'static str {
        match self {
            Self::Monitor => "monitor",
            Self::Gpu => "gpu",
            Self::Processes => "processes",
            Self::Services => "services",
            Self::Logs => "logs",
            Self::Tmux => "tmux",
            Self::Docker => "docker",
            Self::Ports => "ports",
            Self::Schedules => "schedules",
            Self::Filesystems => "filesystems",
            Self::Packages => "packages",
        }
    }

    fn is_monitoring_tool(self) -> bool {
        matches!(
            self,
            Self::Monitor | Self::Processes | Self::Gpu | Self::Docker
        )
    }

    pub(in crate::workspace) fn label_key(self) -> &'static str {
        match self {
            Self::Monitor => "sidebar.panels.host_monitor",
            Self::Gpu => "sidebar.panels.gpu",
            Self::Processes => "sidebar.panels.processes",
            Self::Services => "sidebar.panels.services",
            Self::Logs => "sidebar.panels.logs",
            Self::Tmux => "sidebar.panels.tmux",
            Self::Docker => "sidebar.panels.docker",
            Self::Ports => "sidebar.panels.ports",
            Self::Schedules => "sidebar.panels.schedules",
            Self::Filesystems => "sidebar.panels.filesystems",
            Self::Packages => "sidebar.panels.packages",
        }
    }

    pub(in crate::workspace) fn icon(self) -> LucideIcon {
        match self {
            Self::Monitor => LucideIcon::Activity,
            Self::Gpu => LucideIcon::Gpu,
            Self::Processes => LucideIcon::ListTree,
            Self::Services => LucideIcon::ServerCog,
            Self::Logs => LucideIcon::FileText,
            Self::Tmux => LucideIcon::SplitSquareHorizontal,
            Self::Docker => LucideIcon::Docker,
            Self::Ports => LucideIcon::Network,
            Self::Schedules => LucideIcon::CalendarClock,
            Self::Filesystems => LucideIcon::HardDrive,
            Self::Packages => LucideIcon::Package,
        }
    }
}

fn enabled_tools(hidden_host_tools: &[String]) -> Vec<ContextSidebarTool> {
    TOOLS
        .into_iter()
        .filter(|tool| {
            let key = tool.settings_key();
            !hidden_host_tools.iter().any(|hidden| hidden == key)
        })
        .collect()
}

fn monitoring_tool_count(tools: &[ContextSidebarTool]) -> usize {
    tools
        .iter()
        .take_while(|tool| tool.is_monitoring_tool())
        .count()
}

fn visible_tool_count_for_tools(
    tools: &[ContextSidebarTool],
    available_height: f32,
    row_height: f32,
    separator_height: f32,
    ai_enabled: bool,
) -> usize {
    if tools.is_empty() {
        return 0;
    }
    let monitoring_count = monitoring_tool_count(tools);
    // Reserve More only for overflow, and the second divider only when both groups show.
    (0..=tools.len())
        .rev()
        .find(|&count| {
            let more_height = if count < tools.len() { row_height } else { 0.0 };
            tool_stack_height_for_tools(
                count,
                monitoring_count,
                row_height,
                separator_height,
                ai_enabled,
            ) + more_height
                <= available_height
        })
        .unwrap_or(0)
}

#[cfg(test)]
fn visible_tool_count(
    available_height: f32,
    row_height: f32,
    separator_height: f32,
    ai_enabled: bool,
) -> usize {
    visible_tool_count_for_tools(
        &TOOLS,
        available_height,
        row_height,
        separator_height,
        ai_enabled,
    )
}

fn tool_stack_height_for_tools(
    count: usize,
    monitoring_count: usize,
    row_height: f32,
    separator_height: f32,
    ai_enabled: bool,
) -> f32 {
    let separators = if monitoring_count > 0 && count > monitoring_count {
        2.0
    } else {
        1.0
    };
    let leading_tools = if ai_enabled { 2.0 } else { 1.0 };
    (count as f32 + leading_tools) * row_height + separators * separator_height
}

fn overflow_tools_from(tools: &[ContextSidebarTool], visible: usize) -> Vec<ContextSidebarTool> {
    tools[visible.min(tools.len())..].to_vec()
}

#[cfg(test)]
fn overflow_tools(visible: usize) -> Vec<ContextSidebarTool> {
    overflow_tools_from(&TOOLS, visible)
}

impl WorkspaceApp {
    fn context_tool_separator(&self) -> gpui::Div {
        div()
            .flex_none()
            .w(px(self.tokens.metrics.divider_width))
            .h(px(self.tokens.metrics.divider_height))
            .mb(px(self.tokens.metrics.activity_icon_gap))
            .bg(self.workspace_divider_color(self.tokens.ui.divider, 0xff))
    }

    /// Plugin panels and action items always render at the end of the rail, so
    /// host tools must give up exactly the space those rows occupy.
    fn plugin_rail_contributions(
        &self,
        cx: &App,
    ) -> (
        Vec<plugin_host::NativePluginRuntimeSidebarPanelContribution>,
        Vec<plugin_host::NativePluginRuntimeActivityBarItemContribution>,
    ) {
        let plugins = self.plugin_entity.read(cx);
        let contributions = plugins.registry().contributions();
        (
            contributions.runtime_sidebar_panels(),
            contributions.runtime_activity_bar_items(),
        )
    }

    pub(in crate::workspace) fn enabled_context_sidebar_tools(&self) -> Vec<ContextSidebarTool> {
        enabled_tools(&self.settings_store.settings().sidebar_ui.hidden_host_tools)
    }

    pub(in crate::workspace) fn toggle_sidebar_host_tool_visibility(
        &mut self,
        tool: ContextSidebarTool,
        cx: &mut Context<Self>,
    ) {
        let key = tool.settings_key();
        self.edit_settings(
            |settings| {
                let hidden = &mut settings.sidebar_ui.hidden_host_tools;
                if let Some(index) = hidden.iter().position(|item| item == key) {
                    hidden.remove(index);
                } else {
                    hidden.push(key.to_string());
                }
            },
            cx,
        );
        self.reconcile_visible_context_sidebar_tools(cx);
    }

    pub(in crate::workspace) fn reconcile_visible_context_sidebar_tools(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let tools = self.enabled_context_sidebar_tools();
        if self
            .context_tools_menu
            .is_some_and(|highlighted| !tools.contains(&highlighted))
        {
            self.context_tools_menu = tools.first().copied();
        }
        let active_tool = self.host_tools.read(cx).active_tool();
        if !tools.contains(&active_tool) {
            if let Some(&replacement) = tools.first() {
                self.host_tools.update(cx, |host_tools, cx| {
                    host_tools.select_sidebar_tool(replacement, cx);
                });
            } else if self.active_context_sidebar_panel == ContextSidebarPanel::HostTools {
                self.active_context_sidebar_panel = ContextSidebarPanel::Sessions;
                if self.context_sidebar_visible() {
                    self.collapse_context_sidebar(cx);
                } else {
                    self.sync_host_tools_lifecycle(false, cx);
                }
            }
        }
    }

    fn context_tool_count_for(
        &self,
        tools: &[ContextSidebarTool],
        window: &Window,
        cx: &App,
    ) -> usize {
        let metrics = self.tokens.metrics;
        let lock_height = if self.settings_store.settings().sidebar_ui.show_app_lock_icon {
            metrics.activity_icon_size + metrics.activity_icon_gap + self.tokens.spacing.two * 2.0
        } else {
            0.0
        };
        let (panels, items) = self.plugin_rail_contributions(cx);
        let plugin_rows = panels.len() + items.len();
        let plugin_height = if plugin_rows == 0 {
            0.0
        } else {
            plugin_rows as f32 * (metrics.activity_icon_size + metrics.activity_icon_gap)
                + metrics.divider_height
                + metrics.activity_icon_gap
        };
        let available = f32::from(window.viewport_size().height)
            - self.window_titlebar_height(window)
            - metrics.tabbar_height
            - PRIMARY_SIDEBAR_CONTENT_TOP_INSET
            - lock_height
            - plugin_height;
        visible_tool_count_for_tools(
            tools,
            available,
            metrics.activity_icon_size + metrics.activity_icon_gap,
            metrics.divider_height + metrics.activity_icon_gap,
            self.settings_store.settings().ai.enabled,
        )
    }

    fn context_tool_active(&self, tool: ContextSidebarTool, cx: &App) -> bool {
        self.context_sidebar_visible()
            && self.active_context_sidebar_panel == ContextSidebarPanel::HostTools
            && self.host_tools.read(cx).active_tool() == tool
    }

    fn activate_context_tool(&mut self, tool: ContextSidebarTool, cx: &mut Context<Self>) {
        self.context_tools_menu = None;
        self.clear_all_workspace_tooltips(cx);
        self.host_tools.update(cx, |host_tools, cx| {
            host_tools.select_sidebar_tool(tool, cx)
        });
        if !self.context_sidebar_visible()
            || self.active_context_sidebar_panel != ContextSidebarPanel::HostTools
        {
            self.open_context_sidebar_panel(ContextSidebarPanel::HostTools, cx);
        }
        cx.notify();
    }

    fn context_tool_button_options(&self, active: bool) -> IconButtonOptions {
        IconButtonOptions {
            radius: ButtonRadius::Md,
            has_background: active,
            background: active.then(|| self.settings_panel_background(self.tokens.ui.bg_panel)),
            border: active.then(|| rgb(self.tokens.ui.border)),
            hover_background: Some(rgb(self.tokens.ui.bg_hover)),
            idle_opacity: 1.0,
            ..IconButtonOptions::compact(self.tokens.metrics.activity_icon_size)
        }
    }

    fn render_sessions_context_tool(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.active_sessions_sidebar_visible();
        div()
            .id("context-sessions-activity")
            .flex_none()
            .mb(px(self.tokens.metrics.activity_icon_gap))
            .child(self.workspace_tooltip_icon_button(
                LucideIcon::Link2,
                self.tokens.metrics.activity_icon_glyph_size,
                rgb(if active {
                    self.tokens.ui.accent
                } else {
                    self.tokens.ui.text
                }),
                self.context_tool_button_options(active),
                self.i18n.t("sidebar.panels.sessions"),
                "context-sessions",
                true,
                cx.listener(|this, _, _, cx| {
                    let _ = this.open_context_sidebar_panel(ContextSidebarPanel::Sessions, cx);
                    cx.stop_propagation();
                }),
                cx.entity(),
            ))
            .on_hover(cx.listener(
                |this, hovered: &bool, _window, cx| {
                    this.set_context_sidebar_activity_hovered(
                        (*hovered).then_some(ContextSidebarPanel::Sessions),
                        cx,
                    );
                },
            ))
            .into_any_element()
    }

    fn render_connections_context_tool(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.context_sidebar_visible()
            && self.active_context_sidebar_panel == ContextSidebarPanel::Connections;
        div()
            .id("context-connections-activity")
            .flex_none()
            .mb(px(self.tokens.metrics.activity_icon_gap))
            .child(self.workspace_tooltip_icon_button(
                LucideIcon::Server,
                self.tokens.metrics.activity_icon_glyph_size,
                rgb(if active {
                    self.tokens.ui.accent
                } else {
                    self.tokens.ui.text
                }),
                self.context_tool_button_options(active),
                self.i18n.t("sidebar.panels.session_manager"),
                "context-connections",
                true,
                cx.listener(|this, _, _, cx| {
                    this.refresh_session_manager_ssh_config_hosts(cx);
                    let _ = this.open_context_sidebar_panel(ContextSidebarPanel::Connections, cx);
                    cx.stop_propagation();
                }),
                cx.entity(),
            ))
            .on_hover(cx.listener(
                |this, hovered: &bool, _window, cx| {
                    this.set_context_sidebar_activity_hovered(
                        (*hovered).then_some(ContextSidebarPanel::Connections),
                        cx,
                    );
                },
            ))
            .into_any_element()
    }

    pub(in crate::workspace) fn render_context_tool_buttons(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tools = self.enabled_context_sidebar_tools();
        let monitoring_count = monitoring_tool_count(&tools);
        let count = self.context_tool_count_for(&tools, window, cx);
        let ai_enabled = self.settings_store.settings().ai.enabled;
        let mut buttons = div()
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(PRIMARY_SIDEBAR_CONTENT_TOP_INSET))
            .child(self.render_sessions_context_tool(cx))
            .child(self.render_connections_context_tool(cx));
        if ai_enabled {
            buttons = buttons.child(self.render_activity_icon(
                SidebarSection::Assistant,
                LucideIcon::Sparkles,
                cx,
            ));
        }
        if !tools.is_empty() {
            buttons = buttons.child(self.context_tool_separator());
        }
        for (index, tool) in tools.iter().take(count).copied().enumerate() {
            if monitoring_count > 0 && index == monitoring_count {
                buttons = buttons.child(self.context_tool_separator());
            }
            let active = self.context_tool_active(tool, cx);
            buttons = buttons.child(
                div()
                    .id(format!("context-tool-activity-{index}"))
                    .flex_none()
                    .mb(px(self.tokens.metrics.activity_icon_gap))
                    .child(self.workspace_tooltip_icon_button(
                        tool.icon(),
                        self.tokens.metrics.activity_icon_glyph_size,
                        rgb(if active {
                            self.tokens.ui.accent
                        } else {
                            self.tokens.ui.text
                        }),
                        self.context_tool_button_options(active),
                        self.i18n.t(tool.label_key()),
                        "context-tool",
                        true,
                        cx.listener(move |this, _, _, cx| {
                            this.activate_context_tool(tool, cx);
                            cx.stop_propagation();
                        }),
                        cx.entity(),
                    ))
                    .on_hover(cx.listener(
                        |this, hovered: &bool, _window, cx| {
                            this.set_context_sidebar_activity_hovered(
                                (*hovered).then_some(ContextSidebarPanel::HostTools),
                                cx,
                            );
                        },
                    )),
            );
        }
        let overflow = overflow_tools_from(&tools, count);
        if !overflow.is_empty() {
            let active = self.context_tools_menu.is_some()
                || overflow
                    .iter()
                    .any(|tool| self.context_tool_active(*tool, cx));
            buttons = buttons.child(
                div()
                    .flex_none()
                    .mb(px(self.tokens.metrics.activity_icon_gap))
                    .child(self.workspace_tooltip_icon_button(
                        LucideIcon::MoreVertical,
                        self.tokens.metrics.activity_icon_glyph_size,
                        rgb(if active {
                            self.tokens.ui.accent
                        } else {
                            self.tokens.ui.text
                        }),
                        self.context_tool_button_options(active),
                        self.i18n.t("sidebar.panels.more_tools"),
                        "context-tools-more",
                        true,
                        cx.listener(move |this, _, window, cx| {
                            let open = this.context_tools_menu.is_none();
                            this.prepare_modal_interaction_boundary(cx);
                            if open {
                                let tools = this.enabled_context_sidebar_tools();
                                let overflow = overflow_tools_from(
                                    &tools,
                                    this.context_tool_count_for(&tools, window, cx),
                                );
                                if overflow.is_empty() {
                                    return;
                                }
                                let active_tool = this.host_tools.read(cx).active_tool();
                                let index = overflow
                                    .iter()
                                    .position(|tool| *tool == active_tool)
                                    .unwrap_or(0);
                                this.context_tools_menu = Some(overflow[index]);
                                this.context_tools_menu_scroll.scroll_to_item(index);
                                window.focus(&this.focus_handle, cx);
                            }
                            cx.stop_propagation();
                            cx.notify();
                        }),
                        cx.entity(),
                    )),
            );
        }
        // Plugin entries follow the host-tool block so their rows never shift
        // the More trigger that the overflow menu anchors to.
        buttons = buttons.children(self.render_plugin_rail_buttons(cx));
        buttons.into_any_element()
    }

    fn render_plugin_rail_buttons(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let (panels, items) = self.plugin_rail_contributions(cx);
        if panels.is_empty() && items.is_empty() {
            return Vec::new();
        }
        let mut rows = vec![self.context_tool_separator().into_any_element()];
        for panel in panels {
            rows.push(self.render_plugin_sidebar_activity_icon(panel, cx));
        }
        for item in items {
            rows.push(self.render_plugin_activity_action_icon(item, cx));
        }
        rows
    }

    pub(in crate::workspace) fn render_context_tools_menu(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let highlighted = self.context_tools_menu?;
        if self.settings_store.settings().sidebar_ui.zen_mode {
            self.context_tools_menu = None;
            return None;
        }
        let tools = self.enabled_context_sidebar_tools();
        let monitoring_count = monitoring_tool_count(&tools);
        let count = self.context_tool_count_for(&tools, window, cx);
        let overflow = overflow_tools_from(&tools, count);
        if overflow.is_empty() {
            self.context_tools_menu = None;
            return None;
        }
        let highlighted = if overflow.contains(&highlighted) {
            highlighted
        } else {
            overflow[0]
        };
        let max_height = (f32::from(window.viewport_size().height) - 24.0)
            .min(self.tokens.metrics.ui_select_max_height);
        let mut popup = select_overlay_popup_with_max_height(&self.tokens, 220.0, max_height)
            .id("context-tools-menu")
            .track_scroll(&self.context_tools_menu_scroll);
        for (index, tool) in overflow.into_iter().enumerate() {
            let option = select_option_action(
                select_option_highlighted(
                    &self.tokens,
                    self.i18n.t(tool.label_key()),
                    self.context_tool_active(tool, cx),
                    highlighted == tool,
                ),
                false,
                false,
                cx.listener(move |this, _, _, cx| {
                    this.activate_context_tool(tool, cx);
                    cx.stop_propagation();
                }),
            );
            // Keep one scroll item per tool, including its optional group divider.
            popup = popup.child(
                div()
                    .when(
                        monitoring_count > 0 && index > 0 && count + index == monitoring_count,
                        |row| row.child(select_separator(&self.tokens)),
                    )
                    .child(option),
            );
        }
        let metrics = self.tokens.metrics;
        // The fixed rail has no scroll transform. Derive the trigger position from the same
        // layout inputs in this frame, so resizing cannot leave a cached anchor behind.
        let top = self.window_titlebar_height(window)
            + metrics.tabbar_height
            + PRIMARY_SIDEBAR_CONTENT_TOP_INSET
            + tool_stack_height_for_tools(
                count,
                monitoring_count,
                metrics.activity_icon_size + metrics.activity_icon_gap,
                metrics.divider_height + metrics.activity_icon_gap,
                self.settings_store.settings().ai.enabled,
            );
        Some(
            popover_backdrop()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.context_tools_menu = None;
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _, _, cx| {
                        this.context_tools_menu = None;
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .child(deferred(
                    anchored()
                        .anchor(Corner::TopRight)
                        .position(point(
                            window.viewport_size().width - px(metrics.activity_bar_width + 4.0),
                            px(top),
                        ))
                        .position_mode(AnchoredPositionMode::Window)
                        .snap_to_window_with_margin(px(8.0))
                        .child(popup),
                ))
                .into_any_element(),
        )
    }

    pub(in crate::workspace) fn handle_context_tools_menu_key(
        &mut self,
        key: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(highlighted) = self.context_tools_menu else {
            return false;
        };
        let tools = self.enabled_context_sidebar_tools();
        let overflow = overflow_tools_from(&tools, self.context_tool_count_for(&tools, window, cx));
        if overflow.is_empty() {
            self.context_tools_menu = None;
            return false;
        }
        let index = overflow
            .iter()
            .position(|tool| *tool == highlighted)
            .unwrap_or(0);
        let next = match key {
            "escape" | "tab" => {
                self.context_tools_menu = None;
                cx.notify();
                return true;
            }
            "enter" | "space" | " " => {
                self.activate_context_tool(overflow[index], cx);
                return true;
            }
            "arrowdown" | "down" => browser_behavior::browser_select_next_index(
                index,
                overflow.len(),
                browser_behavior::BrowserSelectKeyDirection::Next,
            ),
            "arrowup" | "up" => browser_behavior::browser_select_next_index(
                index,
                overflow.len(),
                browser_behavior::BrowserSelectKeyDirection::Previous,
            ),
            "home" => 0,
            "end" => overflow.len() - 1,
            _ => return true,
        };
        self.context_tools_menu = Some(overflow[next]);
        self.context_tools_menu_scroll.scroll_to_item(next);
        cx.notify();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrinking_the_rail_moves_only_trailing_tools_into_more() {
        // All tools fit exactly without reserving an unnecessary More button.
        let all_visible = visible_tool_count(538.0, 40.0, 9.0, true);
        assert_eq!(all_visible, 11);
        assert_eq!(overflow_tools(all_visible), vec![]);
        assert_eq!(visible_tool_count(700.0, 40.0, 9.0, true), 11);
        // Once the last row no longer fits, More needs a row of its own.
        let almost_all = visible_tool_count(537.0, 40.0, 9.0, true);
        assert_eq!(almost_all, 9);
        assert_eq!(
            overflow_tools(almost_all),
            vec![ContextSidebarTool::Schedules, ContextSidebarTool::Packages]
        );
        assert_eq!(visible_tool_count(300.0, 40.0, 9.0, true), 4);
        assert_eq!(visible_tool_count(289.0, 40.0, 9.0, true), 4);
        // Four monitoring rows and More fit exactly; one fewer pixel overflows a row.
        assert_eq!(visible_tool_count(288.0, 40.0, 9.0, true), 3);
        let visible = visible_tool_count(210.0, 40.0, 9.0, true);
        assert_eq!(
            &TOOLS[..visible],
            &[ContextSidebarTool::Monitor, ContextSidebarTool::Processes]
        );
        assert_eq!(
            overflow_tools(visible),
            vec![
                ContextSidebarTool::Gpu,
                ContextSidebarTool::Docker,
                ContextSidebarTool::Services,
                ContextSidebarTool::Logs,
                ContextSidebarTool::Ports,
                ContextSidebarTool::Tmux,
                ContextSidebarTool::Filesystems,
                ContextSidebarTool::Schedules,
                ContextSidebarTool::Packages
            ]
        );
        assert_eq!(visible_tool_count(80.0, 40.0, 9.0, true), 0);

        // When AI is disabled, one fewer leading tool is rendered (only Sessions, saving 40px).
        assert_eq!(visible_tool_count(498.0, 40.0, 9.0, false), 11);
        assert_eq!(visible_tool_count(497.0, 40.0, 9.0, false), 9);
    }

    #[test]
    fn hidden_host_tools_filter_rail_order_and_omit_redundant_group_divider() {
        let hidden = vec![
            "monitor".to_string(),
            "processes".to_string(),
            "gpu".to_string(),
            "docker".to_string(),
            "packages".to_string(),
        ];
        let tools = enabled_tools(&hidden);
        assert_eq!(
            tools,
            vec![
                ContextSidebarTool::Services,
                ContextSidebarTool::Logs,
                ContextSidebarTool::Ports,
                ContextSidebarTool::Tmux,
                ContextSidebarTool::Filesystems,
                ContextSidebarTool::Schedules,
            ]
        );
        assert_eq!(monitoring_tool_count(&tools), 0);
        // Without monitoring tools, only one divider is needed: 6 tools + 2 leading = 8 * 40 + 9 = 329.
        assert_eq!(visible_tool_count_for_tools(&tools, 329.0, 40.0, 9.0, true), 6);
        assert_eq!(overflow_tools_from(&tools, 6), Vec::<ContextSidebarTool>::new());
        assert_eq!(visible_tool_count_for_tools(&[], 500.0, 40.0, 9.0, true), 0);
    }
}
