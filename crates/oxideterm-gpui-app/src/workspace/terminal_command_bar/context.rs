// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use super::*;

fn ssh_latency_display(
    sample: oxideterm_ssh::SshLatencySnapshot,
    i18n: &oxideterm_i18n::I18n,
) -> (String, StatusTone, String) {
    use oxideterm_ssh::SshLatencySnapshot::*;
    let (label, tone, key) = match sample {
        Measured {
            millis,
            measured_at,
        } => {
            let tone = match millis {
                0..100 => StatusTone::Success,
                100..300 => StatusTone::Warning,
                _ => StatusTone::Error,
            };
            // Bound the toolbar label; the tooltip retains the exact measurement.
            // Truncate tenths so 999.999 seconds does not round into an extra digit.
            let label = match millis {
                0..1000 => format!("{millis} ms"),
                1000..1_000_000 => format!("{}.{:01} s", millis / 1000, millis % 1000 / 100),
                _ => "≥1000 s".into(),
            };
            let detail = i18n
                .t("terminal.ssh_latency.measured")
                .replace("{{millis}}", &millis.to_string())
                .replace("{{seconds}}", &measured_at.elapsed().as_secs().to_string());
            return (label, tone, detail);
        }
        Timeout => (
            format!(">{} s", oxideterm_ssh::SSH_LATENCY_TIMEOUT.as_secs()),
            StatusTone::Error,
            "terminal.ssh_latency.timeout",
        ),
        Pending => (
            "…".into(),
            StatusTone::Neutral,
            "terminal.ssh_latency.pending",
        ),
        Unavailable => (
            "—".into(),
            StatusTone::Neutral,
            "terminal.ssh_latency.unavailable",
        ),
        Unsupported => (
            "—".into(),
            StatusTone::Neutral,
            "terminal.ssh_latency.unsupported",
        ),
        Stale => (
            "—".into(),
            StatusTone::Neutral,
            "terminal.ssh_latency.stale",
        ),
    };
    let detail = i18n.t(key).replace(
        "{{seconds}}",
        &oxideterm_ssh::SSH_LATENCY_TIMEOUT.as_secs().to_string(),
    );
    (label, tone, detail)
}

#[cfg(test)]
mod latency_tests {
    use super::*;
    use oxideterm_i18n::{I18n, Locale};
    use oxideterm_ssh::SshLatencySnapshot;

    #[test]
    fn latency_labels_preserve_severity_and_extreme_measurements() {
        let i18n = I18n::new(Locale::En);
        for (millis, label, tone) in [
            (1, "1 ms", StatusTone::Success),
            (99, "99 ms", StatusTone::Success),
            (100, "100 ms", StatusTone::Warning),
            (299, "299 ms", StatusTone::Warning),
            (300, "300 ms", StatusTone::Error),
            (999, "999 ms", StatusTone::Error),
            (1000, "1.0 s", StatusTone::Error),
            (1999, "1.9 s", StatusTone::Error),
            (999_999, "999.9 s", StatusTone::Error),
            (1_000_000, "≥1000 s", StatusTone::Error),
            (u64::MAX, "≥1000 s", StatusTone::Error),
        ] {
            let (actual, actual_tone, detail) = ssh_latency_display(
                SshLatencySnapshot::Measured {
                    millis,
                    measured_at: tokio::time::Instant::now(),
                },
                &i18n,
            );
            assert_eq!(actual, label);
            assert_eq!(actual_tone, tone);
            assert!(detail.contains(&format!("{millis} ms")), "{detail}");
        }
        for (sample, label, tone, detail) in [
            (
                SshLatencySnapshot::Pending,
                "…",
                StatusTone::Neutral,
                "Measuring SSH latency…",
            ),
            (
                SshLatencySnapshot::Timeout,
                ">3 s",
                StatusTone::Error,
                "No reply within 3 s. Still waiting; this does not indicate a disconnected session.",
            ),
            (
                SshLatencySnapshot::Unavailable,
                "—",
                StatusTone::Neutral,
                "SSH connection unavailable",
            ),
            (
                SshLatencySnapshot::Unsupported,
                "—",
                StatusTone::Neutral,
                "Latency probing is disabled for this device's compatibility mode.",
            ),
            (
                SshLatencySnapshot::Stale,
                "—",
                StatusTone::Neutral,
                "Latency sample expired",
            ),
        ] {
            assert_eq!(
                ssh_latency_display(sample, &i18n),
                (label.into(), tone, detail.into())
            );
        }
    }
}

impl WorkspaceApp {
    fn terminal_command_icon_button(
        &self,
        icon: LucideIcon,
        icon_color: Rgba,
        options: IconButtonOptions,
        tooltip_id: &'static str,
        tooltip_title: String,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        self.workspace_icon_action_button(icon, 14.0, icon_color, options, listener, cx)
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
        self.terminal_command_icon_button(
            icon,
            icon_color,
            IconButtonOptions {
                disabled,
                background,
                hover_background: Some(rgb(self.tokens.ui.bg_hover)),
                ..IconButtonOptions::opaque_toolbar(24.0, ButtonRadius::Md)
            },
            tooltip_id,
            tooltip_title,
            listener,
            cx,
        )
    }

    /// Latency readout for the command bar action cluster. Returns nothing when
    /// the active pane is not a shared SSH transport or monitoring is disabled,
    /// so the cluster keeps its existing geometry for local and dedicated
    /// channel sessions.
    pub(super) fn render_terminal_ssh_latency_chip(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self
            .settings_store
            .settings()
            .terminal
            .command_bar
            .ssh_latency
        {
            return None;
        }
        let node = self.active_ssh_terminal_node_id(cx)?;
        let visible = window.is_visible() && !window.is_minimized();
        let sample = self
            .workspace_runtime
            .update(cx, |runtime, _| runtime.ssh_latency(&node, visible));
        let (label, tone, detail) = ssh_latency_display(sample, &self.i18n);
        let tooltip_id = "terminal-command-ssh-latency";
        let mono_font = settings_mono_font_family(self.settings_store.settings());
        Some(
            div()
                .h(px(20.0))
                .px(px(2.0))
                .flex_none()
                .flex()
                .items_center()
                .whitespace_nowrap()
                .id(tooltip_id)
                .on_mouse_move({
                    let title = detail;
                    cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                        this.queue_workspace_tooltip(
                            tooltip_id,
                            title.clone(),
                            f32::from(event.position.x) + 12.0,
                            f32::from(event.position.y) + 16.0,
                            cx,
                        );
                    })
                })
                .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                    if !*hovered {
                        this.clear_workspace_tooltip(tooltip_id, cx);
                    }
                }))
                .child(monospace_datum(
                    &self.tokens,
                    label,
                    Some(mono_font),
                    MonospaceDatumOptions::new(match tone {
                        StatusTone::Success => MonospaceDatumTone::Success,
                        StatusTone::Warning => MonospaceDatumTone::Warning,
                        StatusTone::Error => MonospaceDatumTone::Error,
                        _ => MonospaceDatumTone::Muted,
                    })
                    .text_size(11.0),
                ))
                .into_any_element(),
        )
    }

    /// Capsule icon button for the command composer control row. Mirrors the
    /// 24px bordered capsules on the right of that row so the task actions read
    /// as one control group instead of flat icons next to a filled button.
    pub(super) fn terminal_command_capsule_button(
        &self,
        icon: LucideIcon,
        icon_color: Rgba,
        disabled: bool,
        tooltip_id: &'static str,
        tooltip_title: String,
        listener: impl Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = self.tokens.ui;
        self.terminal_command_icon_button(
            icon,
            icon_color,
            IconButtonOptions {
                disabled,
                background: Some(rgba((theme.bg << 8) | 0x60)),
                border: Some(rgba((theme.border << 8) | 0x80)),
                hover_background: Some(rgba((theme.bg_hover << 8) | 0x80)),
                ..IconButtonOptions::opaque_toolbar(24.0, ButtonRadius::Sm)
            },
            tooltip_id,
            tooltip_title,
            listener,
            cx,
        )
    }
}
