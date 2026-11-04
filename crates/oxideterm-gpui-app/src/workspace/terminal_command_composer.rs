// Copyright (C) 2026 AnalyseDeCircuit
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use gpui::{AppContext, Context, Entity, Task, Timer, WeakEntity};
use oxideterm_gpui_editor::{
    EditorContextMenuLabels, EditorPresentation, EditorSettings, TextEditorView,
};
use oxideterm_gpui_terminal::TerminalPane;
use oxideterm_gpui_ui::text_input::TextInputViewport;
use oxideterm_terminal::{
    TerminalSendFrame, TerminalSendInputMode, TerminalSendPacing, TerminalSendPlanError,
    build_terminal_send_plan,
};
use oxideterm_theme::ThemeTokens;
use oxideterm_workspace::PaneId;
use zeroize::Zeroizing;

pub(super) const TERMINAL_COMPOSER_DEFAULT_HEIGHT: f32 = 240.0;
pub(super) const TERMINAL_COMPOSER_MIN_HEIGHT: f32 = 200.0;
pub(super) const TERMINAL_COMPOSER_MAX_VIEWPORT_RATIO: f32 = 0.65;
const TERMINAL_COMPOSER_MIN_TERMINAL_HEIGHT: f32 = 180.0;
const TERMINAL_COMPOSER_WINDOW_CHROME_RESERVE: f32 = 96.0;
pub(super) const TERMINAL_COMPOSER_RESIZE_HOTZONE_HEIGHT: f32 = 14.0;
pub(super) const TERMINAL_COMPOSER_MIN_INTERVAL_MS: u64 = 20;
pub(super) const TERMINAL_COMPOSER_MAX_INTERVAL_MS: u64 = 60_000;
pub(super) const TERMINAL_COMPOSER_MAX_REPEAT_COUNT: u32 = 9_999;
const TERMINAL_COMPOSER_PROGRESS_NOTIFY_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct TerminalCommandComposerId(pub(super) u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminalCommandComposerTargetScope {
    Current,
    All,
    Broadcast,
    Selected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminalCommandComposerStatus {
    Idle,
    Running,
    Completed,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminalCommandComposerFailure {
    EmptyInput,
    InvalidHex,
    NoTargets,
    TargetBusy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalCommandComposerOutcome {
    Completed,
    Stopped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalCommandComposerLayout {
    Hidden,
    Compact,
    Expanded,
}

impl TerminalCommandComposerLayout {
    fn toggled_visibility(self) -> Self {
        if self == Self::Hidden {
            Self::Compact
        } else {
            Self::Hidden
        }
    }

    fn with_expanded(self, expanded: bool) -> Self {
        if expanded {
            Self::Expanded
        } else if self == Self::Hidden {
            Self::Hidden
        } else {
            Self::Compact
        }
    }

    fn editor_presentation(self) -> EditorPresentation {
        if self == Self::Expanded {
            EditorPresentation::Document
        } else {
            EditorPresentation::Inline
        }
    }
}

#[derive(Clone)]
pub(super) struct TerminalCommandComposerTarget {
    pub(super) pane_id: PaneId,
    pub(super) pane: WeakEntity<TerminalPane>,
}

#[derive(Clone, Copy)]
struct TerminalCommandComposerResizeDrag {
    start_cursor_y: gpui::Pixels,
    start_height: f32,
}

#[derive(Clone)]
pub(super) struct TerminalCommandComposerDocumentSnapshot {
    pub(super) id: TerminalCommandComposerId,
    pub(super) editor: Entity<TextEditorView>,
    pub(super) input_mode: TerminalSendInputMode,
    pub(super) pacing: TerminalSendPacing,
    pub(super) interval_ms: u64,
    pub(super) repeat_count: u32,
    pub(super) target_scope: TerminalCommandComposerTargetScope,
    pub(super) selected_targets: HashSet<PaneId>,
    pub(super) status: TerminalCommandComposerStatus,
    pub(super) failure: Option<TerminalCommandComposerFailure>,
    pub(super) completed_units: u64,
    pub(super) total_units: u64,
    pub(super) accepted_writes: u64,
    pub(super) skipped_writes: u64,
}

struct TerminalCommandComposerDocument {
    id: TerminalCommandComposerId,
    editor: Entity<TextEditorView>,
    compact_draft: Zeroizing<String>,
    compact_viewport: TextInputViewport,
    input_mode: TerminalSendInputMode,
    pacing: TerminalSendPacing,
    interval_ms: u64,
    repeat_count: u32,
    target_scope: TerminalCommandComposerTargetScope,
    selected_targets: HashSet<PaneId>,
    status: TerminalCommandComposerStatus,
    failure: Option<TerminalCommandComposerFailure>,
    completed_units: u64,
    total_units: u64,
    accepted_writes: u64,
    skipped_writes: u64,
    generation: u64,
    last_progress_notify: Option<Instant>,
}

impl TerminalCommandComposerDocument {
    fn snapshot(&self) -> TerminalCommandComposerDocumentSnapshot {
        TerminalCommandComposerDocumentSnapshot {
            id: self.id,
            editor: self.editor.clone(),
            input_mode: self.input_mode,
            pacing: self.pacing,
            interval_ms: self.interval_ms,
            repeat_count: self.repeat_count,
            target_scope: self.target_scope,
            selected_targets: self.selected_targets.clone(),
            status: self.status,
            failure: self.failure,
            completed_units: self.completed_units,
            total_units: self.total_units,
            accepted_writes: self.accepted_writes,
            skipped_writes: self.skipped_writes,
        }
    }
}

struct TerminalCommandComposerRunTarget {
    pane_id: PaneId,
    pane: WeakEntity<TerminalPane>,
}

/// Owns composer documents, target snapshots, timers, cancellation, and progress.
pub(super) struct TerminalCommandComposerEntity {
    layout: TerminalCommandComposerLayout,
    compact_focused: bool,
    compact_suggestions_open: bool,
    compact_suggestion_highlighted: Option<usize>,
    panel_height: f32,
    resize_drag: Option<TerminalCommandComposerResizeDrag>,
    documents: Vec<TerminalCommandComposerDocument>,
    active_document_id: TerminalCommandComposerId,
    active_tasks: HashMap<TerminalCommandComposerId, Task<()>>,
    target_owners: HashMap<PaneId, TerminalCommandComposerId>,
    next_document_id: u64,
    compact_editor_placeholder: String,
    expanded_editor_placeholder: String,
    editor_context_menu_labels: EditorContextMenuLabels,
    editor_tokens: ThemeTokens,
}

impl TerminalCommandComposerEntity {
    pub(super) fn new(
        tokens: ThemeTokens,
        compact_editor_placeholder: String,
        expanded_editor_placeholder: String,
        editor_context_menu_labels: EditorContextMenuLabels,
        cx: &mut Context<Self>,
    ) -> Self {
        let first_id = TerminalCommandComposerId(1);
        let first = Self::new_document(
            first_id,
            tokens,
            &compact_editor_placeholder,
            &editor_context_menu_labels,
            TerminalCommandComposerLayout::Compact.editor_presentation(),
            cx,
        );
        Self {
            layout: TerminalCommandComposerLayout::Compact,
            compact_focused: false,
            compact_suggestions_open: false,
            compact_suggestion_highlighted: None,
            panel_height: TERMINAL_COMPOSER_DEFAULT_HEIGHT,
            resize_drag: None,
            documents: vec![first],
            active_document_id: first_id,
            active_tasks: HashMap::new(),
            target_owners: HashMap::new(),
            next_document_id: 2,
            compact_editor_placeholder,
            expanded_editor_placeholder,
            editor_context_menu_labels,
            editor_tokens: tokens,
        }
    }

    fn new_document(
        id: TerminalCommandComposerId,
        tokens: ThemeTokens,
        placeholder: &str,
        context_menu_labels: &EditorContextMenuLabels,
        presentation: EditorPresentation,
        cx: &mut Context<Self>,
    ) -> TerminalCommandComposerDocument {
        let placeholder = placeholder.to_string();
        let context_menu_labels = context_menu_labels.clone();
        let editor = cx.new(|cx| {
            let mut editor = TextEditorView::new(String::new(), &tokens, cx);
            let settings = EditorSettings {
                soft_wrap: false,
                indentation_markers: false,
                highlight_special_chars: false,
                highlight_current_line: false,
                placeholder: Some(placeholder),
                ..EditorSettings::default()
            };
            editor.set_settings(settings, cx);
            editor.set_border_visible(false);
            editor.set_presentation(presentation, cx);
            editor.set_context_menu_labels(context_menu_labels);
            editor
        });
        TerminalCommandComposerDocument {
            id,
            editor,
            compact_draft: Zeroizing::new(String::new()),
            compact_viewport: TextInputViewport::default(),
            input_mode: TerminalSendInputMode::Text,
            pacing: TerminalSendPacing::Line,
            interval_ms: 1_000,
            repeat_count: 1,
            target_scope: TerminalCommandComposerTargetScope::Current,
            selected_targets: HashSet::new(),
            status: TerminalCommandComposerStatus::Idle,
            failure: None,
            completed_units: 0,
            total_units: 0,
            accepted_writes: 0,
            skipped_writes: 0,
            generation: 0,
            last_progress_notify: None,
        }
    }

    pub(super) fn is_expanded(&self) -> bool {
        self.layout == TerminalCommandComposerLayout::Expanded
    }

    pub(super) fn is_visible(&self) -> bool {
        self.layout != TerminalCommandComposerLayout::Hidden
    }

    pub(super) fn toggle_visible(&mut self, cx: &mut Context<Self>) -> bool {
        let next_layout = self.layout.toggled_visibility();
        self.sync_text_for_layout_change(next_layout, cx);
        self.layout = next_layout;
        self.compact_focused = false;
        self.dismiss_compact_suggestions();
        // Hiding changes presentation only; documents and running jobs stay owned here.
        self.resize_drag = None;
        self.sync_editor_presentation(cx);
        cx.notify();
        self.is_visible()
    }

    pub(super) fn set_expanded(&mut self, expanded: bool, cx: &mut Context<Self>) {
        let next_layout = self.layout.with_expanded(expanded);
        if self.layout == next_layout {
            return;
        }
        self.sync_text_for_layout_change(next_layout, cx);
        self.layout = next_layout;
        self.compact_focused = false;
        self.dismiss_compact_suggestions();
        self.resize_drag = None;
        self.sync_editor_presentation(cx);
        cx.notify();
    }

    pub(super) fn toggle_expanded(&mut self, cx: &mut Context<Self>) {
        self.set_expanded(!self.is_expanded(), cx);
    }

    pub(super) fn document_snapshots(&self) -> Vec<TerminalCommandComposerDocumentSnapshot> {
        self.documents
            .iter()
            .map(TerminalCommandComposerDocument::snapshot)
            .collect()
    }

    pub(super) fn active_document_snapshot(
        &self,
    ) -> Option<TerminalCommandComposerDocumentSnapshot> {
        self.document(self.active_document_id)
            .map(TerminalCommandComposerDocument::snapshot)
    }

    pub(super) fn active_document_id(&self) -> TerminalCommandComposerId {
        self.active_document_id
    }

    pub(super) fn compact_focused(&self) -> bool {
        self.compact_focused
    }

    pub(super) fn set_compact_focused(&mut self, focused: bool, cx: &mut Context<Self>) {
        let focused = focused && self.layout == TerminalCommandComposerLayout::Compact;
        if self.compact_focused != focused {
            self.compact_focused = focused;
            if !focused {
                self.dismiss_compact_suggestions();
            }
            cx.notify();
        }
    }

    pub(super) fn compact_suggestions_open(&self) -> bool {
        self.compact_suggestions_open
    }

    pub(super) fn compact_suggestion_highlighted(&self) -> Option<usize> {
        self.compact_suggestion_highlighted
    }

    pub(super) fn move_compact_suggestion_selection(
        &mut self,
        suggestions_len: usize,
        move_down: bool,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        if suggestions_len == 0 {
            if self.dismiss_compact_suggestions() {
                cx.notify();
            }
            return None;
        }
        let last = suggestions_len.saturating_sub(1);
        let next = if move_down {
            self.compact_suggestion_highlighted
                .map(|index| index.saturating_add(1).min(last))
                .unwrap_or(0)
        } else {
            self.compact_suggestion_highlighted
                .map(|index| index.saturating_sub(1))
                .unwrap_or(last)
        };
        self.compact_suggestions_open = true;
        self.compact_suggestion_highlighted = Some(next);
        cx.notify();
        Some(next)
    }

    pub(super) fn dismiss_compact_suggestions(&mut self) -> bool {
        let had_highlight = self.compact_suggestion_highlighted.take().is_some();
        let changed = self.compact_suggestions_open || had_highlight;
        self.compact_suggestions_open = false;
        changed
    }

    pub(super) fn show_compact_history(&mut self, cx: &mut Context<Self>) {
        self.compact_suggestions_open = true;
        self.compact_suggestion_highlighted = None;
        cx.notify();
    }

    pub(super) fn active_compact_draft(&self) -> Option<&str> {
        self.document(self.active_document_id)
            .map(|document| document.compact_draft.as_str())
    }

    pub(super) fn active_compact_viewport(&self) -> Option<TextInputViewport> {
        self.document(self.active_document_id)
            .map(|document| document.compact_viewport.clone())
    }

    pub(super) fn replace_active_compact_text(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) -> TerminalCommandComposerId {
        let document_id = self.editable_active_document_id(cx);
        if let Some(document) = self.document_mut(document_id) {
            document.compact_draft = Zeroizing::new(text);
            document.failure = None;
        }
        self.dismiss_compact_suggestions();
        cx.notify();
        document_id
    }

    pub(super) fn clear_compact_text(
        &mut self,
        document_id: TerminalCommandComposerId,
        cx: &mut Context<Self>,
    ) {
        if let Some(document) = self.document_mut(document_id) {
            document.compact_draft = Zeroizing::new(String::new());
            document.compact_viewport = TextInputViewport::default();
            self.dismiss_compact_suggestions();
            cx.notify();
        }
    }

    pub(super) fn replace_active_text(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) -> TerminalCommandComposerId {
        if !self.is_expanded() {
            return self.replace_active_compact_text(text, cx);
        }
        let (document_id, editor) = self.editable_active_editor(cx);
        editor.update(cx, |editor, cx| {
            editor.replace_text_external(text, cx);
            editor.move_cursor_to_document_end(cx);
        });
        document_id
    }

    pub(super) fn running_count(&self) -> usize {
        self.documents
            .iter()
            .filter(|document| document.status == TerminalCommandComposerStatus::Running)
            .count()
    }

    pub(super) fn set_active_document(
        &mut self,
        document_id: TerminalCommandComposerId,
        cx: &mut Context<Self>,
    ) {
        if self.active_document_id == document_id || self.document(document_id).is_none() {
            return;
        }
        self.active_document_id = document_id;
        self.dismiss_compact_suggestions();
        cx.notify();
    }

    pub(super) fn add_document(&mut self, cx: &mut Context<Self>) -> TerminalCommandComposerId {
        let document_id = TerminalCommandComposerId(self.next_document_id);
        self.next_document_id = self.next_document_id.wrapping_add(1).max(1);
        self.documents.push(Self::new_document(
            document_id,
            self.editor_tokens,
            if self.is_expanded() {
                &self.expanded_editor_placeholder
            } else {
                &self.compact_editor_placeholder
            },
            &self.editor_context_menu_labels,
            self.layout.editor_presentation(),
            cx,
        ));
        self.active_document_id = document_id;
        cx.notify();
        document_id
    }

    pub(super) fn remove_document(
        &mut self,
        document_id: TerminalCommandComposerId,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.documents.len() == 1 {
            return false;
        }
        self.cancel_task(document_id);
        let Some(index) = self
            .documents
            .iter()
            .position(|document| document.id == document_id)
        else {
            return false;
        };
        self.documents.remove(index);
        if self.active_document_id == document_id {
            let next_index = index.min(self.documents.len().saturating_sub(1));
            self.active_document_id = self.documents[next_index].id;
        }
        cx.notify();
        true
    }

    pub(super) fn set_input_mode(
        &mut self,
        document_id: TerminalCommandComposerId,
        mode: TerminalSendInputMode,
        cx: &mut Context<Self>,
    ) {
        if self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            return;
        }
        if let Some(document) = self.document_mut(document_id)
            && document.input_mode != mode
        {
            document.input_mode = mode;
            document.failure = None;
            self.dismiss_compact_suggestions();
            cx.notify();
        }
    }

    pub(super) fn set_pacing(
        &mut self,
        document_id: TerminalCommandComposerId,
        pacing: TerminalSendPacing,
        cx: &mut Context<Self>,
    ) {
        if self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            return;
        }
        if let Some(document) = self.document_mut(document_id)
            && document.pacing != pacing
        {
            document.pacing = pacing;
            document.interval_ms = document.interval_ms.max(match pacing {
                TerminalSendPacing::Line => 100,
                TerminalSendPacing::Character => TERMINAL_COMPOSER_MIN_INTERVAL_MS,
            });
            document.failure = None;
            cx.notify();
        }
    }

    pub(super) fn adjust_interval(
        &mut self,
        document_id: TerminalCommandComposerId,
        delta_ms: i64,
        cx: &mut Context<Self>,
    ) {
        if self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            return;
        }
        if let Some(document) = self.document_mut(document_id) {
            let minimum = match document.pacing {
                TerminalSendPacing::Line => 100,
                TerminalSendPacing::Character => TERMINAL_COMPOSER_MIN_INTERVAL_MS,
            };
            let next = if delta_ms.is_negative() {
                document.interval_ms.saturating_sub(delta_ms.unsigned_abs())
            } else {
                document.interval_ms.saturating_add(delta_ms as u64)
            }
            .clamp(minimum, TERMINAL_COMPOSER_MAX_INTERVAL_MS);
            if next != document.interval_ms {
                document.interval_ms = next;
                cx.notify();
            }
        }
    }

    pub(super) fn adjust_repeat_count(
        &mut self,
        document_id: TerminalCommandComposerId,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        if self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            return;
        }
        if let Some(document) = self.document_mut(document_id) {
            let current = document.repeat_count;
            let next = if delta.is_negative() {
                let abs_delta = delta.unsigned_abs();
                if current <= abs_delta {
                    0
                } else {
                    current.saturating_sub(abs_delta)
                }
            } else {
                let add_delta = delta as u32;
                if current == 0 {
                    add_delta.clamp(1, TERMINAL_COMPOSER_MAX_REPEAT_COUNT)
                } else {
                    current
                        .saturating_add(add_delta)
                        .min(TERMINAL_COMPOSER_MAX_REPEAT_COUNT)
                }
            };
            if next != document.repeat_count {
                document.repeat_count = next;
                cx.notify();
            }
        }
    }

    pub(super) fn cycle_repeat_preset(
        &mut self,
        document_id: TerminalCommandComposerId,
        cx: &mut Context<Self>,
    ) {
        if self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            return;
        }
        if let Some(document) = self.document_mut(document_id) {
            let next = match document.repeat_count {
                0 => 1,
                1..=9 => 10,
                10..=99 => 100,
                100..=999 => 1000,
                _ => 0,
            };
            if next != document.repeat_count {
                document.repeat_count = next;
                cx.notify();
            }
        }
    }

    pub(super) fn set_target_scope(
        &mut self,
        document_id: TerminalCommandComposerId,
        scope: TerminalCommandComposerTargetScope,
        cx: &mut Context<Self>,
    ) {
        if self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            return;
        }
        if let Some(document) = self.document_mut(document_id)
            && document.target_scope != scope
        {
            document.target_scope = scope;
            document.failure = None;
            self.dismiss_compact_suggestions();
            cx.notify();
        }
    }

    pub(super) fn toggle_selected_target(
        &mut self,
        document_id: TerminalCommandComposerId,
        pane_id: PaneId,
        cx: &mut Context<Self>,
    ) {
        if self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            return;
        }
        if let Some(document) = self.document_mut(document_id) {
            if !document.selected_targets.remove(&pane_id) {
                document.selected_targets.insert(pane_id);
            }
            document.target_scope = TerminalCommandComposerTargetScope::Selected;
            document.failure = None;
            cx.notify();
        }
    }

    pub(super) fn set_selected_targets(
        &mut self,
        document_id: TerminalCommandComposerId,
        targets: &[PaneId],
        cx: &mut Context<Self>,
    ) {
        if self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            return;
        }
        if let Some(document) = self.document_mut(document_id) {
            document.selected_targets.clear();
            document.selected_targets.extend(targets.iter().copied());
            document.target_scope = TerminalCommandComposerTargetScope::Selected;
            document.failure = None;
            cx.notify();
        }
    }

    pub(super) fn retain_live_targets(
        &mut self,
        live_panes: &HashSet<PaneId>,
        cx: &mut Context<Self>,
    ) {
        let mut changed = false;
        for document in &mut self.documents {
            let previous = document.selected_targets.len();
            document
                .selected_targets
                .retain(|pane_id| live_panes.contains(pane_id));
            changed |= document.selected_targets.len() != previous;
        }
        if changed {
            cx.notify();
        }
    }

    pub(super) fn start(
        &mut self,
        document_id: TerminalCommandComposerId,
        current_pane_id: Option<PaneId>,
        candidates: Vec<TerminalCommandComposerTarget>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(document) = self.document(document_id) else {
            return false;
        };
        // Starting is not an implicit restart gesture. Refusing a second Start
        // keeps the active task and its target ownership unchanged.
        if document.status == TerminalCommandComposerStatus::Running {
            return false;
        }
        let input = if self.is_expanded() {
            Zeroizing::new(document.editor.read(cx).buffer().text())
        } else {
            Zeroizing::new(document.compact_draft.to_string())
        };
        let mode = document.input_mode;
        let pacing = document.pacing;
        let repeat_count = document.repeat_count;
        let interval = Duration::from_millis(document.interval_ms);
        let scope = document.target_scope;
        let selected_targets = document.selected_targets.clone();
        let plan = match build_terminal_send_plan(input, mode, pacing, repeat_count) {
            Ok(plan) => plan,
            Err(error) => {
                self.set_failure(document_id, plan_failure(error), cx);
                return false;
            }
        };

        let targets = resolve_run_targets(scope, current_pane_id, &selected_targets, candidates);
        if targets.is_empty() {
            self.set_failure(document_id, TerminalCommandComposerFailure::NoTargets, cx);
            return false;
        }
        if targets.iter().any(|target| {
            self.target_owners
                .get(&target.pane_id)
                .is_some_and(|owner| *owner != document_id)
        }) {
            self.set_failure(document_id, TerminalCommandComposerFailure::TargetBusy, cx);
            return false;
        }

        self.cancel_task(document_id);
        let generation = {
            let document = self
                .document_mut(document_id)
                .expect("composer document disappeared before task start");
            document.generation = document.generation.wrapping_add(1);
            document.status = TerminalCommandComposerStatus::Running;
            document.failure = None;
            document.completed_units = 0;
            document.total_units = plan.total_units();
            document.accepted_writes = 0;
            document.skipped_writes = 0;
            document.last_progress_notify = None;
            document.generation
        };
        for target in &targets {
            self.target_owners.insert(target.pane_id, document_id);
        }
        if let Some(editor) = self
            .document(document_id)
            .map(|document| document.editor.clone())
        {
            editor.update(cx, |editor, _cx| editor.set_read_only(true));
        }

        let task = cx.spawn(async move |composer, cx| {
            let mut stopped_for_missing_targets = false;
            let mut repeat_index: u64 = 0;
            let is_infinite = plan.is_infinite();
            let limit = plan.repeat_count() as u64;
            'repeats: loop {
                if !is_infinite && repeat_index >= limit {
                    break 'repeats;
                }
                for (frame_index, frame) in plan.frames().iter().enumerate() {
                    let is_final_unit = !is_infinite
                        && repeat_index + 1 == limit
                        && frame_index + 1 == plan.frames().len();
                    let should_continue = composer
                        .update(cx, |composer, cx| {
                            composer.dispatch_frame(document_id, generation, frame, &targets, cx)
                        })
                        .unwrap_or(false);
                    if !should_continue {
                        stopped_for_missing_targets = true;
                        break 'repeats;
                    }
                    if !is_final_unit {
                        Timer::after(interval).await;
                    }
                }
                repeat_index = repeat_index.saturating_add(1);
            }
            let _ = composer.update(cx, |composer, cx| {
                composer.finish_run(
                    document_id,
                    generation,
                    if stopped_for_missing_targets {
                        TerminalCommandComposerOutcome::Stopped
                    } else {
                        TerminalCommandComposerOutcome::Completed
                    },
                    cx,
                );
            });
        });
        self.active_tasks.insert(document_id, task);
        cx.notify();
        true
    }

    pub(super) fn stop(
        &mut self,
        document_id: TerminalCommandComposerId,
        cx: &mut Context<Self>,
    ) -> bool {
        let running = self
            .document(document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running);
        if !running {
            return false;
        }
        self.cancel_task(document_id);
        if let Some(document) = self.document_mut(document_id) {
            document.generation = document.generation.wrapping_add(1);
            document.status = TerminalCommandComposerStatus::Stopped;
            document.editor.update(cx, |editor, _cx| {
                editor.set_read_only(false);
            });
        }
        cx.notify();
        true
    }

    pub(super) fn stop_all(&mut self, cx: &mut Context<Self>) {
        let running_ids = self
            .documents
            .iter()
            .filter(|document| document.status == TerminalCommandComposerStatus::Running)
            .map(|document| document.id)
            .collect::<Vec<_>>();
        for document_id in running_ids {
            let _ = self.stop(document_id, cx);
        }
    }

    fn dispatch_frame(
        &mut self,
        document_id: TerminalCommandComposerId,
        generation: u64,
        frame: &TerminalSendFrame,
        targets: &[TerminalCommandComposerRunTarget],
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(document) = self.document(document_id) else {
            return false;
        };
        if document.generation != generation
            || document.status != TerminalCommandComposerStatus::Running
        {
            return false;
        }

        let mut accepted = 0u64;
        let mut skipped = 0u64;
        let mut live_targets = 0usize;
        for target in targets {
            let result = target.pane.update(cx, |pane, cx| match frame {
                TerminalSendFrame::TextLine(line) => pane.send_composer_line(line, cx),
                TerminalSendFrame::TextChunk(text) => pane.send_composer_text_chunk(text, cx),
                TerminalSendFrame::RawBytes(bytes) => pane.send_composer_raw_bytes(bytes, cx),
            });
            match result {
                Ok(true) => {
                    live_targets += 1;
                    accepted = accepted.saturating_add(1);
                }
                Ok(false) => {
                    live_targets += 1;
                    skipped = skipped.saturating_add(1);
                }
                Err(_) => skipped = skipped.saturating_add(1),
            }
        }
        let panel_expanded = self.is_expanded();
        let Some(document) = self.document_mut(document_id) else {
            return false;
        };
        document.completed_units = document.completed_units.saturating_add(1);
        document.accepted_writes = document.accepted_writes.saturating_add(accepted);
        document.skipped_writes = document.skipped_writes.saturating_add(skipped);
        let now = Instant::now();
        let progress_complete =
            document.total_units > 0 && document.completed_units >= document.total_units;
        let repaint_due = document.last_progress_notify.is_none_or(|last| {
            now.duration_since(last) >= TERMINAL_COMPOSER_PROGRESS_NOTIFY_INTERVAL
        });
        if panel_expanded && (progress_complete || repaint_due) {
            document.last_progress_notify = Some(now);
            cx.notify();
        }
        // A live pane that rejected the frame is not a successful local write.
        accepted > 0 && live_targets > 0
    }

    fn finish_run(
        &mut self,
        document_id: TerminalCommandComposerId,
        generation: u64,
        outcome: TerminalCommandComposerOutcome,
        cx: &mut Context<Self>,
    ) {
        let Some(document) = self.document_mut(document_id) else {
            return;
        };
        if document.generation != generation
            || document.status != TerminalCommandComposerStatus::Running
        {
            return;
        }
        document.status = match outcome {
            TerminalCommandComposerOutcome::Completed => TerminalCommandComposerStatus::Completed,
            TerminalCommandComposerOutcome::Stopped => TerminalCommandComposerStatus::Stopped,
        };
        document.editor.update(cx, |editor, _cx| {
            editor.set_read_only(false);
        });
        self.release_target_ownership(document_id);
        cx.notify();
    }

    fn set_failure(
        &mut self,
        document_id: TerminalCommandComposerId,
        failure: TerminalCommandComposerFailure,
        cx: &mut Context<Self>,
    ) {
        if let Some(document) = self.document_mut(document_id) {
            document.status = TerminalCommandComposerStatus::Failed;
            document.failure = Some(failure);
            document.completed_units = 0;
            document.total_units = 0;
            document.accepted_writes = 0;
            document.skipped_writes = 0;
            cx.notify();
        }
    }

    fn cancel_task(&mut self, document_id: TerminalCommandComposerId) {
        // Dropping the Entity-owned GPUI task cancels its pending timer.
        self.active_tasks.remove(&document_id);
        self.release_target_ownership(document_id);
    }

    fn release_target_ownership(&mut self, document_id: TerminalCommandComposerId) {
        self.target_owners.retain(|_, owner| *owner != document_id);
    }

    fn editable_active_editor(
        &mut self,
        cx: &mut Context<Self>,
    ) -> (TerminalCommandComposerId, Entity<TextEditorView>) {
        let document_id = self.editable_active_document_id(cx);
        let editor = self
            .document(document_id)
            .expect("active composer document disappeared")
            .editor
            .clone();
        (document_id, editor)
    }

    fn editable_active_document_id(&mut self, cx: &mut Context<Self>) -> TerminalCommandComposerId {
        if self
            .document(self.active_document_id)
            .is_some_and(|document| document.status == TerminalCommandComposerStatus::Running)
        {
            self.add_document(cx)
        } else {
            self.active_document_id
        }
    }

    fn document(
        &self,
        document_id: TerminalCommandComposerId,
    ) -> Option<&TerminalCommandComposerDocument> {
        self.documents
            .iter()
            .find(|document| document.id == document_id)
    }

    fn document_mut(
        &mut self,
        document_id: TerminalCommandComposerId,
    ) -> Option<&mut TerminalCommandComposerDocument> {
        self.documents
            .iter_mut()
            .find(|document| document.id == document_id)
    }

    pub(super) fn panel_height_for_viewport(&self, viewport_height: f32) -> f32 {
        adjusted_composer_panel_height(self.panel_height, 0.0, viewport_height)
    }

    pub(super) fn is_resizing(&self) -> bool {
        self.resize_drag.is_some()
    }

    pub(super) fn start_resize(
        &mut self,
        cursor_y: gpui::Pixels,
        viewport_height: f32,
        cx: &mut Context<Self>,
    ) {
        let current_height = self.panel_height_for_viewport(viewport_height);
        self.panel_height = current_height;
        self.resize_drag = Some(TerminalCommandComposerResizeDrag {
            start_cursor_y: cursor_y,
            start_height: current_height,
        });
        cx.notify();
    }

    pub(super) fn update_resize(
        &mut self,
        cursor_y: gpui::Pixels,
        viewport_height: f32,
        dragging: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.resize_drag else {
            return;
        };
        if !dragging {
            self.finish_resize(cx);
            return;
        }
        let next_height = adjusted_composer_panel_height(
            drag.start_height,
            f32::from(cursor_y - drag.start_cursor_y),
            viewport_height,
        );
        if (next_height - self.panel_height).abs() >= f32::EPSILON {
            self.panel_height = next_height;
            cx.notify();
        }
    }

    pub(super) fn finish_resize(&mut self, cx: &mut Context<Self>) {
        if self.resize_drag.take().is_some() {
            cx.notify();
        }
    }

    pub(super) fn reset_height(&mut self, viewport_height: f32, cx: &mut Context<Self>) {
        let height =
            adjusted_composer_panel_height(TERMINAL_COMPOSER_DEFAULT_HEIGHT, 0.0, viewport_height);
        let changed = (height - self.panel_height).abs() >= f32::EPSILON;
        self.panel_height = height;
        self.resize_drag = None;
        if changed {
            cx.notify();
        }
    }

    pub(super) fn sync_editor_appearance(
        &mut self,
        tokens: ThemeTokens,
        font_family: String,
        font_size: f32,
        line_height: f32,
        background_active: bool,
        cx: &mut Context<Self>,
    ) {
        self.editor_tokens = tokens;
        for document in &self.documents {
            document.editor.update(cx, |editor, cx| {
                editor.apply_runtime_settings(
                    &tokens,
                    font_family.clone(),
                    font_size,
                    line_height,
                    false,
                    background_active,
                    cx,
                );
            });
        }
    }

    fn sync_editor_presentation(&mut self, cx: &mut Context<Self>) {
        let placeholder = if self.is_expanded() {
            self.expanded_editor_placeholder.clone()
        } else {
            self.compact_editor_placeholder.clone()
        };
        let presentation = self.layout.editor_presentation();
        for document in &self.documents {
            document.editor.update(cx, |editor, cx| {
                editor.set_presentation(presentation, cx);
                editor.set_placeholder(Some(placeholder.clone()), cx);
            });
        }
    }

    fn sync_text_for_layout_change(
        &mut self,
        next_layout: TerminalCommandComposerLayout,
        cx: &mut Context<Self>,
    ) {
        // The rendered surface is the sole live editor; layout boundaries transfer its draft.
        if self.layout == TerminalCommandComposerLayout::Expanded
            && next_layout != TerminalCommandComposerLayout::Expanded
        {
            for document in &mut self.documents {
                document.compact_draft = Zeroizing::new(document.editor.read(cx).buffer().text());
                document.compact_viewport = TextInputViewport::default();
            }
        } else if self.layout != TerminalCommandComposerLayout::Expanded
            && next_layout == TerminalCommandComposerLayout::Expanded
        {
            for document in &self.documents {
                let text = document.compact_draft.to_string();
                document.editor.update(cx, |editor, cx| {
                    editor.replace_text_external(text, cx);
                    editor.move_cursor_to_document_end(cx);
                });
            }
        }
    }
}

fn resolve_run_targets(
    scope: TerminalCommandComposerTargetScope,
    current_pane_id: Option<PaneId>,
    selected_targets: &HashSet<PaneId>,
    candidates: Vec<TerminalCommandComposerTarget>,
) -> Vec<TerminalCommandComposerRunTarget> {
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|target| match scope {
            TerminalCommandComposerTargetScope::Current => current_pane_id == Some(target.pane_id),
            TerminalCommandComposerTargetScope::All => true,
            TerminalCommandComposerTargetScope::Broadcast => {
                selected_targets.contains(&target.pane_id)
            }
            TerminalCommandComposerTargetScope::Selected => {
                selected_targets.contains(&target.pane_id)
            }
        })
        .filter(|target| seen.insert(target.pane_id))
        .map(|target| TerminalCommandComposerRunTarget {
            pane_id: target.pane_id,
            pane: target.pane,
        })
        .collect()
}

fn plan_failure(error: TerminalSendPlanError) -> TerminalCommandComposerFailure {
    match error {
        TerminalSendPlanError::EmptyInput | TerminalSendPlanError::EmptyHexInput => {
            TerminalCommandComposerFailure::EmptyInput
        }
        TerminalSendPlanError::InvalidHexDigit | TerminalSendPlanError::OddHexDigitCount => {
            TerminalCommandComposerFailure::InvalidHex
        }
        TerminalSendPlanError::ZeroRepeatCount | TerminalSendPlanError::UnitCountOverflow => {
            TerminalCommandComposerFailure::EmptyInput
        }
    }
}

fn adjusted_composer_panel_height(start_height: f32, delta_y: f32, viewport_height: f32) -> f32 {
    // Moving the divider upward grows the bottom composer panel.
    let ratio_limit = viewport_height * TERMINAL_COMPOSER_MAX_VIEWPORT_RATIO;
    let terminal_limit = viewport_height
        - TERMINAL_COMPOSER_WINDOW_CHROME_RESERVE
        - TERMINAL_COMPOSER_MIN_TERMINAL_HEIGHT;
    let maximum = ratio_limit
        .min(terminal_limit)
        .max(TERMINAL_COMPOSER_MIN_HEIGHT);
    (start_height - delta_y).clamp(TERMINAL_COMPOSER_MIN_HEIGHT, maximum)
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use oxideterm_theme::default_tokens;

    use super::*;

    fn test_context_menu_labels() -> EditorContextMenuLabels {
        EditorContextMenuLabels {
            copy: "Copy".to_string(),
            cut: "Cut".to_string(),
            paste: "Paste".to_string(),
            select_all: "Select all".to_string(),
        }
    }

    #[test]
    fn plan_errors_map_without_retaining_input_details() {
        assert_eq!(
            plan_failure(TerminalSendPlanError::InvalidHexDigit),
            TerminalCommandComposerFailure::InvalidHex
        );
        assert_eq!(
            plan_failure(TerminalSendPlanError::EmptyInput),
            TerminalCommandComposerFailure::EmptyInput
        );
    }

    #[gpui::test]
    fn compact_and_expanded_inputs_sync_only_when_layout_changes(cx: &mut TestAppContext) {
        let composer = cx.new(|cx| {
            TerminalCommandComposerEntity::new(
                default_tokens(),
                "Command".to_string(),
                "Commands".to_string(),
                test_context_menu_labels(),
                cx,
            )
        });
        composer.update(cx, |composer, cx| {
            composer.replace_active_compact_text("echo compact".to_string(), cx);
            composer.set_expanded(true, cx);
        });
        let editor = composer
            .read_with(cx, |composer, _cx| {
                composer
                    .active_document_snapshot()
                    .map(|snapshot| snapshot.editor)
            })
            .expect("active composer editor");
        assert_eq!(
            editor.read_with(cx, |editor, _cx| editor.buffer().text()),
            "echo compact"
        );

        editor.update(cx, |editor, cx| {
            editor.replace_text_external("echo expanded".to_string(), cx);
        });
        composer.update(cx, |composer, cx| composer.set_expanded(false, cx));
        composer.read_with(cx, |composer, _cx| {
            assert_eq!(composer.active_compact_draft(), Some("echo expanded"));
        });
    }

    #[gpui::test]
    fn compact_suggestion_selection_starts_at_the_directional_edge(cx: &mut TestAppContext) {
        let composer = cx.new(|cx| {
            TerminalCommandComposerEntity::new(
                default_tokens(),
                "Command".to_string(),
                "Commands".to_string(),
                test_context_menu_labels(),
                cx,
            )
        });

        composer.update(cx, |composer, cx| {
            assert_eq!(
                composer.move_compact_suggestion_selection(3, true, cx),
                Some(0)
            );
            assert_eq!(
                composer.move_compact_suggestion_selection(3, true, cx),
                Some(1)
            );
            assert_eq!(
                composer.move_compact_suggestion_selection(3, false, cx),
                Some(0)
            );
            assert!(composer.dismiss_compact_suggestions());
            assert_eq!(
                composer.move_compact_suggestion_selection(3, false, cx),
                Some(2)
            );
        });
    }

    #[gpui::test]
    fn adjust_repeat_count_supports_infinite_and_presets(cx: &mut TestAppContext) {
        let composer = cx.new(|cx| {
            TerminalCommandComposerEntity::new(
                default_tokens(),
                "Command".to_string(),
                "Commands".to_string(),
                test_context_menu_labels(),
                cx,
            )
        });

        composer.update(cx, |composer, cx| {
            let document_id = composer.active_document_id();

            // Default is 1
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 1);

            // 1 clicked minus -> 0 (infinite)
            composer.adjust_repeat_count(document_id, -1, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 0);

            // From 0, clicking minus stays 0
            composer.adjust_repeat_count(document_id, -1, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 0);

            // From 0, clicking plus (delta 1) returns to 1
            composer.adjust_repeat_count(document_id, 1, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 1);

            // Step by 1000
            composer.adjust_repeat_count(document_id, 1000, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 1001);

            // Cycle presets
            composer.document_mut(document_id).unwrap().repeat_count = 1;
            composer.cycle_repeat_preset(document_id, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 10);
            composer.cycle_repeat_preset(document_id, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 100);
            composer.cycle_repeat_preset(document_id, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 1000);
            composer.cycle_repeat_preset(document_id, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 0);
            composer.cycle_repeat_preset(document_id, cx);
            assert_eq!(composer.document(document_id).unwrap().repeat_count, 1);
        });
    }
}
