// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use super::*;

impl WorkspaceApp {
    pub(super) fn terminal_command_action_button(
        &self,
        icon: LucideIcon,
        icon_color: Rgba,
        disabled: bool,
        background: Option<Rgba>,
        tooltip_id: &'static str,
        tooltip_title: String,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        // Tauri TerminalCommandBarActions uses a shared h-6/w-6 rounded-md
        // button for split, broadcast, recording, and cast controls. Keep the
        // geometry local to the terminal bar while routing activation through
        // the workspace button guard shared with FileManager/SFTP actions.
        self.workspace_icon_action_button(
            icon,
            14.0,
            icon_color,
            IconButtonOptions {
                disabled,
                background,
                hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                ..IconButtonOptions::opaque_toolbar(24.0, ButtonRadius::Md)
            },
            listener,
            cx,
        )
        .id(tooltip_id)
        .on_mouse_move(
            cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                this.queue_workspace_tooltip(
                    tooltip_id,
                    tooltip_title.clone(),
                    f32::from(event.position.x) + 12.0,
                    f32::from(event.position.y) + 16.0,
                    cx,
                );
            }),
        )
        .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
            if !*hovered {
                this.clear_workspace_tooltip(tooltip_id, cx);
            }
        }))
    }
}
