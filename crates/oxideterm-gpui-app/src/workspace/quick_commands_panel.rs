use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, Context, MouseButton, MouseMoveEvent, Pixels, Window, div, prelude::*, px, rgb,
    rgba,
};

use super::super::{WorkspaceApp, settings_ui_font_family};
use super::view::{QuickCommandsRenderSnapshot, quick_command_lucide_icon};
use super::{
    QuickCommand, QuickCommandButtonContextMenu, QuickCommandCategory, QuickCommandIcon,
};
use crate::assets::LucideIcon;

pub(in crate::workspace) const QUICK_COMMANDS_BAR_HEIGHT: f32 = 32.0;
const DEFAULT_HEIGHT: f32 = 240.0;
const MIN_HEIGHT: f32 = 180.0;
const MIN_TERMINAL_HEIGHT: f32 = 180.0;

pub(in crate::workspace) struct QuickCommandsPanelState {
    // Layout measurements exclude the toolbar, quick bar, and composer input.
    pub(in crate::workspace) available_height: Rc<Cell<f32>>,
    height: f32,
    drag: Option<(Pixels, f32)>,
}

impl Default for QuickCommandsPanelState {
    fn default() -> Self {
        Self {
            available_height: Rc::new(Cell::new(0.0)),
            height: DEFAULT_HEIGHT,
            drag: None,
        }
    }
}

#[allow(dead_code)]
impl QuickCommandsPanelState {
    fn clamp_height(&self, height: f32) -> f32 {
        let available = self.available_height.get();
        let maximum = (available - MIN_TERMINAL_HEIGHT)
            .max(0.0)
            .min(available * 0.5);
        height.clamp(MIN_HEIGHT.min(maximum), maximum)
    }

    pub(in crate::workspace) fn height(&self) -> f32 {
        self.clamp_height(self.height)
    }

    pub(in crate::workspace) fn is_resizing(&self) -> bool {
        self.drag.is_some()
    }

    pub(in crate::workspace) fn finish_resize(&mut self) -> bool {
        self.drag.take().is_some()
    }
}

impl WorkspaceApp {
    pub(in crate::workspace) fn toggle_terminal_quick_commands_panel(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let open = self
            .terminal
            .update(cx, |terminal, _| terminal.quick_commands.toggle_open());
        if !open {
            self.ime_marked_text = None;
            self.clear_ime_selection();
        }
        cx.notify();
    }

    #[allow(dead_code)]
    pub(in crate::workspace) fn prepare_terminal_quick_commands_panel(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _| {
            terminal.quick_commands.open = true;
        });
        cx.notify();
    }

    #[allow(dead_code)]
    pub(in crate::workspace) fn render_terminal_quick_commands_panel(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.render_terminal_quick_commands_bar(cx)
    }

    pub(in crate::workspace) fn render_terminal_quick_commands_bar(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let commands = &self.terminal.read(cx).quick_commands;
        let open = commands.is_open();
        let snapshot = open.then(|| self.quick_commands_render_snapshot(cx));
        let mut tokens = self.tokens;
        tokens.motion.spatial_enabled = true;
        let has_background = self.window_background_preferences().is_some();
        let theme = tokens.ui;

        let content = snapshot.map(|snapshot| {
            let bar = div()
                .id("terminal-quick-commands-bar")
                .relative()
                .w_full()
                .h(px(QUICK_COMMANDS_BAR_HEIGHT))
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .px(px(8.0))
                .border_t_1()
                .border_color(self.workspace_chrome_divider())
                .bg(if has_background {
                    rgba(
                        (theme.bg << 8)
                            | oxideterm_gpui_ui::surface::theme_glass_card_background_alpha(&tokens),
                    )
                } else {
                    rgb(theme.bg)
                })
                .text_size(px(tokens.metrics.ui_text_xs))
                .font_family(settings_ui_font_family(
                    &self.settings_store.settings().appearance.ui_font_family,
                ))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .flex_1()
                        .min_w(px(0.0))
                        .h_full()
                        .child(self.render_quick_commands_bar_category_selector(&snapshot, cx))
                        .child(
                            div()
                                .flex_none()
                                .w(px(1.0))
                                .h(px(14.0))
                                .mx(px(2.0))
                                .bg(rgba((theme.border << 8) | 0x80)),
                        )
                        .child(self.render_quick_commands_bar_buttons(&snapshot, cx)),
                )
                .child(self.render_quick_commands_bar_manager_button(cx))
                .into_any_element();

            if has_background {
                oxideterm_gpui_ui::surface::tab_surface(&tokens, bar)
            } else {
                bar
            }
        });

        oxideterm_gpui_ui::motion::auto_height(&tokens, "quick-commands-bar-height", content)
            .target_height(if open { QUICK_COMMANDS_BAR_HEIGHT } else { 0.0 })
            .into_any_element()
    }

    fn render_quick_commands_bar_category_selector(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let category = snapshot
            .categories
            .iter()
            .find(|c| c.id == snapshot.active_category)
            .cloned()
            .unwrap_or_else(|| QuickCommandCategory {
                id: "system".to_string(),
                name: "system".to_string(),
                icon: QuickCommandIcon::Zap,
                icon_color: None,
                sort_order: 0,
            });

        div()
            .id("quick-commands-bar-category-selector")
            .flex_none()
            .h(px(22.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(5.0))
            .rounded(px(self.tokens.radii.sm))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x80))
            .bg(rgba((theme.bg_panel << 8) | 0xcc))
            .hover(|style| {
                style
                    .bg(rgba((theme.bg_hover << 8) | 0xe6))
                    .border_color(rgb(theme.accent))
            })
            .cursor_pointer()
            .text_color(rgb(theme.text))
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .child(Self::render_lucide_icon(
                quick_command_lucide_icon(category.icon),
                12.0,
                rgb(theme.accent),
            ))
            .child(
                div()
                    .max_w(px(100.0))
                    .truncate()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(category.name),
            )
            .child(Self::render_lucide_icon(
                LucideIcon::ChevronDown,
                11.0,
                rgba((theme.text_muted << 8) | 0xb3),
            ))
            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _window, cx| {
                let delta_y = match event.delta {
                    gpui::ScrollDelta::Lines(lines) => lines.y,
                    gpui::ScrollDelta::Pixels(pixels) => f32::from(pixels.y),
                };
                if delta_y.abs() > 0.05 {
                    let mut changed = false;
                    this.terminal.update(cx, |terminal, _cx| {
                        let categories = &terminal.quick_commands.store.categories;
                        if categories.is_empty() {
                            return;
                        }
                        let current_idx = categories
                            .iter()
                            .position(|c| c.id == terminal.quick_commands.store.active_category)
                            .unwrap_or(0);
                        let next_idx = if delta_y > 0.0 {
                            current_idx.saturating_sub(1)
                        } else {
                            (current_idx + 1).min(categories.len().saturating_sub(1))
                        };
                        if next_idx != current_idx {
                            terminal.quick_commands.store.active_category =
                                categories[next_idx].id.clone();
                            changed = true;
                        }
                    });
                    if changed {
                        cx.notify();
                    }
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.terminal.update(cx, |terminal, _cx| {
                        terminal.quick_commands.category_menu_open =
                            !terminal.quick_commands.category_menu_open;
                    });
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn render_quick_commands_category_menu(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let snapshot = self.quick_commands_render_snapshot(cx);
        let theme = self.tokens.ui;
        let categories = snapshot.categories.clone();
        let active_id = snapshot.active_category.clone();

        let mut list = div().flex().flex_col().gap(px(2.0));

        for (index, cat) in categories.into_iter().enumerate() {
            let cat_id = cat.id.clone();
            let cat_name = cat.name.clone();
            let cat_icon = cat.icon;
            let is_active = cat_id == active_id;
            let cat_id_for_click = cat_id.clone();

            let item = div()
                .id(("quick-cmd-cat-menu-item", index))
                .h(px(26.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(self.tokens.radii.sm))
                .bg(if is_active {
                    rgba((theme.accent << 8) | 0x22)
                } else {
                    rgba(0x00000000)
                })
                .hover(|style| style.bg(rgba((theme.bg_hover << 8) | 0xb3)))
                .cursor_pointer()
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .text_color(if is_active {
                    rgb(theme.accent)
                } else {
                    rgb(theme.text)
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(Self::render_lucide_icon(
                            quick_command_lucide_icon(cat_icon),
                            12.0,
                            if is_active {
                                rgb(theme.accent)
                            } else {
                                rgb(theme.text_muted)
                            },
                        ))
                        .child(div().truncate().child(cat_name)),
                )
                .when(is_active, |row| {
                    row.child(Self::render_lucide_icon(
                        LucideIcon::Check,
                        12.0,
                        rgb(theme.accent),
                    ))
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        this.terminal.update(cx, |terminal, _cx| {
                            terminal.quick_commands.store.active_category =
                                cat_id_for_click.clone();
                            terminal.quick_commands.category_menu_open = false;
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );

            list = list.child(item);
        }

        div()
            .id("quick-commands-category-dropdown-panel")
            .absolute()
            .bottom(px(40.0))
            .left(px(8.0))
            .w(px(160.0))
            .max_h(px(200.0))
            .overflow_y_scroll()
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgb(theme.border))
            .bg(rgba((theme.bg_elevated << 8) | 0xfc))
            .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&self.tokens))
            .p(px(4.0))
            .occlude()
            .child(list)
            .into_any_element()
    }

    pub(in crate::workspace) fn open_quick_command_button_context_menu(
        &mut self,
        command: QuickCommand,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.category_menu_open = false;
            terminal.quick_commands.button_context_menu = Some(QuickCommandButtonContextMenu {
                command,
                x,
                y,
            });
        });
        cx.notify();
    }

    pub(in crate::workspace) fn dismiss_terminal_quick_commands_button_context_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        self.terminal
            .update(cx, |terminal, _cx| terminal.quick_commands.dismiss_button_context_menu())
    }

    pub(in crate::workspace) fn render_quick_commands_button_context_menu(
        &self,
        menu: &QuickCommandButtonContextMenu,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let viewport = window.viewport_size();
        let viewport_width = f32::from(viewport.width);
        const CAPSULE_WIDTH: f32 = 148.0;
        const CAPSULE_HEIGHT: f32 = 28.0;

        let mut pos_x = menu.x - CAPSULE_WIDTH / 2.0;
        pos_x = pos_x.clamp(8.0, (viewport_width - CAPSULE_WIDTH - 8.0).max(8.0));
        let pos_y = (menu.y - CAPSULE_HEIGHT - 12.0).max(8.0);

        let edit_command = menu.command.clone();
        let edit_btn = div()
            .id("quick-cmd-capsule-edit")
            .flex_1()
            .h_full()
            .px(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .rounded_full()
            .hover(|s| s.bg(rgba((theme.bg_hover << 8) | 0xb3)))
            .cursor_pointer()
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .text_color(rgb(theme.text))
            .child(Self::render_lucide_icon(
                LucideIcon::Pencil,
                11.0,
                rgb(theme.text_muted),
            ))
            .child(self.i18n.t("terminal.quick_commands.edit"))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event, _window, cx| {
                this.dismiss_terminal_quick_commands_button_context_menu(cx);
                this.start_quick_command_edit(edit_command.clone(), cx);
                cx.stop_propagation();
            }));

        let delete_id = menu.command.id.clone();
        let delete_btn = div()
            .id("quick-cmd-capsule-delete")
            .flex_1()
            .h_full()
            .px(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .rounded_full()
            .hover(|s| {
                s.bg(rgba((theme.error << 8) | 0x22))
                    .text_color(rgb(theme.error))
            })
            .cursor_pointer()
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .text_color(rgb(theme.text))
            .child(Self::render_lucide_icon(
                LucideIcon::Trash2,
                11.0,
                rgb(theme.text_muted),
            ))
            .child(self.i18n.t("terminal.quick_commands.delete"))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _event, _window, cx| {
                let id = delete_id.clone();
                this.dismiss_terminal_quick_commands_button_context_menu(cx);
                this.terminal.update(cx, |terminal, _cx| {
                    terminal.quick_commands.delete_command(&id);
                });
                cx.notify();
                cx.stop_propagation();
            }));

        div()
            .id("quick-commands-button-capsule-menu")
            .absolute()
            .top(px(pos_y))
            .left(px(pos_x))
            .w(px(CAPSULE_WIDTH))
            .h(px(CAPSULE_HEIGHT))
            .rounded_full()
            .border_1()
            .border_color(rgba((theme.border << 8) | 0xcc))
            .bg(rgba((theme.bg_elevated << 8) | 0xfc))
            .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&self.tokens))
            .px(px(3.0))
            .py(px(2.0))
            .flex()
            .items_center()
            .gap(px(2.0))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(edit_btn)
            .child(
                div()
                    .flex_none()
                    .w(px(1.0))
                    .h(px(12.0))
                    .bg(rgba((theme.border << 8) | 0x66)),
            )
            .child(delete_btn)
            .into_any_element()
    }

    fn render_quick_commands_bar_buttons(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let scroll_handle = self.terminal.read(cx).quick_commands.buttons_scroll.clone();
        let scroll_for_wheel = scroll_handle.clone();
        let commands = snapshot.visible_commands.iter().cloned().collect::<Vec<_>>();

        let mut button_row = div()
            .flex()
            .items_center()
            .h_full()
            .gap(px(6.0));

        for (index, command) in commands.into_iter().enumerate() {
            let command_id = command.id.clone();
            let command_name = command.name.clone();
            let command_text = command.command.clone();
            let tooltip_id = format!("quick-cmd-btn-{}", command_id);
            let tooltip_id_hover = tooltip_id.clone();
            let tooltip_text = command_text.clone();
            let command_for_run = command.clone();
            let command_for_menu = command.clone();

            let button = div()
                .id(("quick-command-bar-btn", index))
                .flex_none()
                .h(px(22.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .rounded(px(self.tokens.radii.sm))
                .border_1()
                .border_color(rgba((theme.border << 8) | 0x80))
                .bg(rgba((theme.bg_panel << 8) | 0xcc))
                .hover(|style| {
                    style
                        .bg(rgba((theme.bg_hover << 8) | 0xe6))
                        .border_color(rgb(theme.accent))
                        .text_color(rgb(theme.accent))
                })
                .cursor_pointer()
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .text_color(rgb(theme.text))
                .child(command_name)
                .on_mouse_move(cx.listener(move |this, event: &gpui::MouseMoveEvent, _window, cx| {
                    this.queue_workspace_tooltip_delayed(
                        tooltip_id.clone(),
                        tooltip_text.clone(),
                        f32::from(event.position.x) + 12.0,
                        f32::from(event.position.y) + 16.0,
                        cx,
                    );
                }))
                .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                    if !*hovered {
                        this.clear_workspace_tooltip(&tooltip_id_hover, cx);
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, window, cx| {
                        this.dismiss_terminal_quick_commands_button_context_menu(cx);
                        this.run_quick_command_model(&command_for_run, window, cx);
                        cx.stop_propagation();
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, _window, cx| {
                        this.clear_all_workspace_tooltips(cx);
                        this.open_quick_command_button_context_menu(
                            command_for_menu.clone(),
                            f32::from(event.position.x),
                            f32::from(event.position.y),
                            cx,
                        );
                        cx.stop_propagation();
                    }),
                );

            button_row = button_row.child(button);
        }

        div()
            .id("quick-commands-bar-buttons-viewport")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .items_center()
            .overflow_x_scroll()
            .track_scroll(&scroll_handle)
            .on_scroll_wheel(move |event, _window, cx| {
                if crate::workspace::breadcrumb_scroll::scroll_breadcrumb_by_wheel(
                    &scroll_for_wheel,
                    event,
                    px(22.0),
                )
                .unwrap_or(false)
                {
                    _window.refresh();
                }
                cx.stop_propagation();
            })
            .child(button_row)
            .into_any_element()
    }

    fn render_quick_commands_bar_manager_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.tokens.ui;
        let tooltip_id = "quick-commands-bar-manager-btn".to_string();
        let tooltip_id_hover = tooltip_id.clone();
        let tooltip_title = self.i18n.t("terminal.quick_commands.manage");
        let tooltip_title_move = tooltip_title.clone();

        div()
            .id("quick-commands-bar-manager-btn")
            .flex_none()
            .h(px(22.0))
            .w(px(22.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(self.tokens.radii.sm))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x80))
            .bg(rgba((theme.bg_panel << 8) | 0xcc))
            .hover(|style| {
                style
                    .bg(rgba((theme.bg_hover << 8) | 0xe6))
                    .border_color(rgb(theme.accent))
            })
            .cursor_pointer()
            .child(Self::render_lucide_icon(
                LucideIcon::Settings,
                12.0,
                rgb(theme.text_muted),
            ))
            .on_mouse_move(cx.listener(move |this, event: &gpui::MouseMoveEvent, _window, cx| {
                this.queue_workspace_tooltip(
                    tooltip_id.clone(),
                    tooltip_title_move.clone(),
                    f32::from(event.position.x) + 12.0,
                    f32::from(event.position.y) + 16.0,
                    cx,
                );
            }))
            .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                if !*hovered {
                    this.clear_workspace_tooltip(&tooltip_id_hover, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, window, cx| {
                    this.open_quick_commands_manager(window, cx);
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    pub(in crate::workspace) fn update_terminal_quick_commands_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let changed = self.terminal.update(cx, |terminal, _| {
            let panel = &mut terminal.quick_commands.panel;
            let Some((start_y, start_height)) = panel.drag else {
                return false;
            };
            if !event.dragging() {
                return panel.finish_resize();
            }
            let height = panel.clamp_height(start_height - f32::from(event.position.y - start_y));
            let changed = (height - panel.height).abs() > 0.1;
            panel.height = height;
            changed
        });
        if changed {
            cx.notify();
        }
    }

    pub(in crate::workspace) fn finish_terminal_quick_commands_resize(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.terminal.update(cx, |terminal, _| {
            terminal.quick_commands.panel.finish_resize()
        }) {
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Render, ScrollHandle, TestAppContext};

    struct DockFixture {
        panel: QuickCommandsPanelState,
        scroll: ScrollHandle,
        open: bool,
    }

    impl Render for DockFixture {
        fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let mut tokens = oxideterm_theme::default_tokens();
            tokens.motion.enabled = false;
            self.panel
                .available_height
                .set(f32::from(window.viewport_size().height) - 64.0);
            let height = self.panel.height();
            let sidebar = div().w(px(160.0)).flex_none().h_full().into_any_element();
            let body = div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(49.0))
                        .flex_none()
                        .debug_selector(|| "search".into()),
                )
                .child(
                    div()
                        .id("rows")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll)
                        .children((0..20).map(|index| {
                            div()
                                .h(px(48.0))
                                .flex_none()
                                .debug_selector(move || format!("row-{index}"))
                        })),
                )
                .into_any_element();
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .debug_selector(|| "terminal".into()),
                )
                .child(
                    div()
                        .h(px(32.0))
                        .flex_none()
                        .debug_selector(|| "toolbar".into()),
                )
                .child(
                    oxideterm_gpui_ui::motion::auto_height(
                        &tokens,
                        "dock",
                        Some(
                            panel_frame(&tokens, false, height, sidebar, body)
                                .debug_selector(|| "dock".into())
                                .into_any_element(),
                        ),
                    )
                    .target_height(if self.open { height } else { 0.0 }),
                )
                .child(
                    div()
                        .h(px(32.0))
                        .flex_none()
                        .debug_selector(|| "input".into()),
                )
        }
    }

    #[gpui::test]
    fn dock_occupies_space_and_scrolls_within_small_window_bounds(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| DockFixture {
            panel: QuickCommandsPanelState::default(),
            scroll: ScrollHandle::new(),
            open: true,
        });
        for (width, height, expected_dock_height) in [(900.0, 700.0, 260.0), (480.0, 400.0, 156.0)]
        {
            cx.simulate_resize(gpui::size(px(width), px(height)));
            cx.update(|window, app| window.draw(app).clear(app));
            let terminal = cx.debug_bounds("terminal").unwrap();
            let toolbar = cx.debug_bounds("toolbar").unwrap();
            let dock = cx.debug_bounds("dock").unwrap();
            let input = cx.debug_bounds("input").unwrap();
            assert_eq!(dock.size, gpui::size(px(width), px(expected_dock_height)));
            assert_eq!(terminal.bottom(), toolbar.top());
            assert_eq!(toolbar.bottom(), dock.top());
            assert_eq!(dock.bottom(), input.top());
            assert_eq!(input.bottom(), px(height));
            assert!(terminal.size.height >= px(MIN_TERMINAL_HEIGHT));
            let scroll = view.read_with(cx, |view, _| view.scroll.clone());
            let search = cx.debug_bounds("search").unwrap();
            scroll.set_offset(gpui::point(px(0.0), -scroll.max_offset().y));
            cx.update(|window, app| {
                window.refresh();
                window.draw(app).clear(app);
            });
            assert_eq!(cx.debug_bounds("search").unwrap(), search);
            let last = cx.debug_bounds("row-19").unwrap();
            assert!(last.top() >= scroll.bounds().top());
            assert!((last.bottom() - scroll.bounds().bottom()).abs() < px(1.0));
        }
        view.update(cx, |view, cx| {
            view.open = false;
            cx.notify();
        });
        cx.update(|window, app| window.draw(app).clear(app));
        assert_eq!(cx.debug_bounds("terminal").unwrap().size.height, px(336.0));
        assert_eq!(
            cx.debug_bounds("toolbar").unwrap().bottom(),
            cx.debug_bounds("input").unwrap().top()
        );
    }
}
