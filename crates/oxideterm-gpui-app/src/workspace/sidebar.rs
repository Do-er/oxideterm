use super::*;
use oxideterm_gpui_ui::button::ButtonRadius;
use oxideterm_gpui_ui::{
    IconButtonOptions, TreeBranchMetrics, color_for_background, color_for_background_or_alpha,
    tree_child,
};

// Active sessions are a high-frequency navigator, so keep its density closer
// to a compact desktop connection list than to a settings or form surface.
const SESSION_TREE_NODE_HEIGHT: f32 = 28.0;
const SESSION_TREE_ITEM_HEIGHT: f32 = 24.0;
const SESSION_TREE_TEXT_SIZE: f32 = 12.0;
const SESSION_TREE_META_TEXT_SIZE: f32 = 10.0;
const SESSION_TREE_ICON_SIZE: f32 = 14.0;
const SESSION_TREE_CHILD_ICON_SIZE: f32 = 12.0;
// Primary sidebar content needs a small inset below the header divider so the
// first interactive surface does not visually merge with workspace chrome.
const PRIMARY_SIDEBAR_CONTENT_TOP_INSET: f32 = 4.0;
// Selected terminal row highlight alpha in the active sessions tree.
const ACTIVE_SESSION_TERMINAL_ACTIVE_BG_ALPHA: u32 = 0x24;
// Tauri EventLogPanel rows use `min-h-[24px]` with `px-3 py-1`; keep the
// native estimate next to the shared virtual-list call so scroll-to-index and
// sticky-bottom behavior stay browser-like.
const EMBEDDED_SFTP_MIN_SESSION_FRACTION: f32 = 0.2;
const EMBEDDED_SFTP_MAX_SESSION_FRACTION: f32 = 0.75;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SidebarSection {
    Forwards,
    Terminal,
    Network,
    Assistant,
    HostTools,
    Automation,
    Workspace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContextSidebarPanel {
    Sessions,
    Connections,
    Assistant,
    HostTools,
    PluginPanel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContextSidebarTool {
    Monitor,
    Gpu,
    Processes,
    Services,
    Logs,
    Tmux,
    Docker,
    Ports,
    Schedules,
    Filesystems,
    Packages,
}

#[derive(Clone, Copy)]
pub(in crate::workspace) struct SessionStatusStyle {
    icon: LucideIcon,
    dot_color: u32,
    ring: bool,
}

#[derive(Clone, Copy)]
pub(in crate::workspace) enum SessionActionVariant {
    Primary,
    Danger,
}

impl SidebarSection {
    pub(super) fn from_settings_key(key: &str) -> Self {
        match key {
            // Retired primary-sidebar panels restore the remaining blank body;
            // their features now live outside this sidebar.
            "connections" | "saved" | "sftp" | "extensions" | "cloud_sync" | "runtime"
            | "knowledge" | "files" | "notifications" | "settings" | "sessions" => Self::Forwards,
            "forwards" => Self::Forwards,
            "connection_pool" | "terminal" => Self::Terminal,
            // Retired health-page keys now restore the Host Tools replacement,
            // which is also where the standalone monitor entry ended up.
            "connection_monitor" | "activity" | "monitor" => Self::HostTools,
            "network" | "topology" => Self::Network,
            "ai" | "assistant" => Self::Assistant,
            "host_tools" => Self::HostTools,
            "automation" => Self::Automation,
            "workspace" => Self::Workspace,
            _ => Self::Forwards,
        }
    }

    pub(super) fn as_settings_key(self) -> &'static str {
        match self {
            Self::Forwards => "forwards",
            Self::Terminal => "connection_pool",
            Self::Network => "topology",
            Self::Assistant => "ai",
            Self::HostTools => "host_tools",
            Self::Automation => "automation",
            Self::Workspace => "workspace",
        }
    }
}

impl WorkspaceApp {
    pub(in crate::workspace) fn effective_sidebar_panel_section(&self) -> SidebarSection {
        match self.active_sidebar_section {
            SidebarSection::Forwards => self.active_sidebar_section,
            // Tauri separates activity-bar actions from sidebar sections. Keep
            // action-only entries from replacing the remaining sidebar body.
            SidebarSection::Terminal
            | SidebarSection::Network
            | SidebarSection::Assistant
            | SidebarSection::HostTools
            | SidebarSection::Automation
            | SidebarSection::Workspace => SidebarSection::Forwards,
        }
    }
}

mod activity;
mod ai;
mod helpers;
mod region;
pub(in crate::workspace) use region::sidebar_resize_hotzone_chrome;
mod sessions;
mod state;
mod titlebar;
mod tools;
pub(in crate::workspace) use titlebar::{
    client_titlebar_button_layout, handle_window_drag_mouse_down,
};

pub(in crate::workspace) use ai::{
    AcpApplicationToolTurn, AiChatPromptKeyAction, AiCompactionDelivery,
    AiCompactionDeliverySender, AiInlinePanelState, AiInputVisualLine, AiStreamDelivery,
    AiStreamDeliveryEvent, AiStreamDeliverySender, ai_chat_prompt_key_action,
    ai_input_line_index_for_offset, ai_input_visual_lines, ai_now_ms,
    handle_acp_application_tool_call,
};
use helpers::*;
pub(in crate::workspace) use state::{
    clamp_responsive_sidebar_width, context_sidebar_panel_visible,
};

#[cfg(test)]
mod sidebar_persistence_tests {
    use super::SidebarSection;

    #[test]
    fn sidebar_sections_roundtrip_persisted_settings_keys() {
        let sections = [
            SidebarSection::Forwards,
            SidebarSection::Terminal,
            SidebarSection::Network,
            SidebarSection::Assistant,
            SidebarSection::HostTools,
            SidebarSection::Automation,
            SidebarSection::Workspace,
        ];

        for section in sections {
            assert_eq!(
                SidebarSection::from_settings_key(section.as_settings_key()),
                section
            );
        }
    }

    #[test]
    fn retired_sidebar_keys_restore_blank_panel() {
        for key in [
            "sessions",
            "connections",
            "saved",
            "sftp",
            "extensions",
            "cloud_sync",
            "runtime",
            "knowledge",
            "files",
            "notifications",
            "settings",
        ] {
            assert_eq!(
                SidebarSection::from_settings_key(key),
                SidebarSection::Forwards
            );
        }
    }

    #[test]
    fn retired_monitor_key_restores_host_tools() {
        assert_eq!(
            SidebarSection::from_settings_key("monitor"),
            SidebarSection::HostTools
        );
    }
}
