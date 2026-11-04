// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use super::*;
use gpui::Task;
use oxideterm_editor_core::utf16::replace_utf16;
use std::{ops::Range, path::Path};

#[derive(Default)]
/// Keeps broadcast selection semantics together so stale targets cannot widen a command.
struct TerminalBroadcastState {
    groups: super::terminal_sync_groups::TerminalSyncGroups,
    selected_group_id: Option<uuid::Uuid>,
    group_editor: Option<TerminalBroadcastGroupEditor>,
    menu_open: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::workspace) enum TerminalBroadcastGroupEditKind {
    Create,
    Rename(uuid::Uuid),
}

struct TerminalBroadcastGroupEditor {
    kind: TerminalBroadcastGroupEditKind,
    value: String,
}

/// Owns terminal-wide state that is shared across panes and surfaces.
pub(in crate::workspace) struct WorkspaceTerminalEntity {
    pub(super) last_ssh_terminal_panes: HashMap<NodeId, PaneId>,
    pub(super) cast_player: Option<terminal_cast::TerminalCastPlayerState>,
    pub(super) cast_seek_dragging: bool,
    pub(super) cast_tick_generation: u64,
    pub(super) cast_tick_scheduled: bool,
    pub(super) cast_tick_task: Option<Task<()>>,
    pub(super) quick_commands: quick_commands::TerminalQuickCommandsState,
    broadcast: TerminalBroadcastState,
}

impl WorkspaceTerminalEntity {
    pub(in crate::workspace) fn new(settings_path: &Path) -> Self {
        Self {
            last_ssh_terminal_panes: HashMap::new(),
            cast_player: None,
            cast_seek_dragging: false,
            cast_tick_generation: 0,
            cast_tick_scheduled: false,
            cast_tick_task: None,
            quick_commands: quick_commands::TerminalQuickCommandsState::load(settings_path),
            broadcast: TerminalBroadcastState::default(),
        }
    }

    pub(in crate::workspace) fn broadcast_enabled(&self) -> bool {
        self.broadcast
            .groups
            .enabled(self.broadcast.selected_group_id)
    }

    pub(in crate::workspace) fn broadcast_menu_open(&self) -> bool {
        self.broadcast.menu_open
    }

    pub(in crate::workspace) fn cast_player_open(&self) -> bool {
        self.cast_player.is_some()
    }

    pub(in crate::workspace) fn sync_groups(
        &self,
    ) -> &super::terminal_sync_groups::TerminalSyncGroups {
        &self.broadcast.groups
    }

    pub(in crate::workspace) fn sync_groups_mut(
        &mut self,
    ) -> &mut super::terminal_sync_groups::TerminalSyncGroups {
        &mut self.broadcast.groups
    }

    pub(in crate::workspace) fn selected_broadcast_group_id(&self) -> Option<uuid::Uuid> {
        self.broadcast.selected_group_id
    }

    pub(in crate::workspace) fn broadcast_group_editor(
        &self,
    ) -> Option<(TerminalBroadcastGroupEditKind, &str)> {
        self.broadcast
            .group_editor
            .as_ref()
            .map(|editor| (editor.kind, editor.value.as_str()))
    }

    pub(in crate::workspace) fn begin_broadcast_group_create(&mut self) {
        self.broadcast.group_editor = Some(TerminalBroadcastGroupEditor {
            kind: TerminalBroadcastGroupEditKind::Create,
            value: String::new(),
        });
        self.broadcast.menu_open = true;
    }

    pub(in crate::workspace) fn begin_broadcast_group_rename(
        &mut self,
        group_id: uuid::Uuid,
        name: String,
    ) {
        self.broadcast.group_editor = Some(TerminalBroadcastGroupEditor {
            kind: TerminalBroadcastGroupEditKind::Rename(group_id),
            value: name,
        });
        self.broadcast.menu_open = true;
    }

    pub(in crate::workspace) fn replace_broadcast_group_editor_text(
        &mut self,
        replacement_range: Option<Range<usize>>,
        text: &str,
    ) -> bool {
        let Some(editor) = self.broadcast.group_editor.as_mut() else {
            return false;
        };
        replace_utf16(&mut editor.value, replacement_range, text);
        true
    }

    pub(in crate::workspace) fn cancel_broadcast_group_edit(&mut self) -> bool {
        self.broadcast.group_editor.take().is_some()
    }

    pub(in crate::workspace) fn toggle_broadcast(&mut self) {
        self.broadcast
            .groups
            .toggle_enabled(self.broadcast.selected_group_id);
    }

    pub(in crate::workspace) fn select_broadcast_group(
        &mut self,
        group_id: uuid::Uuid,
        targets: &[PaneId],
    ) {
        self.broadcast.groups.initialize(Some(group_id), targets);
        self.broadcast.selected_group_id = Some(group_id);
    }

    pub(in crate::workspace) fn clear_selected_broadcast_group(&mut self) {
        // Selecting the temporary group does not stop any named group.
        self.broadcast.selected_group_id = None;
        self.broadcast.groups.initialize(None, &[]);
    }

    pub(in crate::workspace) fn dismiss_broadcast_menu(&mut self) -> bool {
        let was_open = self.broadcast.menu_open;
        self.broadcast.menu_open = false;
        self.broadcast.group_editor = None;
        was_open
    }

    pub(in crate::workspace) fn set_broadcast_menu_open(&mut self, open: bool) {
        self.broadcast.menu_open = open;
    }

    pub(in crate::workspace) fn toggle_broadcast_target(&mut self, pane_id: PaneId) {
        let group = self.broadcast.selected_group_id;
        if self
            .broadcast
            .groups
            .member(pane_id)
            .is_some_and(|member| member.group == group)
        {
            self.broadcast.groups.remove(pane_id);
        } else {
            self.broadcast.groups.add(group, pane_id);
        }
        self.broadcast.menu_open = true;
    }

    pub(in crate::workspace) fn retain_live_broadcast_targets(
        &mut self,
        live_panes: &HashSet<PaneId>,
    ) {
        self.broadcast.groups.retain_panes(live_panes);
    }

    pub(in crate::workspace) fn filter_broadcast_targets(
        &self,
        source: PaneId,
        candidates: Vec<PaneId>,
    ) -> Vec<PaneId> {
        let targets: HashSet<_> = self.broadcast.groups.targets(source).into_iter().collect();
        candidates
            .into_iter()
            .filter(|pane| targets.contains(pane))
            .collect()
    }
}

impl WorkspaceApp {
    pub(in crate::workspace) fn remember_ssh_terminal_pane(&mut self, pane: PaneId, cx: &mut App) {
        let Some(node) = self.session_id_for_pane(pane, cx).and_then(|session| {
            self.workspace_runtime
                .read(cx)
                .ssh_terminal_node_id(session)
        }) else {
            return;
        };
        self.terminal.update(cx, |terminal, _cx| {
            // Store only an identity; the file browser never retains a terminal or SSH node.
            terminal.last_ssh_terminal_panes.insert(node, pane);
        });
    }

    /// Keeps the SFTP reveal mapping in sync when the active terminal pane changes.
    pub(in crate::workspace) fn sync_active_terminal_metadata_context(&mut self, cx: &mut App) {
        if let Some(pane) = self.active_pane_id(cx) {
            self.remember_ssh_terminal_pane(pane, cx);
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use gpui::TestAppContext;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TERMINAL_TEST_SETTINGS_ID: AtomicU64 = AtomicU64::new(1);

    pub(in crate::workspace) fn new_terminal_entity(
        cx: &mut TestAppContext,
    ) -> Entity<WorkspaceTerminalEntity> {
        let settings_path = std::env::temp_dir().join(format!(
            "oxideterm-terminal-entity-tests-{}-{}.json",
            std::process::id(),
            NEXT_TERMINAL_TEST_SETTINGS_ID.fetch_add(1, Ordering::Relaxed)
        ));
        cx.new(|_cx| WorkspaceTerminalEntity::new(&settings_path))
    }

    #[gpui::test]
    fn selecting_another_group_does_not_retarget_input_and_closed_members_never_widen_it(
        cx: &mut TestAppContext,
    ) {
        let terminal = new_terminal_entity(cx);
        let first = uuid::Uuid::from_u128(1);
        let second = uuid::Uuid::from_u128(2);
        let [a, b, c, d, outside] = [1, 2, 3, 4, 5].map(PaneId);
        terminal.update(cx, |terminal, _| {
            terminal.select_broadcast_group(first, &[a, b]);
            assert!(!terminal.broadcast_enabled());
            terminal.toggle_broadcast();
            terminal.select_broadcast_group(second, &[c, d]);
            terminal.toggle_broadcast();
            assert_eq!(terminal.selected_broadcast_group_id(), Some(second));
            assert_eq!(
                terminal.filter_broadcast_targets(a, vec![b, c, d, outside]),
                vec![b]
            );
            assert_eq!(
                terminal.filter_broadcast_targets(c, vec![a, b, d, outside]),
                vec![d]
            );
            assert_eq!(
                terminal.filter_broadcast_targets(outside, vec![a, b, c, d]),
                vec![]
            );
            terminal.retain_live_broadcast_targets(&HashSet::from([a, outside]));
            assert_eq!(terminal.filter_broadcast_targets(a, vec![outside]), vec![]);
            assert!(!terminal.broadcast_enabled());
        });
    }

    #[gpui::test]
    fn empty_named_broadcast_group_stays_disabled(cx: &mut TestAppContext) {
        let terminal = new_terminal_entity(cx);
        terminal.update(cx, |terminal, _cx| {
            terminal.select_broadcast_group(uuid::Uuid::new_v4(), &[]);
            assert!(!terminal.broadcast_enabled());
            terminal.toggle_broadcast();
            assert!(!terminal.broadcast_enabled());
        });
    }
}
