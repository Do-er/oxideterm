use super::WorkspaceApp;
use crate::assets::{DISTRO_ICONS, DistroIcon, LucideIcon};
use gpui::{AnyElement, Context, IntoElement, Rgba, Styled, StyledImage, img, px};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SessionIcon {
    Lucide(LucideIcon),
    Distro(&'static DistroIcon),
}

impl From<LucideIcon> for SessionIcon {
    fn from(icon: LucideIcon) -> Self {
        Self::Lucide(icon)
    }
}

impl SessionIcon {
    pub(super) fn render(self, size: f32, color: Rgba) -> AnyElement {
        match self {
            Self::Lucide(icon) => WorkspaceApp::render_lucide_icon(icon, size, color),
            // Use the color-preserving image renderer; a theme mask would alter the brand artwork.
            Self::Distro(icon) => img(icon.path)
                .size(px(size))
                .object_fit(gpui::ObjectFit::Contain)
                .into_any_element(),
        }
    }
}

pub(super) fn resolved_session_icon(
    manual: Option<&str>,
    system_name: Option<&str>,
) -> SessionIcon {
    if let Some(manual) = manual.map(str::trim).filter(|id| !id.is_empty()) {
        return session_icon_from_id(Some(manual)).unwrap_or(LucideIcon::Server.into());
    }
    // Match the distribution's name, not ID_LIKE or an incidental substring in a derivative name.
    let Some(system_name) = system_name else {
        return LucideIcon::Server.into();
    };
    DISTRO_ICONS
        .iter()
        .find(|icon| {
            let mut words = system_name.split_whitespace();
            if words
                .next()
                .is_some_and(|word| word.eq_ignore_ascii_case(icon.id))
            {
                return true;
            }
            let mut words = system_name.split_whitespace();
            icon.name.split_whitespace().all(|part| {
                words
                    .next()
                    .is_some_and(|word| word.eq_ignore_ascii_case(part))
            })
        })
        .map(SessionIcon::Distro)
        .unwrap_or(LucideIcon::Server.into())
}

pub(super) fn detect_distro_icon_id_from_name(system_name: &str) -> Option<&'static str> {
    DISTRO_ICONS
        .iter()
        .find(|icon| {
            let mut words = system_name.split_whitespace();
            if words
                .next()
                .is_some_and(|word| word.eq_ignore_ascii_case(icon.id))
            {
                return true;
            }
            let mut words = system_name.split_whitespace();
            icon.name.split_whitespace().all(|part| {
                words
                    .next()
                    .is_some_and(|word| word.eq_ignore_ascii_case(part))
            })
        })
        .map(|icon| icon.id)
}

pub(super) fn is_distro_icon_id(id: &str) -> bool {
    DISTRO_ICONS.iter().any(|icon| icon.id == id)
}

impl WorkspaceApp {
    pub(in crate::workspace) fn try_sync_detected_session_icon(
        &mut self,
        node_id: &oxideterm_ssh::NodeId,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(saved_id) = self
            .ssh_nodes
            .get(node_id)
            .and_then(|node| node.saved_connection_id.clone())
        else {
            return false;
        };
        let Some(connection_id) = self.node_router.connection_id_for_node(node_id) else {
            return false;
        };
        let Some(handle) = self.ssh_registry.get(&connection_id) else {
            return false;
        };
        let Some(env) = handle.remote_env() else {
            return false;
        };
        let Some(detected_id) = env
            .os_version
            .as_deref()
            .and_then(detect_distro_icon_id_from_name)
        else {
            return false;
        };

        let Some(conn) = self.connection_store.get(&saved_id).cloned() else {
            return false;
        };

        // If user configured a manual non-distro icon, respect their explicit choice.
        if let Some(ref curr) = conn.icon {
            if !is_distro_icon_id(curr) {
                return true;
            }
            if curr == detected_id {
                return true;
            }
        }

        let request = oxideterm_connections::SaveConnectionRequest {
            totp_credential_id: None,
            id: Some(conn.id),
            name: conn.name,
            group: conn.group,
            notes: conn.notes,
            host: conn.host,
            port: conn.port,
            username: conn.username,
            auth: conn.auth,
            proxy_chain: conn.proxy_chain,
            upstream_proxy: conn.upstream_proxy,
            proxy_command: conn.proxy_command,
            color: conn.color,
            icon_background_color: conn.icon_background_color,
            icon: Some(detected_id.to_string()),
            tags: conn.tags,
            connect_timeout_seconds: conn.options.effective_connect_timeout_seconds(),
            agent_forwarding: conn.options.agent_forwarding,
            identity_agent: conn.options.identity_agent,
            agent_forwarding_socket: conn.options.agent_forwarding_socket,
            legacy_ssh_compatibility: conn.options.legacy_ssh_compatibility,
            ssh_algorithms: conn.options.ssh_algorithms,
            dedicated_new_terminal_connection: conn.options.dedicated_new_terminal_connection,
            ssh_channel_strategy: conn.options.ssh_channel_strategy,
            x11_forwarding: conn.options.x11_forwarding,
            post_connect_command: conn.post_connect_command,
            terminal: conn.options.terminal,
        };

        if self.connection_store.upsert(request).is_ok() {
            self.queue_cloud_sync_dirty_refresh(cx);
            cx.notify();
            return true;
        }
        false
    }

    pub(in crate::workspace) fn spawn_detected_session_icon_sync(
        &self,
        node_id: oxideterm_ssh::NodeId,
        cx: &mut Context<Self>,
    ) {
        let entity = cx.entity().clone();
        cx.spawn(async move |_, cx| {
            for _ in 0..30 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let done = entity.update(cx, |this, cx| {
                    this.try_sync_detected_session_icon(&node_id, cx)
                });
                if done {
                    break;
                }
            }
        })
        .detach();
    }

    pub(in crate::workspace) fn node_session_icon(
        &self,
        node_id: &oxideterm_ssh::NodeId,
    ) -> SessionIcon {
        if node_id.0 == "local-terminal-group" {
            return LucideIcon::Terminal.into();
        }
        if let Some(profile_id) = node_id.0.strip_prefix("local-profile-") {
            let manual = self
                .connection_store
                .local_terminal_profiles()
                .iter()
                .find(|profile| profile.id == profile_id)
                .and_then(|profile| profile.icon.as_deref());
            return manual
                .and_then(|icon| session_icon_from_id(Some(icon)))
                .unwrap_or(LucideIcon::Terminal.into());
        }
        let manual = self
            .ssh_nodes
            .get(node_id)
            .and_then(|node| node.saved_connection_id.as_deref())
            .and_then(|id| self.connection_store.get(id))
            .and_then(|connection| connection.icon.as_deref());
        let environment = self
            .node_router
            .connection_id_for_node(node_id)
            .and_then(|id| self.ssh_registry.get(&id))
            .and_then(|handle| handle.remote_env());
        resolved_session_icon(
            manual,
            environment
                .as_ref()
                .and_then(|env| env.os_version.as_deref()),
        )
    }
}

#[derive(Clone, Copy)]
pub(super) struct SessionIconChoice {
    pub id: &'static str,
    pub icon: LucideIcon,
}

// Persist stable string ids instead of enum names so stored connections can
// survive icon rendering changes and later brand-icon additions.
pub(super) const SESSION_ICON_CHOICES: &[SessionIconChoice] = &[
    SessionIconChoice {
        id: "activity",
        icon: LucideIcon::Activity,
    },
    SessionIconChoice {
        id: "app-window",
        icon: LucideIcon::AppWindow,
    },
    SessionIconChoice {
        id: "arrow-left-right",
        icon: LucideIcon::ArrowLeftRight,
    },
    SessionIconChoice {
        id: "arrow-up-down",
        icon: LucideIcon::ArrowUpDown,
    },
    SessionIconChoice {
        id: "book-open",
        icon: LucideIcon::BookOpen,
    },
    SessionIconChoice {
        id: "bot",
        icon: LucideIcon::Bot,
    },
    SessionIconChoice {
        id: "brain",
        icon: LucideIcon::Brain,
    },
    SessionIconChoice {
        id: "cable",
        icon: LucideIcon::Cable,
    },
    SessionIconChoice {
        id: "check-circle",
        icon: LucideIcon::CheckCircle,
    },
    SessionIconChoice {
        id: "circle",
        icon: LucideIcon::Circle,
    },
    SessionIconChoice {
        id: "clock",
        icon: LucideIcon::Clock,
    },
    SessionIconChoice {
        id: "cloud",
        icon: LucideIcon::Cloud,
    },
    SessionIconChoice {
        id: "code",
        icon: LucideIcon::Code2,
    },
    SessionIconChoice {
        id: "cpu",
        icon: LucideIcon::Cpu,
    },
    SessionIconChoice {
        id: "download",
        icon: LucideIcon::Download,
    },
    SessionIconChoice {
        id: "file",
        icon: LucideIcon::File,
    },
    SessionIconChoice {
        id: "file-archive",
        icon: LucideIcon::FileArchive,
    },
    SessionIconChoice {
        id: "file-audio",
        icon: LucideIcon::FileAudio,
    },
    SessionIconChoice {
        id: "file-code",
        icon: LucideIcon::FileCode,
    },
    SessionIconChoice {
        id: "file-image",
        icon: LucideIcon::FileImage,
    },
    SessionIconChoice {
        id: "file-json",
        icon: LucideIcon::FileJson,
    },
    SessionIconChoice {
        id: "file-lock",
        icon: LucideIcon::FileLock,
    },
    SessionIconChoice {
        id: "file-play",
        icon: LucideIcon::FilePlay,
    },
    SessionIconChoice {
        id: "file-plus",
        icon: LucideIcon::FilePlus,
    },
    SessionIconChoice {
        id: "file-spreadsheet",
        icon: LucideIcon::FileSpreadsheet,
    },
    SessionIconChoice {
        id: "file-terminal",
        icon: LucideIcon::FileTerminal,
    },
    SessionIconChoice {
        id: "file-text",
        icon: LucideIcon::FileText,
    },
    SessionIconChoice {
        id: "file-video",
        icon: LucideIcon::FileVideo,
    },
    SessionIconChoice {
        id: "folder",
        icon: LucideIcon::Folder,
    },
    SessionIconChoice {
        id: "folder-archive",
        icon: LucideIcon::FolderArchive,
    },
    SessionIconChoice {
        id: "folder-input",
        icon: LucideIcon::FolderInput,
    },
    SessionIconChoice {
        id: "folder-open",
        icon: LucideIcon::FolderOpen,
    },
    SessionIconChoice {
        id: "folder-plus",
        icon: LucideIcon::FolderPlus,
    },
    SessionIconChoice {
        id: "folder-sync",
        icon: LucideIcon::FolderSync,
    },
    SessionIconChoice {
        id: "gauge",
        icon: LucideIcon::Gauge,
    },
    SessionIconChoice {
        id: "git-fork",
        icon: LucideIcon::GitFork,
    },
    SessionIconChoice {
        id: "hard-drive",
        icon: LucideIcon::HardDrive,
    },
    SessionIconChoice {
        id: "hash",
        icon: LucideIcon::Hash,
    },
    SessionIconChoice {
        id: "history",
        icon: LucideIcon::History,
    },
    SessionIconChoice {
        id: "home",
        icon: LucideIcon::Home,
    },
    SessionIconChoice {
        id: "image",
        icon: LucideIcon::Image,
    },
    SessionIconChoice {
        id: "inbox",
        icon: LucideIcon::Inbox,
    },
    SessionIconChoice {
        id: "key",
        icon: LucideIcon::Key,
    },
    SessionIconChoice {
        id: "key-round",
        icon: LucideIcon::KeyRound,
    },
    SessionIconChoice {
        id: "keyboard",
        icon: LucideIcon::Keyboard,
    },
    SessionIconChoice {
        id: "layers",
        icon: LucideIcon::Layers,
    },
    SessionIconChoice {
        id: "layout-list",
        icon: LucideIcon::LayoutList,
    },
    SessionIconChoice {
        id: "link",
        icon: LucideIcon::Link2,
    },
    SessionIconChoice {
        id: "list-checks",
        icon: LucideIcon::ListChecks,
    },
    SessionIconChoice {
        id: "list-tree",
        icon: LucideIcon::ListTree,
    },
    SessionIconChoice {
        id: "lock",
        icon: LucideIcon::Lock,
    },
    SessionIconChoice {
        id: "memory-stick",
        icon: LucideIcon::MemoryStick,
    },
    SessionIconChoice {
        id: "message-square",
        icon: LucideIcon::MessageSquare,
    },
    SessionIconChoice {
        id: "monitor",
        icon: LucideIcon::Monitor,
    },
    SessionIconChoice {
        id: "network",
        icon: LucideIcon::Network,
    },
    SessionIconChoice {
        id: "pin",
        icon: LucideIcon::Pin,
    },
    SessionIconChoice {
        id: "power",
        icon: LucideIcon::Power,
    },
    SessionIconChoice {
        id: "puzzle",
        icon: LucideIcon::Puzzle,
    },
    SessionIconChoice {
        id: "radio",
        icon: LucideIcon::Radio,
    },
    SessionIconChoice {
        id: "refresh",
        icon: LucideIcon::RefreshCw,
    },
    SessionIconChoice {
        id: "rocket",
        icon: LucideIcon::Rocket,
    },
    SessionIconChoice {
        id: "save",
        icon: LucideIcon::Save,
    },
    SessionIconChoice {
        id: "search",
        icon: LucideIcon::Search,
    },
    SessionIconChoice {
        id: "server",
        icon: LucideIcon::Server,
    },
    SessionIconChoice {
        id: "shield",
        icon: LucideIcon::Shield,
    },
    SessionIconChoice {
        id: "shield-alert",
        icon: LucideIcon::ShieldAlert,
    },
    SessionIconChoice {
        id: "shield-check",
        icon: LucideIcon::ShieldCheck,
    },
    SessionIconChoice {
        id: "shield-off",
        icon: LucideIcon::ShieldOff,
    },
    SessionIconChoice {
        id: "sparkles",
        icon: LucideIcon::Sparkles,
    },
    SessionIconChoice {
        id: "split-horizontal",
        icon: LucideIcon::SplitSquareHorizontal,
    },
    SessionIconChoice {
        id: "split-vertical",
        icon: LucideIcon::SplitSquareVertical,
    },
    SessionIconChoice {
        id: "star",
        icon: LucideIcon::Star,
    },
    SessionIconChoice {
        id: "stop-circle",
        icon: LucideIcon::StopCircle,
    },
    SessionIconChoice {
        id: "terminal",
        icon: LucideIcon::Terminal,
    },
    SessionIconChoice {
        id: "upload",
        icon: LucideIcon::Upload,
    },
    SessionIconChoice {
        id: "wifi",
        icon: LucideIcon::Wifi,
    },
    SessionIconChoice {
        id: "wifi-off",
        icon: LucideIcon::WifiOff,
    },
    SessionIconChoice {
        id: "wrench",
        icon: LucideIcon::Wrench,
    },
    SessionIconChoice {
        id: "zap",
        icon: LucideIcon::Zap,
    },
];

pub(super) fn session_icon_from_id(icon_id: Option<&str>) -> Option<SessionIcon> {
    let icon_id = icon_id?.trim();
    if let Some(icon) = DISTRO_ICONS.iter().find(|icon| icon.id == icon_id) {
        return Some(SessionIcon::Distro(icon));
    }
    let legacy_icon = match icon_id {
        "database" => Some(LucideIcon::Archive),
        "docker" => Some(LucideIcon::Layers),
        "kubernetes" => Some(LucideIcon::Puzzle),
        _ => None,
    };
    if legacy_icon.is_some() {
        return legacy_icon.map(SessionIcon::Lucide);
    }
    SESSION_ICON_CHOICES
        .iter()
        .find(|choice| choice.id == icon_id)
        .map(|choice| SessionIcon::Lucide(choice.icon))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distro_resolution_preserves_manual_icons_and_does_not_guess_derivatives() {
        for (name, id) in [
            ("Ubuntu 24.04 LTS", "ubuntu"),
            ("Arch Linux", "archlinux"),
            ("archlinux", "archlinux"),
            ("Debian GNU/Linux 13 (trixie)", "debian"),
            ("Gentoo Linux", "gentoo"),
            ("NixOS 25.11", "nixos"),
            ("Rocky Linux 9.6 (Blue Onyx)", "rocky"),
            ("Linux Mint 22.2", "linuxmint"),
            ("Linux Mint (based on Debian)", "linuxmint"),
            ("linuxmint", "linuxmint"),
        ] {
            let SessionIcon::Distro(icon) = resolved_session_icon(None, Some(name)) else {
                panic!("missing distribution icon for {name}");
            };
            assert_eq!(icon.id, id, "{name}");
        }
        assert_eq!(
            resolved_session_icon(Some("terminal"), Some("Debian GNU/Linux")),
            SessionIcon::Lucide(LucideIcon::Terminal)
        );
        let SessionIcon::Distro(manual) =
            resolved_session_icon(Some("gentoo"), Some("Debian GNU/Linux"))
        else {
            panic!("manual distribution icon was not preserved");
        };
        assert_eq!(manual.id, "gentoo");
        for name in [
            "Ubuntu-like Linux",
            "Archcraft Linux",
            "Linux Mintish",
            "LMDE (based on Debian)",
            "unknown",
            "",
        ] {
            assert_eq!(
                resolved_session_icon(None, Some(name)),
                SessionIcon::Lucide(LucideIcon::Server)
            );
        }
        assert_eq!(
            resolved_session_icon(Some("unavailable-icon"), Some("NixOS")),
            SessionIcon::Lucide(LucideIcon::Server)
        );
    }
}
