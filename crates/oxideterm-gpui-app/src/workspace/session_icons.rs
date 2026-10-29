use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::WorkspaceApp;
use crate::assets::{DISTRO_ICONS, DistroIcon, LucideIcon};
use gpui::{
    AnyElement, App, Context, DevicePixels, IntoElement, RenderImage, Rgba, Styled, StyledImage,
    SvgSize, img, px, size,
};

/// Edge length, in device pixels, that bundled brand artwork is rasterized at.
///
/// The largest slot drawing one of these icons is 20 logical px, so 96 px still
/// covers a 4x display. Keep this bounded and independent of the canvas each SVG
/// file declares: every cached bitmap costs `BRAND_ICON_RASTER_PX² × 4` bytes plus
/// a sprite-atlas texture of the same size, and both live for the whole process.
const BRAND_ICON_RASTER_PX: i32 = 96;

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
    /// Renders the icon at `size` logical px.
    ///
    /// Brand artwork keeps its own colors, so it cannot use the monochrome
    /// `svg()` mask path. It must not be routed back through `img(icon.path)`
    /// either: the resource-image path rasterizes a bundled SVG at the canvas
    /// its file declares, doubles it with GPUI's smooth-SVG scale factor, then
    /// pins that bitmap plus a same-size sprite-atlas texture for the lifetime
    /// of the process. `distro-icons/rocky.svg` once declared a 4096 px canvas
    /// for 192 units of artwork, so a single 14 px auto-icon row cost ~800 MB of
    /// resident memory that no panel collapse could release. Rasterizing once at
    /// [`BRAND_ICON_RASTER_PX`] and letting the layout scale the result down keeps
    /// the cost independent of how any SVG file is authored.
    pub(super) fn render(self, size: f32, color: Rgba, cx: &App) -> AnyElement {
        match self {
            Self::Lucide(icon) => WorkspaceApp::render_lucide_icon(icon, size, color),
            Self::Distro(icon) => match brand_icon_image(icon, cx) {
                Some(image) => img(image)
                    .size(px(size))
                    .object_fit(gpui::ObjectFit::Contain)
                    .into_any_element(),
                // Bundled artwork that fails to parse still needs a visible mark.
                None => WorkspaceApp::render_lucide_icon(LucideIcon::Server, size, color),
            },
        }
    }
}

fn brand_icon_image(icon: &'static DistroIcon, cx: &App) -> Option<Arc<RenderImage>> {
    fn cache() -> &'static Mutex<HashMap<&'static str, Arc<RenderImage>>> {
        static CACHE: OnceLock<Mutex<HashMap<&'static str, Arc<RenderImage>>>> = OnceLock::new();
        CACHE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    if let Some(image) = cache().lock().ok()?.get(icon.id).cloned() {
        return Some(image);
    }

    // Bundled artwork never changes, so one raster per icon id is enough and the
    // cache cannot grow beyond `DISTRO_ICONS.len()` entries.
    let renderer = cx.svg_renderer();
    let parsed = renderer.parse_svg(icon.bytes).ok()?;
    let edge = DevicePixels(BRAND_ICON_RASTER_PX);
    let image = renderer
        .render_parsed(&parsed, SvgSize::Size(size(edge, edge)))
        .ok()?;
    cache().lock().ok()?.insert(icon.id, image.clone());
    Some(image)
}

const AUTO_ICON_PLACEHOLDER_ID: &str = "rocky";

pub(super) fn resolved_session_icon(
    manual: Option<&str>,
    system_name: Option<&str>,
) -> SessionIcon {
    if let Some(manual) = manual.map(str::trim).filter(|id| !id.is_empty()) {
        return session_icon_from_id(Some(manual)).unwrap_or(LucideIcon::Server.into());
    }
    // Auto-detected icons show the placeholder distribution until the remote OS is known.
    let Some(system_name) = system_name else {
        return session_icon_from_id(Some(AUTO_ICON_PLACEHOLDER_ID))
            .unwrap_or(LucideIcon::Server.into());
    };
    detect_distro_icon(system_name)
        .map(SessionIcon::Distro)
        .unwrap_or(LucideIcon::Server.into())
}

/// Resolves which bundled distribution icon describes a detected OS name.
///
/// The name arrives as `/etc/os-release` `PRETTY_NAME` (for example
/// `Phosphor OpenBMC (Phosphor OpenBMC Project Reference Distro) 1.10.31.37`)
/// or, when that field is missing, as the bare `ID` (`openbmc-phosphor`). A
/// distribution can therefore appear anywhere in that text, not only as its
/// first word, while a derivative must still not inherit an icon just because
/// its name contains the base name (`raspbian` is not `debian`). Whole-token
/// comparison satisfies both: an icon id may match any single token, and an
/// icon's display name matches one contiguous run of tokens.
///
/// Real `os-release` values name their own distribution first, so the earliest
/// match wins: `Linux Mint (based on Debian)` stays Linux Mint even though it
/// names Debian later in the same string.
fn detect_distro_icon(system_name: &str) -> Option<&'static DistroIcon> {
    let tokens = os_name_tokens(system_name);
    DISTRO_ICONS
        .iter()
        .filter_map(|icon| distro_match_position(&tokens, icon).map(|position| (position, icon)))
        .min_by_key(|(position, _)| *position)
        .map(|(_, icon)| icon)
}

/// Position of the first token window that identifies `icon`, if any.
fn distro_match_position(tokens: &[String], icon: &DistroIcon) -> Option<usize> {
    let id_position = tokens.iter().position(|token| token == icon.id);
    let name_tokens = os_name_tokens(icon.name);
    let name_position = if name_tokens.is_empty() {
        None
    } else {
        tokens
            .windows(name_tokens.len())
            .position(|window| window == name_tokens.as_slice())
    };
    match (id_position, name_position) {
        (Some(id), Some(name)) => Some(id.min(name)),
        (position, None) | (None, position) => position,
    }
}

/// Lowercased alphanumeric tokens, so `openbmc-phosphor`, `Phosphor OpenBMC`
/// and `Rocky Linux 9.4` compare by words instead of by raw text or substring.
fn os_name_tokens(value: &str) -> Vec<String> {
    value
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

pub(super) fn detect_distro_icon_id_from_name(system_name: &str) -> Option<&'static str> {
    detect_distro_icon(system_name).map(|icon| icon.id)
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
    fn distro_resolution_matches_whole_tokens_and_keeps_manual_icons() {
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
            // BMC images name their vendor first, so the distribution is not the
            // leading word; the id token still identifies it.
            (
                "Phosphor OpenBMC (Phosphor OpenBMC Project Reference Distro) 1.10.31.37",
                "openbmc",
            ),
            ("openbmc-phosphor", "openbmc"),
            // A derivative that names its base later in the text resolves to the
            // base distribution instead of the generic server glyph.
            ("LMDE (based on Debian)", "debian"),
            ("Ubuntu-like Linux", "ubuntu"),
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
        // Partial words never identify a distribution: `raspbian` is not
        // `debian`, and `mintish` is not `mint`.
        for name in [
            "Raspbian GNU/Linux 12 (bookworm)",
            "Archcraft Linux",
            "Linux Mintish",
            "OpenWrt 23.05",
            "Pop!_OS 22.04 LTS",
            "Darwin",
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
