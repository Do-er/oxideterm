// Copyright (C) 2026 OxideTerm contributors.
// SPDX-License-Identifier: GPL-3.0-only

use oxideterm_environment::{
    CurrentDirectoryScope, CurrentDirectorySnapshot, CurrentDirectorySource,
};
use oxideterm_ssh::NodeId;

use super::*;

pub(in crate::workspace) fn terminal_cwd_snapshot_from_pane(
    scope: CurrentDirectoryScope,
    pane: &TerminalPane,
) -> Option<CurrentDirectorySnapshot> {
    let current_cwd = pane.current_working_directory();
    let current_source = pane.current_working_directory_source();
    // A pending user-selected directory and a shell-owned OSC report are
    // more current than an asynchronous process probe.
    if pane.current_working_directory_is_pending()
        || current_source == Some(TerminalWorkingDirectorySource::ShellIntegration)
    {
        let cwd = current_cwd.clone()?;
        let source = match current_source {
            Some(TerminalWorkingDirectorySource::ShellIntegration) => {
                CurrentDirectorySource::ShellIntegration
            }
            Some(TerminalWorkingDirectorySource::VisibleCommand) => {
                CurrentDirectorySource::UserAction
            }
            Some(TerminalWorkingDirectorySource::SessionDefault) => {
                CurrentDirectorySource::SessionDefault
            }
            None => CurrentDirectorySource::VisibleText,
        };
        return CurrentDirectorySnapshot::new(scope, cwd, source);
    }
    if matches!(&scope, CurrentDirectoryScope::Local)
        && let Some(snapshot) = pane.process_info().cwd.and_then(|path| {
            CurrentDirectorySnapshot::new(
                scope.clone(),
                path.to_string_lossy().to_string(),
                CurrentDirectorySource::ProcessFallback,
            )
        })
    {
        return Some(snapshot);
    }
    if let Some(cwd) = current_cwd {
        let source = match current_source {
            Some(TerminalWorkingDirectorySource::VisibleCommand) => {
                CurrentDirectorySource::UserAction
            }
            Some(TerminalWorkingDirectorySource::SessionDefault) => {
                CurrentDirectorySource::SessionDefault
            }
            _ => CurrentDirectorySource::VisibleText,
        };
        return CurrentDirectorySnapshot::new(scope, cwd, source);
    }
    None
}

impl WorkspaceApp {
    pub(in crate::workspace) fn terminal_current_directory_awareness_enabled(&self) -> bool {
        self.settings_store
            .settings()
            .terminal
            .command_bar
            .current_directory_awareness
    }

    pub(in crate::workspace) fn active_terminal_cwd_snapshot(
        &self,
        cx: &App,
    ) -> Option<CurrentDirectorySnapshot> {
        let (scope, pane_id) = self.active_terminal_cwd_scope_and_pane(cx)?;
        self.terminal_cwd_snapshot_for_pane(scope, pane_id, cx)
    }

    pub(in crate::workspace) fn active_local_terminal_cwd_path(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        if !self.terminal_current_directory_awareness_enabled() {
            return None;
        }
        let (scope, pane_id) = self.active_terminal_cwd_scope_and_pane(cx)?;
        if !matches!(&scope, CurrentDirectoryScope::Local) {
            return None;
        }
        self.terminal_cwd_snapshot_for_pane(scope, pane_id, cx)
            .map(|snapshot| snapshot.path().to_string())
    }

    pub(in crate::workspace) fn active_ssh_terminal_cwd_path_for_node(
        &self,
        node_id: &NodeId,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        if !self.terminal_current_directory_awareness_enabled() {
            return None;
        }
        let (scope, pane_id) = self.active_terminal_cwd_scope_and_pane(cx)?;
        match &scope {
            CurrentDirectoryScope::SshNode(active_node_id) if active_node_id == &node_id.0 => {}
            _ => return None,
        }
        self.terminal_cwd_snapshot_for_pane(scope, pane_id, cx)
            .map(|snapshot| snapshot.path().to_string())
    }

    pub(in crate::workspace) fn active_terminal_cwd_scope_and_pane(
        &self,
        cx: &App,
    ) -> Option<(CurrentDirectoryScope, PaneId)> {
        let tab = self.active_tab(cx)?;
        let pane_id = tab.active_pane_id?;
        let scope = match self.terminal_kind_for_pane(pane_id, cx)? {
            oxideterm_terminal::TerminalSessionKind::LocalPty => CurrentDirectoryScope::Local,
            oxideterm_terminal::TerminalSessionKind::SshPty => {
                let session_id = self.active_terminal_session_id(cx)?;
                let node_id = self
                    .workspace_runtime
                    .read(cx)
                    .ssh_terminal_node_id(session_id)?;
                CurrentDirectoryScope::ssh_node(node_id.0)
            }
            _ => return None,
        };
        Some((scope, pane_id))
    }

    fn terminal_cwd_snapshot_for_pane(
        &self,
        scope: CurrentDirectoryScope,
        pane_id: PaneId,
        cx: &App,
    ) -> Option<CurrentDirectorySnapshot> {
        let tab_host = self.tab_host.read(cx);
        let pane = tab_host.panes().get(&pane_id)?.read(cx);
        terminal_cwd_snapshot_from_pane(scope, pane)
    }
}
