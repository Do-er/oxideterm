use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

use gpui::{
    AnchoredPositionMode, AnyElement, App, Context, Corner, Focusable, KeyDownEvent, MouseButton,
    PathPromptOptions, SharedString, anchored, deferred, div, point, prelude::*, px, rgb, rgba,
};
use oxideterm_editor_core::utf16::replace_utf16;
use oxideterm_gpui_editor::{EditorContextMenuLabels, EditorSettings, TextEditorView};
use oxideterm_gpui_ui::{
    SectionHeaderOptions, StatusPillOptions, StatusTone,
    button::{ButtonOptions, ButtonRadius, ButtonSize, ButtonVariant, button_with},
    modal::{dismissible_dialog_backdrop, overlay_content_boundary, popover_backdrop},
    scroll::ScrollableElement,
    section_header,
    select::{SelectAnchorId, select_anchor_probe},
    status_pill,
    text_input::{TextInputView, text_input_with_viewport},
};
use oxideterm_i18n::I18n;
use oxideterm_quick_commands::{
    FALLBACK_QUICK_COMMAND_CATEGORY_ID, QuickCommandMatchField, QuickCommandRisk,
    QuickCommandTargetContext, classify_command_risk,
    quick_command_category_draft_can_save, quick_command_has_runtime_substitutions,
    validate_quick_command_template,
};
use zeroize::Zeroizing;

use super::super::ime::WorkspaceImeTarget;
use super::super::{
    QUICK_COMMAND_LIST_ESTIMATED_HEIGHT, QUICK_COMMAND_LIST_OVERSCAN, SelectableTextRole,
    TauriVirtualListSpec, TerminalNotice, TerminalNoticeVariant, WorkspaceApp,
    settings_mono_font_family, settings_ui_font_family,
    sync_tauri_variable_list_state_by_signatures,
};
use super::{
    QUICK_COMMAND_TEXTAREA_LINE_HEIGHT, QUICK_COMMAND_TEXTAREA_MIN_HEIGHT,
    QUICK_COMMAND_TEXTAREA_VERTICAL_PADDING, QuickCommand, QuickCommandCategory,
    QuickCommandCategoryDeletePrompt, QuickCommandCategoryDraft, QuickCommandConfirmationPolicy,
    QuickCommandEditorDraft, QuickCommandExecutionDraft, QuickCommandIcon,
    QuickCommandImportStrategy, QuickCommandInput, QuickCommandManagerMode, QuickCommandParameter,
    QuickCommandParameterEditorDraft, QuickCommandParameterKind, QuickCommandTargetProtocol,
    TerminalQuickCommandsState, default_quick_command_categories, now_ms,
};
use crate::assets::LucideIcon;

pub(super) fn quick_command_lucide_icon(icon: QuickCommandIcon) -> LucideIcon {
    match icon {
        QuickCommandIcon::Docker => LucideIcon::Docker,
        QuickCommandIcon::Zap => LucideIcon::Zap,
        QuickCommandIcon::Cpu => LucideIcon::Cpu,
        QuickCommandIcon::Cloud => LucideIcon::Cloud,
        QuickCommandIcon::Git => LucideIcon::GitFork,
        QuickCommandIcon::Shield => LucideIcon::Shield,
        QuickCommandIcon::Package => LucideIcon::Package,
        QuickCommandIcon::Rocket => LucideIcon::Rocket,
        QuickCommandIcon::Settings => LucideIcon::Settings,
        QuickCommandIcon::Network => LucideIcon::Network,
        QuickCommandIcon::SunDim => LucideIcon::SunDim,
        QuickCommandIcon::Cat => LucideIcon::Cat,
        QuickCommandIcon::FaceSlightlySmilingPlus => LucideIcon::FaceSlightlySmilingPlus,
        QuickCommandIcon::IceCreamCone => LucideIcon::IceCreamCone,
        QuickCommandIcon::CloudHail => LucideIcon::CloudHail,
        QuickCommandIcon::Radar => LucideIcon::Radar,
        QuickCommandIcon::Leaf => LucideIcon::Leaf,
        QuickCommandIcon::Apple => LucideIcon::Apple,
        QuickCommandIcon::Webhook => LucideIcon::Webhook,
        QuickCommandIcon::CirclePile => LucideIcon::CirclePile,
        QuickCommandIcon::Palette => LucideIcon::Palette,
        QuickCommandIcon::Sprout => LucideIcon::Sprout,
        QuickCommandIcon::PiggyBank => LucideIcon::PiggyBank,
        QuickCommandIcon::CloudSun => LucideIcon::CloudSun,
        QuickCommandIcon::MoonStar => LucideIcon::MoonStar,
        QuickCommandIcon::Snowflake => LucideIcon::Snowflake,
        QuickCommandIcon::PawPrint => LucideIcon::PawPrint,
        QuickCommandIcon::LoaderPinwheel => LucideIcon::LoaderPinwheel,
    }
}

const QUICK_COMMANDS_MANAGER_WIDTH: f32 = 960.0;
const QUICK_COMMANDS_MANAGER_HEIGHT: f32 = 620.0;
const QUICK_COMMANDS_MANAGER_SIDEBAR_WIDTH: f32 = 300.0;
const QUICK_COMMANDS_MANAGER_CATEGORY_DROPDOWN_WIDTH: f32 = 200.0;

#[derive(Clone)]
struct QuickCommandCategoryDrag {
    id: String,
    name: String,
    position: gpui::Point<gpui::Pixels>,
    tokens: oxideterm_theme::ThemeTokens,
}

impl gpui::Render for QuickCommandCategoryDrag {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        let tokens = self.tokens;
        div().pl(self.position.x).pt(self.position.y).child(
            div()
                .px_3()
                .py_2()
                .max_w(px(220.0))
                .truncate()
                .rounded(px(tokens.radii.md))
                .border_1()
                .border_color(rgb(tokens.ui.accent))
                .bg(rgb(tokens.ui.bg_panel))
                .text_color(rgb(tokens.ui.text))
                .font_family(tokens.metrics.font_family)
                .text_size(px(tokens.metrics.ui_text_sm))
                .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&tokens))
                .child(self.name.clone()),
        )
    }
}

#[derive(Clone)]
struct QuickCommandDrag {
    id: String,
    name: String,
    position: gpui::Point<gpui::Pixels>,
    tokens: oxideterm_theme::ThemeTokens,
}

impl gpui::Render for QuickCommandDrag {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        let tokens = self.tokens;
        div().pl(self.position.x).pt(self.position.y).child(
            div()
                .px_3()
                .py_2()
                .max_w(px(220.0))
                .truncate()
                .rounded(px(tokens.radii.md))
                .border_1()
                .border_color(rgb(tokens.ui.accent))
                .bg(rgb(tokens.ui.bg_panel))
                .text_color(rgb(tokens.ui.text))
                .font_family(tokens.metrics.font_family)
                .text_size(px(tokens.metrics.ui_text_sm))
                .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&tokens))
                .child(self.name.clone()),
        )
    }
}

fn quick_command_editor_frame(
    form: AnyElement,
    footer: AnyElement,
    scroll: &gpui::ScrollHandle,
    tokens: &oxideterm_theme::ThemeTokens,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id("quick-command-editor")
        .h_full()
        .min_h_0()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(rgba((tokens.ui.bg << 8) | 0x59))
        .child(
            div()
                .relative()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .child(
                    div()
                        .id("quick-command-editor-scroll")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(scroll)
                        .child(form),
                )
                .vertical_scrollbar(scroll),
        )
        .child(
            div()
                .id("quick-command-editor-footer")
                .flex_none()
                .border_t_1()
                .border_color(rgb(tokens.ui.border))
                .child(footer),
        )
}

fn quick_commands_export_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn select_quick_command_category_state(
    active_category: &mut String,
    command_editor: &mut Option<QuickCommandEditorDraft>,
    category_editor: &mut Option<QuickCommandCategoryDraft>,
    focused_input: &mut Option<QuickCommandInput>,
    highlighted_command: &mut Option<String>,
    category_id: &str,
) {
    *active_category = category_id.to_string();
    *command_editor = None;
    *category_editor = None;
    *focused_input = None;
    *highlighted_command = None;
}

fn quick_command_editor_can_save(draft: &QuickCommandEditorDraft) -> bool {
    if draft.name.trim().is_empty() || draft.command.trim().is_empty() {
        return false;
    }
    if !draft.line_delay_ms.trim().is_empty() && draft.line_delay_ms.trim().parse::<u64>().is_err()
    {
        return false;
    }
    let mut names = std::collections::HashSet::new();
    let mut parameters = Vec::with_capacity(draft.parameters.len());
    for parameter in &draft.parameters {
        let name = parameter.name.trim();
        let mut characters = name.chars();
        let valid_name = characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            && characters.all(|character| character.is_ascii_alphanumeric() || character == '_');
        let choices = parameter
            .choices
            .split([',', '\n'])
            .map(str::trim)
            .filter(|choice| !choice.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        let default_value = parameter.default_value.trim();
        if !valid_name
            || !names.insert(name.to_string())
            || parameter.label.trim().is_empty()
            || (parameter.kind == QuickCommandParameterKind::Choice && choices.is_empty())
            || (parameter.kind == QuickCommandParameterKind::Choice
                && !default_value.is_empty()
                && !choices.iter().any(|choice| choice == default_value))
            || (parameter.kind == QuickCommandParameterKind::Secret
                && (!default_value.is_empty() || !choices.is_empty()))
        {
            return false;
        }
        parameters.push(QuickCommandParameter {
            name: name.to_string(),
            label: parameter.label.trim().to_string(),
            kind: parameter.kind,
            default_value: (parameter.kind != QuickCommandParameterKind::Secret
                && !default_value.is_empty())
            .then(|| default_value.to_string()),
            choices: if parameter.kind == QuickCommandParameterKind::Secret {
                Vec::new()
            } else {
                choices
            },
            required: parameter.required,
        });
    }
    // A command cannot enter persisted state with misspelled or malformed template tokens.
    validate_quick_command_template(&draft.command, &parameters).is_ok()
}

fn quick_command_space_inserts_literal(platform: bool, control: bool, alt: bool) -> bool {
    !platform && !control && !alt
}

pub(in crate::workspace) fn quick_command_input_uses_monospace(input: QuickCommandInput) -> bool {
    matches!(
        input,
        QuickCommandInput::CommandText
            | QuickCommandInput::CategoryColor
            | QuickCommandInput::CommandHostPattern
            | QuickCommandInput::ParameterName(_)
            | QuickCommandInput::ParameterDefault(_)
            | QuickCommandInput::ParameterChoices(_)
    )
}

fn quick_command_risk_tone(risk: QuickCommandRisk) -> StatusTone {
    // Risk colors are presentation policy and remain owned by the GPUI layer.
    match risk {
        QuickCommandRisk::High => StatusTone::Error,
        QuickCommandRisk::Medium => StatusTone::Warning,
    }
}

fn quick_command_risk_label(i18n: &I18n, risk: QuickCommandRisk) -> String {
    match risk {
        QuickCommandRisk::High => i18n.t("terminal.quick_commands.risk_high"),
        QuickCommandRisk::Medium => i18n.t("terminal.quick_commands.risk_medium"),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuickCommandRiskBadge {
    Classified(QuickCommandRisk),
    Dynamic,
}

fn quick_command_risk_badge(template: &str) -> Option<QuickCommandRiskBadge> {
    classify_command_risk(template)
        .map(QuickCommandRiskBadge::Classified)
        .or_else(|| {
            quick_command_has_runtime_substitutions(template)
                .then_some(QuickCommandRiskBadge::Dynamic)
        })
}

fn quick_command_risk_badge_label(i18n: &I18n, badge: QuickCommandRiskBadge) -> String {
    match badge {
        QuickCommandRiskBadge::Classified(risk) => quick_command_risk_label(i18n, risk),
        QuickCommandRiskBadge::Dynamic => i18n.t("terminal.quick_commands.risk_dynamic"),
    }
}

fn quick_command_risk_badge_tone(badge: QuickCommandRiskBadge) -> StatusTone {
    match badge {
        QuickCommandRiskBadge::Classified(risk) => quick_command_risk_tone(risk),
        QuickCommandRiskBadge::Dynamic => StatusTone::Warning,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuickCommandKeyDirection {
    Next,
    Previous,
}

fn quick_command_highlighted_index(
    visible_commands: &[QuickCommand],
    highlighted_command: Option<&str>,
) -> Option<usize> {
    highlighted_command.and_then(|id| {
        visible_commands
            .iter()
            .position(|command| command.id.as_str() == id)
    })
}

fn quick_command_keyboard_highlight(
    visible_commands: &[QuickCommand],
    highlighted_command: Option<&str>,
    direction: QuickCommandKeyDirection,
) -> Option<String> {
    if visible_commands.is_empty() {
        return None;
    }
    let current = quick_command_highlighted_index(visible_commands, highlighted_command);
    let next = match direction {
        QuickCommandKeyDirection::Next => current
            .map(|index| (index + 1).min(visible_commands.len() - 1))
            .unwrap_or(0),
        QuickCommandKeyDirection::Previous => current
            .map(|index| index.saturating_sub(1))
            .unwrap_or(visible_commands.len() - 1),
    };
    Some(visible_commands[next].id.clone())
}

fn quick_command_highlight_at(visible_commands: &[QuickCommand], index: usize) -> Option<String> {
    visible_commands
        .get(index.min(visible_commands.len().saturating_sub(1)))
        .map(|command| command.id.clone())
}

fn quick_command_row_signature(command: &QuickCommand) -> u64 {
    let mut hasher = DefaultHasher::new();
    // The command id is the row key; text fields and edit timestamps affect
    // visible row content, so include them when syncing GPUI ListState.
    command.id.hash(&mut hasher);
    command.name.hash(&mut hasher);
    command.command.hash(&mut hasher);
    command.category.hash(&mut hasher);
    command.description.hash(&mut hasher);
    command.parameters.hash(&mut hasher);
    command.availability.hash(&mut hasher);
    command.confirmation.hash(&mut hasher);
    command.sort_order.hash(&mut hasher);
    command.updated_at.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone)]
pub(super) struct QuickCommandsRenderSnapshot {
    pub(super) categories: Vec<QuickCommandCategory>,
    pub(super) category_counts: HashMap<String, usize>,
    pub(super) active_category: String,
    pub(super) query: String,
    pub(super) focused_input: Option<QuickCommandInput>,
    pub(super) highlighted_command: Option<String>,
    pub(super) command_editor: Option<QuickCommandEditorDraft>,
    pub(super) category_editor: Option<QuickCommandCategoryDraft>,
    pub(super) pending_category_delete: Option<QuickCommandCategoryDeletePrompt>,
    pub(super) visible_commands: Arc<Vec<QuickCommand>>,
    pub(super) managing: bool,
    pub(super) manager_category_dropdown_open: bool,
    pub(super) editor_category_menu_open: bool,
    pub(super) manager_mode: QuickCommandManagerMode,
    pub(super) manager_command_category_filter: Option<String>,
    pub(super) manager_search_open: bool,
}

impl TerminalQuickCommandsState {
    fn visible_commands_for_targets(
        &self,
        target: Option<&QuickCommandTargetContext>,
    ) -> Vec<QuickCommand> {
        self.store.visible_commands_for_targets(target)
    }

    pub(in crate::workspace) fn is_open(&self) -> bool {
        self.open
    }

    pub(in crate::workspace) fn category_menu_open(&self) -> bool {
        self.category_menu_open
    }

    pub(in crate::workspace) fn dismiss_category_menu(&mut self) -> bool {
        std::mem::take(&mut self.category_menu_open)
    }

    pub(in crate::workspace) fn manager_open(&self) -> bool {
        self.manager_open
    }

    pub(in crate::workspace) fn execution_pending(&self) -> bool {
        self.pending_execution.is_some()
    }

    pub(in crate::workspace) fn has_open_or_pending(&self) -> bool {
        self.open || self.pending_execution.is_some()
    }

    pub(in crate::workspace) fn focused_input(&self) -> Option<QuickCommandInput> {
        self.store.focused_input
    }

    pub(in crate::workspace) fn close(&mut self) -> bool {
        let changed = self.has_open_or_pending()
            || self.pinned
            || self.store.focused_input.is_some()
            || self.store.highlighted_command.is_some();
        self.open = false;
        self.panel.finish_resize();
        self.pinned = false;
        self.pending_execution = None;
        self.button_context_menu = None;
        self.store.focused_input = None;
        self.store.highlighted_command = None;
        changed
    }

    pub(in crate::workspace) fn toggle_open(&mut self) -> bool {
        if self.open {
            self.close();
            false
        } else {
            self.open = true;
            self.pinned = true;
            true
        }
    }

    pub(in crate::workspace) fn finish_execution(&mut self) {
        // Confirmation text may contain shell parameters, so dropping the
        // zeroizing owner is part of every completion path.
        self.pending_execution = None;
        self.open = true;
        self.pinned = true;
        self.store.focused_input = None;
    }

    pub(in crate::workspace) fn request_execution(&mut self, command: QuickCommand) {
        self.editor_scroll = gpui::ScrollHandle::new();
        let parameter_values = command
            .parameters
            .iter()
            .map(|parameter| Zeroizing::new(parameter.default_value.clone().unwrap_or_default()))
            .collect();
        self.store.focused_input = (!command.parameters.is_empty())
            .then_some(QuickCommandInput::ParameterDefault(0));
        self.pending_execution = Some(QuickCommandExecutionDraft {
            command,
            parameter_values,
        });
        self.open = true;
    }

    fn prepare_insertion(&mut self, command: String, _keep_open: bool) -> String {
        self.open = true;
        self.pinned = true;
        self.store.focused_input = None;
        self.store.highlighted_command = None;
        command
    }

    fn open_manager(&mut self) {
        // Retain the drawer dock and its pinned status when opening the manager modal.
        self.panel.finish_resize();
        self.pending_execution = None;
        self.pending_category_delete = None;
        self.manager_open = true;
        self.manager_category_dropdown_open = false;
        self.editor_category_menu_open = false;
        self.manager_mode = QuickCommandManagerMode::Command;
        self.manager_command_category_filter = Some(self.store.active_category.clone());
        self.manager_search_open = false;
        self.store.command_editor = None;
        self.command_input = None;
        self.command_input_subscription = None;
        self.store.category_editor = None;
        self.store.focused_input = Some(QuickCommandInput::Search);
        self.store.highlighted_command = None;
    }

    fn close_manager(&mut self) -> bool {
        let changed = self.manager_open
            || self.store.command_editor.is_some()
            || self.store.category_editor.is_some()
            || self.store.focused_input.is_some()
            || self.pending_category_delete.is_some()
            || self.manager_category_dropdown_open
            || self.editor_category_menu_open;
        self.manager_open = false;
        self.manager_category_dropdown_open = false;
        self.editor_category_menu_open = false;
        self.button_context_menu = None;
        self.pending_category_delete = None;
        self.store.command_editor = None;
        self.command_input = None;
        self.command_input_subscription = None;
        self.store.category_editor = None;
        self.store.focused_input = None;
        self.store.highlighted_command = None;
        changed
    }

    fn select_category(&mut self, category_id: &str) {
        self.command_input = None;
        self.command_input_subscription = None;
        select_quick_command_category_state(
            &mut self.store.active_category,
            &mut self.store.command_editor,
            &mut self.store.category_editor,
            &mut self.store.focused_input,
            &mut self.store.highlighted_command,
            category_id,
        );
    }

    pub(in crate::workspace) fn category_delete_pending(&self) -> bool {
        self.pending_category_delete.is_some()
    }

    fn cancel_category_delete(&mut self) -> bool {
        self.pending_category_delete.take().is_some()
    }

    fn confirm_category_delete(&mut self) -> bool {
        let Some(prompt) = self.pending_category_delete.take() else {
            return false;
        };
        self.store.highlighted_command = None;
        self.store.delete_category(&prompt.id)
    }

    pub(in crate::workspace) fn delete_command(&mut self, command_id: &str) {
        self.store.delete_command(command_id);
        if self
            .store
            .command_editor
            .as_ref()
            .and_then(|draft| draft.id.as_deref())
            == Some(command_id)
        {
            self.store.command_editor = None;
        }
        self.button_context_menu = None;
        self.store.highlighted_command = None;
    }

    fn set_highlighted_command(&mut self, command_id: String) -> bool {
        if self.store.highlighted_command.as_deref() == Some(command_id.as_str()) {
            return false;
        }
        self.store.highlighted_command = Some(command_id);
        true
    }

    fn set_focused_input(&mut self, input: QuickCommandInput) {
        self.store.focused_input = Some(input);
    }

    pub(in crate::workspace) fn input_value(&self, input: QuickCommandInput) -> Option<&str> {
        match input {
            QuickCommandInput::Search => Some(self.store.query.as_str()),
            QuickCommandInput::CommandName => self
                .store
                .command_editor
                .as_ref()
                .map(|draft| draft.name.as_str()),
            QuickCommandInput::CommandText => self
                .store
                .command_editor
                .as_ref()
                .map(|draft| draft.command.as_str()),
            QuickCommandInput::CommandLineDelay => self
                .store
                .command_editor
                .as_ref()
                .map(|draft| draft.line_delay_ms.as_str()),
            QuickCommandInput::CommandDescription => self
                .store
                .command_editor
                .as_ref()
                .map(|draft| draft.description.as_str()),
            QuickCommandInput::CommandHostPattern => self
                .store
                .command_editor
                .as_ref()
                .map(|draft| draft.host_patterns.as_str()),
            QuickCommandInput::ParameterName(index) => self
                .store
                .command_editor
                .as_ref()?
                .parameters
                .get(index)
                .map(|parameter| parameter.name.as_str()),
            QuickCommandInput::ParameterLabel(index) => self
                .store
                .command_editor
                .as_ref()?
                .parameters
                .get(index)
                .map(|parameter| parameter.label.as_str()),
            QuickCommandInput::ParameterDefault(index) => {
                if let Some(execution) = self.pending_execution.as_ref() {
                    execution
                        .parameter_values
                        .get(index)
                        .map(|value| value.as_str())
                } else {
                    self.store
                        .command_editor
                        .as_ref()?
                        .parameters
                        .get(index)
                        .map(|parameter| parameter.default_value.as_str())
                }
            }
            QuickCommandInput::ParameterChoices(index) => self
                .store
                .command_editor
                .as_ref()?
                .parameters
                .get(index)
                .map(|parameter| parameter.choices.as_str()),
            QuickCommandInput::CategoryName => self
                .store
                .category_editor
                .as_ref()
                .map(|draft| draft.name.as_str()),
            QuickCommandInput::CategoryColor => self
                .store
                .category_editor
                .as_ref()
                .map(|draft| draft.icon_color.as_str()),
        }
    }

    fn input_value_mut(&mut self, input: QuickCommandInput) -> Option<&mut String> {
        match input {
            QuickCommandInput::Search => Some(&mut self.store.query),
            QuickCommandInput::CommandName => self
                .store
                .command_editor
                .as_mut()
                .map(|draft| &mut draft.name),
            QuickCommandInput::CommandText => self
                .store
                .command_editor
                .as_mut()
                .map(|draft| &mut draft.command),
            QuickCommandInput::CommandLineDelay => self
                .store
                .command_editor
                .as_mut()
                .map(|draft| &mut draft.line_delay_ms),
            QuickCommandInput::CommandDescription => self
                .store
                .command_editor
                .as_mut()
                .map(|draft| &mut draft.description),
            QuickCommandInput::CommandHostPattern => self
                .store
                .command_editor
                .as_mut()
                .map(|draft| &mut draft.host_patterns),
            QuickCommandInput::ParameterName(index) => self
                .store
                .command_editor
                .as_mut()?
                .parameters
                .get_mut(index)
                .map(|parameter| &mut parameter.name),
            QuickCommandInput::ParameterLabel(index) => self
                .store
                .command_editor
                .as_mut()?
                .parameters
                .get_mut(index)
                .map(|parameter| &mut parameter.label),
            QuickCommandInput::ParameterDefault(index) => {
                if let Some(execution) = self.pending_execution.as_mut() {
                    execution
                        .parameter_values
                        .get_mut(index)
                        .map(|value| &mut **value)
                } else {
                    self.store
                        .command_editor
                        .as_mut()?
                        .parameters
                        .get_mut(index)
                        .map(|parameter| &mut parameter.default_value)
                }
            }
            QuickCommandInput::ParameterChoices(index) => self
                .store
                .command_editor
                .as_mut()?
                .parameters
                .get_mut(index)
                .map(|parameter| &mut parameter.choices),
            QuickCommandInput::CategoryName => self
                .store
                .category_editor
                .as_mut()
                .map(|draft| &mut draft.name),
            QuickCommandInput::CategoryColor => self
                .store
                .category_editor
                .as_mut()
                .map(|draft| &mut draft.icon_color),
        }
    }

    pub(in crate::workspace) fn replace_input(
        &mut self,
        input: QuickCommandInput,
        replacement_range: Option<std::ops::Range<usize>>,
        text: &str,
    ) -> bool {
        if self.store.focused_input != Some(input) {
            return false;
        }
        let Some(value) = self.input_value_mut(input) else {
            return false;
        };
        replace_utf16(value, replacement_range, text);
        if input == QuickCommandInput::Search {
            self.store.highlighted_command = None;
        }
        if input == QuickCommandInput::CategoryName {
            let draft_to_save = self
                .store
                .category_editor
                .as_ref()
                .filter(|draft| quick_command_category_draft_can_save(draft))
                .cloned();
            if let Some(draft) = draft_to_save {
                let saved_id = self.store.upsert_category(draft);
                if let Some(editor_draft) = self.store.category_editor.as_mut() {
                    if editor_draft.id.is_none() {
                        editor_draft.id = Some(saved_id);
                    }
                }
            }
        }
        true
    }

    fn move_highlight(
        &mut self,
        target: Option<&QuickCommandTargetContext>,
        direction: QuickCommandKeyDirection,
    ) {
        let visible_commands = self.visible_commands_for_targets(target);
        self.store.highlighted_command = quick_command_keyboard_highlight(
            &visible_commands,
            self.store.highlighted_command.as_deref(),
            direction,
        );
    }

    fn highlight_edge(&mut self, target: Option<&QuickCommandTargetContext>, end: bool) {
        let visible_commands = self.visible_commands_for_targets(target);
        self.store.highlighted_command = if end {
            visible_commands.last().map(|command| command.id.clone())
        } else {
            quick_command_highlight_at(&visible_commands, 0)
        };
    }

    fn prepare_highlighted_insertion(
        &mut self,
        target: Option<&QuickCommandTargetContext>,
    ) -> Option<String> {
        let visible_commands = self.visible_commands_for_targets(target);
        let selected_index = quick_command_highlighted_index(
            &visible_commands,
            self.store.highlighted_command.as_deref(),
        )
        .unwrap_or(0);
        let command = visible_commands.get(selected_index)?.command.clone();
        Some(self.prepare_insertion(command, self.pinned))
    }

    fn cycle_editor_focus(&mut self, input: QuickCommandInput, forward: bool) -> bool {
        let mut fields = Vec::new();
        if let Some(execution) = self.pending_execution.as_ref() {
            fields.extend((0..execution.command.parameters.len()).map(QuickCommandInput::ParameterDefault));
        } else if self.store.category_editor.is_some() {
            fields.push(QuickCommandInput::CategoryName);
            fields.push(QuickCommandInput::CategoryColor);
        } else if let Some(draft) = self.store.command_editor.as_ref() {
            fields.extend([
                QuickCommandInput::CommandName,
                QuickCommandInput::CommandText,
                QuickCommandInput::CommandLineDelay,
                QuickCommandInput::CommandDescription,
                QuickCommandInput::CommandHostPattern,
            ]);
            for (index, parameter) in draft.parameters.iter().enumerate() {
                fields.extend([
                    QuickCommandInput::ParameterName(index),
                    QuickCommandInput::ParameterLabel(index),
                ]);
                if parameter.kind != QuickCommandParameterKind::Secret {
                    fields.push(QuickCommandInput::ParameterDefault(index));
                }
                if parameter.kind == QuickCommandParameterKind::Choice {
                    fields.push(QuickCommandInput::ParameterChoices(index));
                }
            }
        }
        let Some(index) = fields.iter().position(|candidate| *candidate == input) else {
            return false;
        };
        let next_index = if forward {
            (index + 1) % fields.len()
        } else if index == 0 {
            fields.len() - 1
        } else {
            index - 1
        };
        let Some(next_input) = fields.get(next_index).copied() else {
            return false;
        };
        self.store.focused_input = Some(next_input);
        true
    }

    pub(in crate::workspace) fn blur_input(&mut self) -> bool {
        let was_focused = self.store.focused_input.take().is_some();
        was_focused
    }

    fn start_command_create(&mut self) {
        self.start_command_create_with_content(String::new());
    }

    pub(super) fn start_command_create_with_content(&mut self, command: String) {
        self.store.category_editor = None;
        self.manager_open = true;
        self.store.command_editor = Some(QuickCommandEditorDraft {
            id: None,
            name: String::new(),
            command,
            category: self.store.active_category.clone(),
            description: String::new(),
            host_patterns: String::new(),
            match_field: QuickCommandMatchField::Host,
            parameters: Vec::new(),
            protocols: Vec::new(),
            confirmation: QuickCommandConfirmationPolicy::Inherit,
            line_delay_ms: "1000".to_string(),
            syntax_help_expanded: false,
            created_at: now_ms(),
            sort_order: self
                .store
                .commands
                .iter()
                .map(|command| command.sort_order)
                .max()
                .unwrap_or(-1)
                .saturating_add(1),
        });
        self.store.focused_input = Some(QuickCommandInput::CommandName);
        self.store.highlighted_command = None;
    }

    fn start_command_edit(&mut self, command: QuickCommand) {
        let command_id = command.id.clone();
        let command_category = command.category.clone();
        self.store.category_editor = None;
        self.manager_open = true;
        self.manager_mode = QuickCommandManagerMode::Command;
        self.manager_command_category_filter = Some(command_category);
        self.button_context_menu = None;
        self.store.command_editor = Some(QuickCommandEditorDraft {
            id: Some(command_id),
            name: command.name,
            command: command.command,
            category: command.category,
            description: command.description.unwrap_or_default(),
            host_patterns: command.availability.host_patterns.join(", "),
            match_field: command.availability.match_field,
            parameters: command
                .parameters
                .into_iter()
                .map(|parameter| QuickCommandParameterEditorDraft {
                    name: parameter.name,
                    label: parameter.label,
                    kind: parameter.kind,
                    default_value: parameter.default_value.unwrap_or_default(),
                    choices: parameter.choices.join(", "),
                    required: parameter.required,
                })
                .collect(),
            protocols: command.availability.protocols,
            confirmation: command.confirmation,
            line_delay_ms: command.line_delay_ms.to_string(),
            syntax_help_expanded: false,
            created_at: command.created_at,
            sort_order: command.sort_order,
        });
        self.store.focused_input = Some(QuickCommandInput::CommandName);
        self.store.highlighted_command = None;
    }

    fn start_category_create(&mut self) {
        self.store.command_editor = None;
        self.command_input = None;
        self.command_input_subscription = None;
        self.store.category_editor = Some(QuickCommandCategoryDraft {
            id: None,
            name: String::new(),
            icon: QuickCommandIcon::Zap,
            icon_color: String::new(),
        });
        self.store.focused_input = Some(QuickCommandInput::CategoryName);
        self.store.highlighted_command = None;
    }

    fn start_category_edit(&mut self, category: QuickCommandCategory) {
        self.store.command_editor = None;
        self.command_input = None;
        self.command_input_subscription = None;
        self.store.category_editor = Some(QuickCommandCategoryDraft {
            id: Some(category.id),
            name: category.name,
            icon: category.icon,
            icon_color: category.icon_color.unwrap_or_default(),
        });
        self.store.focused_input = Some(QuickCommandInput::CategoryName);
        self.store.highlighted_command = None;
    }

    fn set_command_category(&mut self, category_id: String) {
        if let Some(draft) = self.store.command_editor.as_mut() {
            draft.category = category_id;
        }
    }

    fn add_command_parameter(&mut self) {
        let Some(draft) = self.store.command_editor.as_mut() else {
            return;
        };
        draft.parameters.push(QuickCommandParameterEditorDraft {
            name: String::new(),
            label: String::new(),
            kind: QuickCommandParameterKind::Text,
            default_value: String::new(),
            choices: String::new(),
            required: false,
        });
        self.store.focused_input = Some(QuickCommandInput::ParameterName(
            draft.parameters.len().saturating_sub(1),
        ));
    }

    fn remove_command_parameter(&mut self, index: usize) {
        if let Some(draft) = self.store.command_editor.as_mut()
            && index < draft.parameters.len()
        {
            draft.parameters.remove(index);
            self.store.focused_input = None;
        }
    }

    fn set_command_parameter_kind(&mut self, index: usize, kind: QuickCommandParameterKind) {
        if let Some(parameter) = self
            .store
            .command_editor
            .as_mut()
            .and_then(|draft| draft.parameters.get_mut(index))
        {
            parameter.kind = kind;
            if kind == QuickCommandParameterKind::Secret {
                // Secret values exist only in the execution dialog and never in persisted drafts.
                parameter.default_value.clear();
                parameter.choices.clear();
            }
        }
    }

    fn toggle_command_parameter_required(&mut self, index: usize) {
        if let Some(parameter) = self
            .store
            .command_editor
            .as_mut()
            .and_then(|draft| draft.parameters.get_mut(index))
        {
            parameter.required = !parameter.required;
        }
    }

    fn toggle_command_protocol(&mut self, protocol: QuickCommandTargetProtocol) {
        let Some(draft) = self.store.command_editor.as_mut() else {
            return;
        };
        if let Some(index) = draft
            .protocols
            .iter()
            .position(|candidate| *candidate == protocol)
        {
            draft.protocols.remove(index);
        } else {
            draft.protocols.push(protocol);
        }
    }

    fn set_execution_parameter_value(&mut self, index: usize, value: String) {
        if let Some(value_slot) = self
            .pending_execution
            .as_mut()
            .and_then(|execution| execution.parameter_values.get_mut(index))
        {
            *value_slot = Zeroizing::new(value);
        }
    }

    fn cancel_editor(&mut self) {
        self.store.command_editor = None;
        self.command_input = None;
        self.command_input_subscription = None;
        self.store.category_editor = None;
        self.store.focused_input = None;
        self.store.highlighted_command = None;
    }

    fn save_command_editor(&mut self) -> bool {
        let Some(draft) = self.store.command_editor.as_ref() else {
            return false;
        };
        if !quick_command_editor_can_save(draft) {
            return false;
        }
        let Some(draft) = self.store.command_editor.take() else {
            return false;
        };
        if !self.store.upsert_editor_command(draft.clone()) {
            self.store.command_editor = Some(draft);
            return false;
        }
        self.command_input = None;
        self.command_input_subscription = None;
        self.store.focused_input = None;
        self.store.highlighted_command = None;
        true
    }

    fn save_category_editor(&mut self) -> bool {
        let Some(draft) = self.store.category_editor.as_ref() else {
            return false;
        };
        if !quick_command_category_draft_can_save(draft) {
            return false;
        }
        let Some(draft) = self.store.category_editor.take() else {
            return false;
        };
        self.store.upsert_category(draft.clone());
        if self.store.last_persist_error.is_some() {
            self.store.category_editor = Some(draft);
            return false;
        }
        self.store.focused_input = None;
        self.store.highlighted_command = None;
        true
    }

    fn render_snapshot(
        &self,
        target: Option<&QuickCommandTargetContext>,
    ) -> QuickCommandsRenderSnapshot {
        let visible_commands = Arc::new(if self.manager_open {
            self.store
                .visible_commands_for_manager(self.manager_command_category_filter.as_deref())
        } else {
            self.visible_commands_for_targets(target)
        });
        let mut category_counts = HashMap::new();
        for command in &self.store.commands {
            *category_counts.entry(command.category.clone()).or_insert(0) += 1;
        }
        let signatures = visible_commands
            .iter()
            .map(quick_command_row_signature)
            .collect::<Vec<_>>();
        let list_spec = TauriVirtualListSpec::new(
            px(QUICK_COMMAND_LIST_ESTIMATED_HEIGHT),
            QUICK_COMMAND_LIST_OVERSCAN,
        );
        sync_tauri_variable_list_state_by_signatures(
            &self.list_state,
            &mut self.list_cache.borrow_mut(),
            "terminal-quick-commands",
            &signatures,
            list_spec,
        );
        QuickCommandsRenderSnapshot {
            categories: self.store.categories.clone(),
            category_counts,
            active_category: self.store.active_category.clone(),
            query: self.store.query.clone(),
            focused_input: self.store.focused_input,
            highlighted_command: self.store.highlighted_command.clone(),
            command_editor: self.store.command_editor.clone(),
            category_editor: self.store.category_editor.clone(),
            pending_category_delete: self.pending_category_delete.clone(),
            visible_commands,
            managing: self.manager_open,
            manager_category_dropdown_open: self.manager_category_dropdown_open,
            editor_category_menu_open: self.editor_category_menu_open,
            manager_mode: self.manager_mode,
            manager_command_category_filter: self.manager_command_category_filter.clone(),
            manager_search_open: self.manager_search_open,
        }
    }
}

impl WorkspaceApp {
    pub(super) fn quick_commands_render_snapshot(
        &self,
        cx: &mut Context<Self>,
    ) -> QuickCommandsRenderSnapshot {
        let target = self
            .active_pane_id(cx)
            .and_then(|pane_id| self.quick_command_context_for_pane(pane_id, cx));
        self.terminal
            .read(cx)
            .quick_commands
            .render_snapshot(target.as_ref())
    }

    pub(in crate::workspace) fn blur_terminal_quick_commands_input(
        &mut self,
        cx: &mut App,
    ) -> bool {
        let changed = self
            .terminal
            .update(cx, |terminal, _| terminal.quick_commands.blur_input());
        if changed {
            self.ime_marked_text = None;
            self.clear_ime_selection();
        }
        changed
    }

    pub(in crate::workspace) fn close_terminal_quick_commands_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let changed = self
            .terminal
            .update(cx, |terminal, _cx| terminal.quick_commands.close());
        if changed {
            self.ime_marked_text = None;
            self.clear_ime_selection();
        }
        changed
    }

    pub(in crate::workspace) fn open_quick_commands_manager(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        self.prepare_modal_interaction_boundary(cx);
        self.terminal
            .update(cx, |terminal, _cx| terminal.quick_commands.open_manager());
        self.ime_marked_text = None;
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    pub(in crate::workspace) fn close_quick_commands_manager(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let changed = self
            .terminal
            .update(cx, |terminal, _cx| terminal.quick_commands.close_manager());
        if changed {
            self.ime_marked_text = None;
            self.clear_ime_selection();
            cx.notify();
        }
        changed
    }

    pub(in crate::workspace) fn cancel_quick_command_category_delete(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let changed = self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.cancel_category_delete()
        });
        if changed {
            cx.notify();
        }
        changed
    }

    pub(in crate::workspace) fn confirm_quick_command_category_delete(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let changed = self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.confirm_category_delete()
        });
        if changed {
            if self
                .settings_store
                .settings()
                .terminal
                .command_bar
                .quick_bar_category
                .as_ref()
                .is_some_and(|id| {
                    !self
                        .terminal
                        .read(cx)
                        .quick_commands
                        .store
                        .categories
                        .iter()
                        .any(|c| &c.id == id)
                })
            {
                self.edit_settings(
                    |settings| settings.terminal.command_bar.quick_bar_category = None,
                    cx,
                );
            }
            cx.notify();
        }
        changed
    }

    pub(in crate::workspace) fn finish_terminal_quick_command_execution(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.finish_execution()
        });
    }

    fn insert_quick_command_into_command_bar(
        &mut self,
        command: String,
        keep_open: bool,
        cx: &mut Context<Self>,
    ) {
        let command = self.terminal.update(cx, |terminal, _cx| {
            terminal
                .quick_commands
                .prepare_insertion(command, keep_open)
        });
        self.ensure_terminal_command_composer_visible(cx);
        self.replace_terminal_command_composer_text(command, cx);
    }

    pub(in crate::workspace) fn handle_quick_commands_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        let Some(input) = self.terminal.read(cx).quick_commands.focused_input() else {
            return;
        };
        let target = self
            .active_pane_id(cx)
            .and_then(|pane_id| self.quick_command_context_for_pane(pane_id, cx));
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if input == QuickCommandInput::Search {
            match key {
                "escape" if !modifiers.platform && !modifiers.control => {
                    if self.terminal.read(cx).quick_commands.manager_open() {
                        self.terminal
                            .update(cx, |terminal, _cx| terminal.quick_commands.blur_input());
                    } else {
                        // Escape from the launcher returns input to the terminal.
                        self.close_terminal_quick_commands_panel(cx);
                        self.focus_active_pane(window, cx);
                    }
                    self.ime_marked_text = None;
                    cx.notify();
                    return;
                }
                "arrowdown" | "down" if !modifiers.platform && !modifiers.control => {
                    self.terminal.update(cx, |terminal, _cx| {
                        terminal
                            .quick_commands
                            .move_highlight(target.as_ref(), QuickCommandKeyDirection::Next)
                    });
                    cx.notify();
                    return;
                }
                "arrowup" | "up" if !modifiers.platform && !modifiers.control => {
                    self.terminal.update(cx, |terminal, _cx| {
                        terminal
                            .quick_commands
                            .move_highlight(target.as_ref(), QuickCommandKeyDirection::Previous)
                    });
                    cx.notify();
                    return;
                }
                "home" if !modifiers.platform && !modifiers.control => {
                    self.terminal.update(cx, |terminal, _cx| {
                        terminal
                            .quick_commands
                            .highlight_edge(target.as_ref(), false)
                    });
                    cx.notify();
                    return;
                }
                "end" if !modifiers.platform && !modifiers.control => {
                    self.terminal.update(cx, |terminal, _cx| {
                        terminal
                            .quick_commands
                            .highlight_edge(target.as_ref(), true)
                    });
                    cx.notify();
                    return;
                }
                "enter" if !modifiers.platform && !modifiers.control => {
                    let command = self.terminal.update(cx, |terminal, _cx| {
                        terminal
                            .quick_commands
                            .prepare_highlighted_insertion(target.as_ref())
                    });
                    if let Some(command) = command {
                        self.ensure_terminal_command_composer_visible(cx);
                        let document_id = self.replace_terminal_command_composer_text(command, cx);
                        self.focus_terminal_command_composer_input(document_id, window, cx);
                        cx.notify();
                    }
                    return;
                }
                _ => {}
            }
        }
        match key {
            "tab" if !modifiers.platform && !modifiers.control => {
                if self.terminal.update(cx, |terminal, _cx| {
                    terminal
                        .quick_commands
                        .cycle_editor_focus(input, !modifiers.shift)
                }) {
                    self.clear_ime_selection();
                    self.ime_marked_text = None;
                    let commands = &self.terminal.read(cx).quick_commands;
                    if commands.focused_input() == Some(QuickCommandInput::CommandText)
                        && let Some(editor) = commands.command_input.as_ref()
                    {
                        window.focus(&editor.read(cx).focus_handle(cx), cx);
                    } else {
                        window.focus(&self.focus_handle, cx);
                    }
                    cx.notify();
                }
            }
            "escape" => {
                if self.terminal.read(cx).quick_commands.execution_pending() {
                    self.cancel_quick_command_parameter_input(window, cx);
                    return;
                }
                if self
                    .terminal
                    .update(cx, |terminal, _cx| terminal.quick_commands.blur_input())
                {
                    self.ime_marked_text = None;
                    window.focus(&self.focus_handle, cx);
                    cx.notify();
                }
            }
            "enter" if matches!(input, QuickCommandInput::ParameterDefault(_)) => {
                self.confirm_quick_command_parameter_input(window, cx);
            }
            "enter"
                if matches!(
                    input,
                    QuickCommandInput::CategoryName | QuickCommandInput::CategoryColor
                ) =>
            {
                if self.terminal.update(cx, |terminal, _cx| {
                    terminal.quick_commands.save_category_editor()
                }) {
                    cx.notify();
                }
            }
            "enter"
                if matches!(
                    input,
                    QuickCommandInput::CommandName
                        | QuickCommandInput::CommandLineDelay
                        | QuickCommandInput::CommandDescription
                        | QuickCommandInput::CommandHostPattern
                ) =>
            {
                self.save_quick_command_editor(cx);
            }
            "space" | " "
                if quick_command_space_inserts_literal(
                    modifiers.platform,
                    modifiers.control,
                    modifiers.alt,
                ) =>
            {
                // Some GPUI platforms deliver Space without key_char, so the
                // platform text owner never commits it. Route that fallback
                // through the same IME replacement path as ordinary text.
                let target = WorkspaceImeTarget::QuickCommand(input);
                let replacement_range = self.ime_selection_range_for_target(target, cx);
                let caret = replacement_range
                    .as_ref()
                    .map(|range| range.start + " ".encode_utf16().count());
                self.replace_ime_target_text(target, replacement_range, " ", caret, cx);
                self.clear_ime_selection();
                if let Some(caret) = caret {
                    self.set_ime_selection_from_anchor(target, caret, caret);
                }
            }
            _ => {}
        }
    }

    pub(in crate::workspace) fn quick_command_input_value(
        &self,
        input: QuickCommandInput,
        cx: &App,
    ) -> Option<String> {
        self.terminal
            .read(cx)
            .quick_commands
            .input_value(input)
            .map(str::to_string)
    }

    pub(in crate::workspace) fn render_quick_command_parameter_modal(
        &self,
        execution: &QuickCommandExecutionDraft,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let is_single_parameter = execution.command.parameters.len() == 1;
        let title = if is_single_parameter {
            execution
                .command
                .parameters
                .first()
                .map(|p| {
                    if p.label.trim().is_empty() {
                        p.name.clone()
                    } else {
                        p.label.clone()
                    }
                })
                .unwrap_or_else(|| execution.command.name.clone())
        } else {
            execution.command.name.clone()
        };

        let mut fields = div().flex().flex_col().gap(px(10.0)).w_full();

        for (index, parameter) in execution.command.parameters.iter().enumerate() {
            let value = execution
                .parameter_values
                .get(index)
                .cloned()
                .unwrap_or_default();
            let input = if parameter.kind == QuickCommandParameterKind::Secret {
                self.render_quick_command_secret_input(
                    QuickCommandInput::ParameterDefault(index),
                    &value,
                    self.terminal.read(cx).quick_commands.store.focused_input,
                    String::new(),
                    cx,
                )
            } else {
                self.render_quick_command_text_input(
                    QuickCommandInput::ParameterDefault(index),
                    value.to_string(),
                    self.terminal.read(cx).quick_commands.store.focused_input,
                    String::new(),
                    cx,
                )
            };
            let mut field = div().flex().flex_col().gap(px(4.0));
            if !is_single_parameter {
                field = field.child(
                    div()
                        .text_size(px(self.tokens.metrics.ui_text_xs))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(theme.text_muted))
                        .child(parameter.label.clone()),
                );
            }
            field = field.child(input);

            if parameter.kind == QuickCommandParameterKind::Choice {
                let mut choices = div().flex().items_center().gap(px(4.0)).flex_wrap();
                for choice in &parameter.choices {
                    let choice_for_click = choice.clone();
                    let selected = choice == value.as_str();
                    choices = choices.child(
                        self.quick_command_text_button(
                            choice.clone(),
                            true,
                            cx.listener(move |this, _event, _window, cx| {
                                this.terminal.update(cx, |terminal, _cx| {
                                    terminal.quick_commands.set_execution_parameter_value(
                                        index,
                                        choice_for_click.clone(),
                                    )
                                });
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        )
                        .border_color(if selected {
                            rgb(theme.accent)
                        } else {
                            rgba((theme.border << 8) | 0x99)
                        }),
                    );
                }
                field = field.child(choices);
            }
            fields = fields.child(field);
        }

        let dialog = div()
            .w(px(380.0))
            .p(px(16.0))
            .rounded(px(self.tokens.radii.lg))
            .border_1()
            .border_color(rgb(theme.border))
            .bg(rgb(theme.bg_panel))
            .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&self.tokens))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.as_str() == "enter" {
                    this.confirm_quick_command_parameter_input(window, cx);
                    cx.stop_propagation();
                } else if event.keystroke.key.as_str() == "escape" {
                    this.cancel_quick_command_parameter_input(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(self.tokens.metrics.ui_text_sm))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(theme.text))
                            .child(title),
                    ),
            )
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .font_family(settings_mono_font_family(self.settings_store.settings()))
                    .text_color(rgb(theme.text_muted))
                    .truncate()
                    .child(execution.command.command.clone()),
            )
            .child(fields);

        dismissible_dialog_backdrop()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, window, cx| {
                    this.cancel_quick_command_parameter_input(window, cx);
                    cx.stop_propagation();
                }),
            )
            .child(overlay_content_boundary(dialog))
            .into_any_element()
    }

    pub(in crate::workspace) fn toggle_quick_commands_manager_category_dropdown(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.manager_category_dropdown_open =
                !terminal.quick_commands.manager_category_dropdown_open;
        });
        cx.notify();
    }

    pub(in crate::workspace) fn toggle_quick_command_editor_category_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.editor_category_menu_open =
                !terminal.quick_commands.editor_category_menu_open;
            terminal.quick_commands.manager_category_dropdown_open = false;
        });
        cx.notify();
    }

    pub(in crate::workspace) fn close_quick_commands_manager_popovers(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let changed = self.terminal.update(cx, |terminal, _cx| {
            let changed = terminal.quick_commands.manager_category_dropdown_open
                || terminal.quick_commands.editor_category_menu_open;
            terminal.quick_commands.manager_category_dropdown_open = false;
            terminal.quick_commands.editor_category_menu_open = false;
            changed
        });
        if changed {
            cx.notify();
        }
    }

    pub(in crate::workspace) fn delete_manager_category(
        &mut self,
        category_id: &str,
        cx: &mut Context<Self>,
    ) {
        let snapshot = self.quick_commands_render_snapshot(cx);
        let count = snapshot.category_counts.get(category_id).copied().unwrap_or(0);
        let Some(cat) = snapshot.categories.iter().find(|c| c.id == category_id).cloned() else {
            return;
        };
        if count > 0 {
            self.terminal.update(cx, |terminal, _cx| {
                terminal.quick_commands.pending_category_delete = Some(QuickCommandCategoryDeletePrompt {
                    id: cat.id,
                    name: cat.name,
                });
                terminal.quick_commands.manager_category_dropdown_open = false;
            });
        } else {
            self.terminal.update(cx, |terminal, _cx| {
                terminal.quick_commands.store.delete_category(&cat.id);
                terminal.quick_commands.manager_category_dropdown_open = false;
            });
        }
        cx.notify();
    }


    pub(in crate::workspace) fn set_quick_commands_manager_mode(
        &mut self,
        mode: QuickCommandManagerMode,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.manager_mode = mode;
            // Switching modes drops in-progress drafts so the inspector never
            // shows a stale editor from the other mode.
            terminal.quick_commands.store.command_editor = None;
            terminal.quick_commands.store.category_editor = None;
            terminal.quick_commands.manager_category_dropdown_open = false;
            terminal.quick_commands.manager_search_open = false;
        });
        cx.notify();
    }

    pub(in crate::workspace) fn select_quick_commands_manager_category(
        &mut self,
        category_filter: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.manager_command_category_filter = category_filter;
            terminal.quick_commands.manager_category_dropdown_open = false;
        });
        cx.notify();
    }

    pub(in crate::workspace) fn toggle_quick_commands_manager_search(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.manager_search_open =
                !terminal.quick_commands.manager_search_open;
            if terminal.quick_commands.manager_search_open {
                terminal.quick_commands.store.focused_input = Some(QuickCommandInput::Search);
            } else {
                terminal.quick_commands.store.query.clear();
            }
        });
        cx.notify();
    }

    pub(in crate::workspace) fn set_quick_command_category_editor_icon(
        &mut self,
        icon: QuickCommandIcon,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            let draft_to_save =
                if let Some(draft) = terminal.quick_commands.store.category_editor.as_mut() {
                    draft.icon = icon;
                    if quick_command_category_draft_can_save(draft) {
                        Some(draft.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
            if let Some(draft) = draft_to_save {
                let saved_id = terminal.quick_commands.store.upsert_category(draft);
                if let Some(editor_draft) =
                    terminal.quick_commands.store.category_editor.as_mut()
                {
                    if editor_draft.id.is_none() {
                        editor_draft.id = Some(saved_id);
                    }
                }
            }
        });
        cx.notify();
    }

    pub(in crate::workspace) fn move_quick_command_to(
        &mut self,
        source_id: &str,
        target_id: &str,
        cx: &mut Context<Self>,
    ) {
        let changed = self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.store.move_command_to(source_id, target_id)
        });
        if changed {
            cx.notify();
        }
    }

    pub(in crate::workspace) fn delete_quick_commands_manager_category_with_commands(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let changed = self.terminal.update(cx, |terminal, _cx| {
            let Some(prompt) = terminal.quick_commands.pending_category_delete.take() else {
                return false;
            };
            terminal.quick_commands.store.highlighted_command = None;
            terminal
                .quick_commands
                .store
                .delete_category_with_commands(&prompt.id)
        });
        if changed {
            cx.notify();
        }
        changed
    }

    pub(in crate::workspace) fn render_quick_commands_manager_modal(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let snapshot = self.quick_commands_render_snapshot(cx);
        let mut manager = oxideterm_gpui_ui::modal_container(&self.tokens)
            .w(px(QUICK_COMMANDS_MANAGER_WIDTH))
            .max_w_full()
            .h(px(QUICK_COMMANDS_MANAGER_HEIGHT))
            .max_h_full()
            .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&self.tokens))
            .flex()
            .flex_col()
            .relative()
            .font_family(settings_ui_font_family(
                &self.settings_store.settings().appearance.ui_font_family,
            ))
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .child(self.render_quick_commands_manager_header(&snapshot, cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .overflow_hidden()
                    .child(self.render_quick_command_manager_master(&snapshot, cx))
                    .child(self.render_quick_command_manager_inspector(&snapshot, cx)),
            );

        if snapshot.manager_category_dropdown_open {
            if let Some(dropdown) = self.render_quick_commands_manager_category_filter_dropdown(
                &snapshot,
                cx,
            ) {
                manager = manager.child(dropdown);
            }
        }

        if snapshot.editor_category_menu_open {
            if let Some(menu) = self.render_quick_command_editor_category_menu(
                &snapshot,
                cx,
            ) {
                manager = manager.child(menu);
            }
        }

        let manager_dialog = dismissible_dialog_backdrop()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.close_quick_commands_manager(cx);
                    cx.stop_propagation();
                }),
            )
            .child(overlay_content_boundary(manager))
            .into_any_element();

        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .child(manager_dialog)
            .when_some(snapshot.pending_category_delete.as_ref(), |root, prompt| {
                root.child(self.render_quick_commands_manager_delete_group_dialog(
                    &snapshot,
                    prompt,
                    cx,
                ))
            })
            .into_any_element()
    }

    fn render_quick_commands_manager_delete_group_dialog(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        prompt: &QuickCommandCategoryDeletePrompt,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let defaults = default_quick_command_categories();
        let destination = snapshot
            .categories
            .iter()
            .chain(defaults.iter())
            .find(|category| category.id == FALLBACK_QUICK_COMMAND_CATEGORY_ID)
            .map(|category| category.name.clone())
            .unwrap_or_else(|| FALLBACK_QUICK_COMMAND_CATEGORY_ID.to_string());
        let description = self
            .i18n
            .t("terminal.quick_commands.delete_group_description")
            .replace("{{name}}", &prompt.name)
            .replace("{{destination}}", &destination);

        let cat = snapshot.categories.iter().find(|c| c.id == prompt.id);
        let category_icon = cat.map(|c| c.icon).unwrap_or(QuickCommandIcon::Zap);
        let count = snapshot
            .category_counts
            .get(prompt.id.as_str())
            .copied()
            .unwrap_or(0);

        let card = div()
            .w(px(420.0))
            .max_w_full()
            .rounded(px(self.tokens.radii.lg))
            .overflow_hidden()
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x99))
            .bg(rgba((theme.bg_elevated << 8) | 0xfc))
            .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&self.tokens))
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(14.0))
                    .px(px(24.0))
                    .pt(px(24.0))
                    .pb(px(20.0))
                    .child(
                        div()
                            .size(px(48.0))
                            .rounded_full()
                            .border_1()
                            .border_color(rgba((theme.error << 8) | 0x40))
                            .bg(rgba((theme.error << 8) | 0x18))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Self::render_lucide_icon(
                                LucideIcon::Trash2,
                                22.0,
                                rgb(theme.error),
                            )),
                    )
                    .child(
                        div()
                            .text_align(gpui::TextAlign::Center)
                            .text_size(px(self.tokens.metrics.ui_text_base))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(theme.text))
                            .child(self.i18n.t("terminal.quick_commands.delete_group")),
                    )
                    .child(
                        div()
                            .w_full()
                            .rounded(px(self.tokens.radii.md))
                            .border_1()
                            .border_color(rgba((theme.border << 8) | 0x4d))
                            .bg(rgba((theme.bg << 8) | 0x40))
                            .px(px(12.0))
                            .py(px(10.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(Self::render_lucide_icon(
                                        quick_command_lucide_icon(category_icon),
                                        15.0,
                                        rgb(theme.accent),
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(self.tokens.metrics.ui_text_sm))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(theme.text))
                                            .child(prompt.name.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(2.0))
                                    .rounded_full()
                                    .bg(rgba((theme.border << 8) | 0x40))
                                    .text_size(px(self.tokens.metrics.ui_text_xs))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(theme.text_muted))
                                    .child(format!(
                                        "{} {}",
                                        count,
                                        self.i18n.t("terminal.quick_commands.commands")
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .text_align(gpui::TextAlign::Center)
                            .text_size(px(self.tokens.metrics.ui_text_xs))
                            .text_color(rgb(theme.text_muted))
                            .child(description),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .px(px(20.0))
                    .py(px(14.0))
                    .border_t_1()
                    .border_color(rgba((theme.border << 8) | 0x40))
                    .bg(rgba((theme.bg << 8) | 0x20))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        button_with(
                            &self.tokens,
                            self.i18n.t("common.actions.cancel"),
                            ButtonOptions {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Default,
                                radius: ButtonRadius::Md,
                                disabled: false,
                            },
                        )
                        .on_mouse_down(MouseButton::Left, cx.listener(
                            |this, _event, _window, cx| {
                                this.cancel_quick_command_category_delete(cx);
                                cx.stop_propagation();
                            },
                        )),
                    )
                    .child(
                        button_with(
                            &self.tokens,
                            self.i18n.t("terminal.quick_commands.move_commands"),
                            ButtonOptions {
                                variant: ButtonVariant::Outline,
                                size: ButtonSize::Default,
                                radius: ButtonRadius::Md,
                                disabled: false,
                            },
                        )
                        .on_mouse_down(MouseButton::Left, cx.listener(
                            |this, _event, _window, cx| {
                                this.confirm_quick_command_category_delete(cx);
                                cx.stop_propagation();
                            },
                        )),
                    )
                    .child(
                        button_with(
                            &self.tokens,
                            self.i18n.t("terminal.quick_commands.delete"),
                            ButtonOptions {
                                variant: ButtonVariant::Destructive,
                                size: ButtonSize::Default,
                                radius: ButtonRadius::Md,
                                disabled: false,
                            },
                        )
                        .on_mouse_down(MouseButton::Left, cx.listener(
                            |this, _event, _window, cx| {
                                this.delete_quick_commands_manager_category_with_commands(cx);
                                cx.stop_propagation();
                            },
                        )),
                    ),
            );

        dismissible_dialog_backdrop()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                this.cancel_quick_command_category_delete(cx);
                cx.stop_propagation();
            }))
            .child(overlay_content_boundary(card))
            .into_any_element()
    }

    fn render_quick_commands_manager_header(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let group_mode = snapshot.manager_mode == QuickCommandManagerMode::Group;

        let make_tab = |label: String, icon: LucideIcon, active: bool, mode: QuickCommandManagerMode| {
            div()
                .h(px(24.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(5.0))
                .rounded(px(self.tokens.radii.sm))
                .cursor_pointer()
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .font_weight(if active {
                    gpui::FontWeight::MEDIUM
                } else {
                    gpui::FontWeight::NORMAL
                })
                .bg(if active {
                    rgba((theme.accent << 8) | 0x2e)
                } else {
                    rgba(0x00000000)
                })
                .text_color(if active {
                    rgb(theme.accent)
                } else {
                    rgb(theme.text_muted)
                })
                .hover(move |s| {
                    s.bg(if active {
                        rgba((theme.accent << 8) | 0x3d)
                    } else {
                        rgba((theme.bg_hover << 8) | 0x80)
                    })
                    .text_color(if active {
                        rgb(theme.accent)
                    } else {
                        rgb(theme.text)
                    })
                })
                .child(Self::render_lucide_icon(
                    icon,
                    12.0,
                    if active {
                        rgb(theme.accent)
                    } else {
                        rgb(theme.text_muted)
                    },
                ))
                .child(div().truncate().child(label))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        this.set_quick_commands_manager_mode(mode, cx);
                        cx.stop_propagation();
                    }),
                )
        };

        let mode_tabs = div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .p(px(2.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x80))
            .bg(rgba((theme.bg << 8) | 0x60))
            .child(make_tab(
                self.i18n.t("terminal.quick_commands.groups"),
                LucideIcon::Folder,
                group_mode,
                QuickCommandManagerMode::Group,
            ))
            .child(make_tab(
                self.i18n.t("terminal.quick_commands.commands"),
                LucideIcon::Terminal,
                !group_mode,
                QuickCommandManagerMode::Command,
            ));

        let add_button = if group_mode {
            self.render_quick_commands_manager_add_group_button(cx)
        } else {
            self.render_quick_commands_manager_add_command_button(cx)
        };

        div()
            .h(px(52.0))
            .flex_none()
            .px(px(16.0))
            .border_b_1()
            .border_color(rgba((theme.border << 8) | 0x40))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(Self::render_lucide_icon(
                                LucideIcon::Send,
                                16.0,
                                rgb(theme.accent),
                            ))
                            .child(
                                div()
                                    .text_size(px(self.tokens.metrics.ui_text_base))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(theme.text))
                                    .child(self.i18n.t("terminal.quick_commands.title")),
                            ),
                    )
                    .child(
                        div()
                            .h(px(14.0))
                            .w(px(1.0))
                            .mx(px(2.0))
                            .bg(rgba((theme.border << 8) | 0x66)),
                    )
                    .child(mode_tabs)
                    .child(
                        div()
                            .h(px(14.0))
                            .w(px(1.0))
                            .mx(px(2.0))
                            .bg(rgba((theme.border << 8) | 0x66)),
                    )
                    .child(add_button)
                    .when(!group_mode, |header| {
                        header.child(self.render_quick_commands_manager_category_filter_button(
                            snapshot, cx,
                        ))
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .when(!group_mode, |right| {
                        right.child(self.render_quick_commands_manager_search_control(snapshot, cx))
                    })
                    .child(self.quick_command_tooltip_icon_button(
                        LucideIcon::Upload,
                        "quick-commands-import",
                        self.i18n.t("terminal.quick_commands.import"),
                        |this, _event, _window, cx| {
                            this.import_quick_commands(cx);
                            cx.stop_propagation();
                        },
                        cx,
                    ))
                    .child(self.quick_command_tooltip_icon_button(
                        LucideIcon::Download,
                        "quick-commands-export",
                        self.i18n.t("terminal.quick_commands.export"),
                        |this, _event, _window, cx| {
                            this.export_quick_commands(cx);
                            cx.stop_propagation();
                        },
                        cx,
                    ))
                    .child(self.quick_command_icon_button(
                        LucideIcon::X,
                        |this, _event, _window, cx| {
                            this.close_quick_commands_manager(cx);
                            cx.stop_propagation();
                        },
                        cx,
                    )),
            )
            .into_any_element()
    }

    fn render_quick_commands_manager_add_group_button(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        div()
            .id("quick-commands-manager-add-group-btn")
            .h(px(28.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x80))
            .bg(rgba((theme.accent << 8) | 0x1f))
            .hover(|style| {
                style
                    .bg(rgba((theme.accent << 8) | 0x33))
                    .border_color(rgb(theme.accent))
            })
            .cursor_pointer()
            .child(Self::render_lucide_icon(LucideIcon::Plus, 11.0, rgb(theme.accent)))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(theme.accent))
                    .child(self.i18n.t("terminal.quick_commands.add_group_short")),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, window, cx| {
                    this.start_quick_command_category_create(cx);
                    window.focus(&this.focus_handle, cx);
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    fn render_quick_commands_manager_category_filter_button(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let is_open = snapshot.manager_category_dropdown_open;
        let selected_category = snapshot
            .manager_command_category_filter
            .as_deref()
            .and_then(|id| snapshot.categories.iter().find(|category| category.id == id));
        let name = selected_category
            .map(|category| category.name.clone())
            .unwrap_or_else(|| self.i18n.t("terminal.quick_commands.all_categories"));
        let category_icon = selected_category.map(|category| category.icon);
        let count = if let Some(category) = selected_category {
            snapshot
                .category_counts
                .get(category.id.as_str())
                .copied()
                .unwrap_or(0)
        } else {
            snapshot.category_counts.values().sum()
        };

        let button = div()
            .id("quick-commands-manager-category-filter-btn")
            .h(px(28.0))
            .w(px(QUICK_COMMANDS_MANAGER_CATEGORY_DROPDOWN_WIDTH))
            .px(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(if is_open {
                rgb(theme.accent)
            } else {
                rgba((theme.border << 8) | 0x80)
            })
            .bg(if is_open {
                rgba((theme.accent << 8) | 0x22)
            } else {
                rgba((theme.bg << 8) | 0x66)
            })
            .hover(|style| {
                style
                    .bg(rgba((theme.bg_hover << 8) | 0xcc))
                    .border_color(rgb(theme.accent))
            })
            .cursor_pointer()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .when_some(category_icon, |row, icon| {
                        row.child(Self::render_lucide_icon(
                            quick_command_lucide_icon(icon),
                            12.0,
                            if is_open {
                                rgb(theme.accent)
                            } else {
                                rgb(theme.text_muted)
                            },
                        ))
                    })
                    .child(
                        div()
                            .truncate()
                            .text_size(px(self.tokens.metrics.ui_text_xs))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(rgb(theme.text))
                            .child(name),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex_none()
                            .h(px(16.0))
                            .min_w(px(16.0))
                            .px(px(5.0))
                            .rounded_full()
                            .bg(rgba((theme.border << 8) | 0x40))
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if is_open {
                                rgb(theme.accent)
                            } else {
                                rgb(theme.text_muted)
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(count.to_string()),
                    )
                    .child(Self::render_lucide_icon(
                        LucideIcon::ChevronDown,
                        10.0,
                        rgb(theme.text_muted),
                    )),
            )
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                this.toggle_quick_commands_manager_category_dropdown(cx);
                cx.stop_propagation();
            }));

        select_anchor_probe(
            SelectAnchorId::QuickCommandsManagerCategoryFilter,
            button,
            {
                let workspace = cx.entity();
                move |anchor, _window, cx| {
                    let _ = workspace.update(cx, |this, _cx| {
                        this.select_anchors.insert(anchor.id, anchor);
                    });
                }
            },
        )
        .into_any_element()
    }

    fn render_quick_commands_manager_search_control(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if snapshot.manager_search_open {
            return div()
                .w(px(160.0))
                .child(self.render_quick_command_text_input(
                    QuickCommandInput::Search,
                    snapshot.query.clone(),
                    snapshot.focused_input,
                    self.i18n.t("terminal.quick_commands.search"),
                    cx,
                ))
                .into_any_element();
        }
        self.quick_command_tooltip_icon_button(
            LucideIcon::Search,
            "quick-commands-manager-search",
            self.i18n.t("terminal.quick_commands.search"),
            |this, _event, _window, cx| {
                this.toggle_quick_commands_manager_search(cx);
                cx.stop_propagation();
            },
            cx,
        )
        .into_any_element()
    }

    fn render_quick_commands_manager_add_command_button(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        div()
            .id("quick-commands-manager-add-cmd-btn")
            .h(px(28.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x80))
            .bg(rgba((theme.accent << 8) | 0x1f))
            .hover(|style| {
                style
                    .bg(rgba((theme.accent << 8) | 0x33))
                    .border_color(rgb(theme.accent))
            })
            .cursor_pointer()
            .child(Self::render_lucide_icon(
                LucideIcon::Plus,
                11.0,
                rgb(theme.accent),
            ))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(theme.accent))
                    .child(self.i18n.t("terminal.quick_commands.add_command_short")),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, window, cx| {
                    this.start_quick_command_create(cx);
                    window.focus(&this.focus_handle, cx);
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    fn render_quick_commands_manager_category_filter_dropdown(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let theme = self.tokens.ui;
        let current_filter = snapshot.manager_command_category_filter.clone();
        let anchor = self
            .select_anchors
            .get(&SelectAnchorId::QuickCommandsManagerCategoryFilter)?;

        let mut list = div().flex().flex_col().gap(px(2.0));

        // "全部" reveals every command across groups for global search.
        let all_selected = current_filter.is_none();
        let all_count: usize = snapshot.category_counts.values().sum();
        list = list.child(
            div()
                .h(px(28.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .rounded(px(self.tokens.radii.sm))
                .bg(if all_selected {
                    rgba((theme.accent << 8) | 0x22)
                } else {
                    rgba(0x00000000)
                })
                .hover(|style| style.bg(rgba((theme.bg_hover << 8) | 0xb3)))
                .cursor_pointer()
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .text_color(if all_selected {
                    rgb(theme.accent)
                } else {
                    rgb(theme.text)
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(self.i18n.t("terminal.quick_commands.all_categories")),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex_none()
                                .h(px(16.0))
                                .min_w(px(16.0))
                                .px(px(5.0))
                                .rounded_full()
                                .bg(rgba((theme.border << 8) | 0x40))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(if all_selected {
                                    rgb(theme.accent)
                                } else {
                                    rgb(theme.text_muted)
                                })
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(all_count.to_string()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .w(px(12.0))
                                .h(px(12.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(all_selected, |slot| {
                                    slot.child(Self::render_lucide_icon(
                                        LucideIcon::Check,
                                        12.0,
                                        rgb(theme.accent),
                                    ))
                                }),
                        ),
                )
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                    this.select_quick_commands_manager_category(None, cx);
                    cx.stop_propagation();
                })),
        );

        for category in &snapshot.categories {
            let is_selected = current_filter.as_deref() == Some(category.id.as_str());
            let cat_id = category.id.clone();
            let count = snapshot
                .category_counts
                .get(category.id.as_str())
                .copied()
                .unwrap_or(0);
            list = list.child(
                div()
                    .h(px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.0))
                    .rounded(px(self.tokens.radii.sm))
                    .bg(if is_selected {
                        rgba((theme.accent << 8) | 0x22)
                    } else {
                        rgba(0x00000000)
                    })
                    .hover(|style| style.bg(rgba((theme.bg_hover << 8) | 0xb3)))
                    .cursor_pointer()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .text_color(if is_selected {
                        rgb(theme.accent)
                    } else {
                        rgb(theme.text)
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(Self::render_lucide_icon(
                                quick_command_lucide_icon(category.icon),
                                13.0,
                                if is_selected {
                                    rgb(theme.accent)
                                } else {
                                    rgb(theme.text_muted)
                                },
                            ))
                            .child(div().flex_1().truncate().child(category.name.clone())),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .flex_none()
                                    .h(px(16.0))
                                    .min_w(px(16.0))
                                    .px(px(5.0))
                                    .rounded_full()
                                    .bg(rgba((theme.border << 8) | 0x40))
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(if is_selected {
                                        rgb(theme.accent)
                                    } else {
                                        rgb(theme.text_muted)
                                    })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(count.to_string()),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .w(px(12.0))
                                    .h(px(12.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(is_selected, |slot| {
                                        slot.child(Self::render_lucide_icon(
                                            LucideIcon::Check,
                                            12.0,
                                            rgb(theme.accent),
                                        ))
                                    }),
                            ),
                    )
                    .on_mouse_down(MouseButton::Left, cx.listener({
                        let id = cat_id.clone();
                        move |this, _event, _window, cx| {
                            this.select_quick_commands_manager_category(Some(id.clone()), cx);
                            cx.stop_propagation();
                        }
                    })),
            );
        }

        let popup = div()
            .id("quick-commands-manager-category-filter-dropdown")
            .w(px(QUICK_COMMANDS_MANAGER_CATEGORY_DROPDOWN_WIDTH))
            .max_h(px(260.0))
            .overflow_y_scroll()
            .p(px(4.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x99))
            .bg(rgba((theme.bg_elevated << 8) | 0xfc))
            .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&self.tokens))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(list);

        Some(
            popover_backdrop()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.terminal.update(cx, |t, _| {
                            t.quick_commands.manager_category_dropdown_open = false;
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _, _, cx| {
                        this.terminal.update(cx, |t, _| {
                            t.quick_commands.manager_category_dropdown_open = false;
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .child(
                    deferred(
                        anchored()
                            .anchor(Corner::TopLeft)
                            .position(anchor.bounds.bottom_left())
                            .offset(point(px(0.0), px(4.0)))
                            .position_mode(AnchoredPositionMode::Window)
                            .child(popup),
                    )
                    .with_priority(oxideterm_gpui_ui::modal::TAURI_SELECT_LAYER_PRIORITY),
                )
                .into_any_element(),
        )
    }

    fn export_quick_commands(&mut self, cx: &mut Context<Self>) {
        let export = self
            .terminal
            .read(cx)
            .quick_commands
            .store
            .export_snapshot_json();
        let Ok(snapshot_json) = export else {
            self.push_workspace_notice(
                TerminalNotice {
                    title: self.i18n.t("terminal.quick_commands.export_failed"),
                    description: None,
                    status_text: None,
                    progress: None,
                    variant: TerminalNoticeVariant::Error,
                },
                cx,
            );
            return;
        };
        // Command bodies may contain user-entered sensitive literals during explicit export.
        let snapshot_json = Zeroizing::new(snapshot_json);
        let receiver = cx.prompt_for_new_path(
            &quick_commands_export_directory(),
            Some("oxideterm-quick-commands.json"),
        );
        cx.spawn(async move |weak, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => {
                    oxideterm_atomic_file::durable_write(&path, snapshot_json.as_bytes())
                        .map(|()| Some(path))
                        .map_err(|_| ())
                }
                Ok(Ok(None)) => Ok(None),
                Ok(Err(_)) | Err(_) => Err(()),
            };
            let _ = weak.update(cx, |this, cx| match result {
                Ok(Some(path)) => this.push_workspace_notice(
                    TerminalNotice {
                        title: this.i18n.t("terminal.quick_commands.export_success"),
                        description: Some(path.to_string_lossy().to_string()),
                        status_text: None,
                        progress: None,
                        variant: TerminalNoticeVariant::Success,
                    },
                    cx,
                ),
                Ok(None) => {}
                Err(()) => this.push_workspace_notice(
                    TerminalNotice {
                        title: this.i18n.t("terminal.quick_commands.export_failed"),
                        description: None,
                        status_text: None,
                        progress: None,
                        variant: TerminalNoticeVariant::Error,
                    },
                    cx,
                ),
            });
        })
        .detach();
    }

    fn import_quick_commands(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(
                self.i18n.t("terminal.quick_commands.import"),
            )),
        });
        cx.spawn(async move |weak, cx| {
            let selected_path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => None,
                Ok(Err(_)) | Err(_) => {
                    let _ = weak.update(cx, |this, cx| {
                        this.push_workspace_notice(
                            TerminalNotice {
                                title: this.i18n.t("terminal.quick_commands.import_failed"),
                                description: None,
                                status_text: None,
                                progress: None,
                                variant: TerminalNoticeVariant::Error,
                            },
                            cx,
                        );
                    });
                    return;
                }
            };
            let Some(path) = selected_path else {
                return;
            };
            let snapshot_json = fs::metadata(&path)
                .map_err(|_| ())
                .and_then(|metadata| {
                    (metadata.len() <= oxideterm_quick_commands::MAX_QUICK_COMMANDS_FILE_BYTES)
                        .then_some(())
                        .ok_or(())
                })
                .and_then(|()| {
                    fs::read_to_string(&path)
                        .map(Zeroizing::new)
                        .map_err(|_| ())
                });
            let _ = weak.update(cx, |this, cx| {
                let Ok(snapshot_json) = snapshot_json else {
                    this.push_workspace_notice(
                        TerminalNotice {
                            title: this.i18n.t("terminal.quick_commands.import_failed"),
                            description: None,
                            status_text: None,
                            progress: None,
                            variant: TerminalNoticeVariant::Error,
                        },
                        cx,
                    );
                    return;
                };
                let result = this.terminal.update(cx, |terminal, _cx| {
                    // Desktop import keeps every existing record and renames conflicts.
                    terminal
                        .quick_commands
                        .store
                        .apply_snapshot_json(&snapshot_json, QuickCommandImportStrategy::Rename)
                });
                if result.errors.is_empty() {
                    let title = this
                        .i18n
                        .t("terminal.quick_commands.import_success")
                        .replace("{{count}}", &result.imported.to_string());
                    this.push_workspace_notice(
                        TerminalNotice {
                            title,
                            description: None,
                            status_text: None,
                            progress: None,
                            variant: TerminalNoticeVariant::Success,
                        },
                        cx,
                    );
                    cx.notify();
                } else {
                    this.push_workspace_notice(
                        TerminalNotice {
                            title: this.i18n.t("terminal.quick_commands.import_failed"),
                            description: None,
                            status_text: None,
                            progress: None,
                            variant: TerminalNoticeVariant::Error,
                        },
                        cx,
                    );
                }
            });
        })
        .detach();
    }

    fn render_quick_command_manager_master(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let content = if snapshot.manager_mode == QuickCommandManagerMode::Group {
            self.render_quick_commands_manager_group_list(snapshot, cx)
        } else {
            self.render_quick_command_rows(snapshot, cx)
        };
        div()
            .w(px(QUICK_COMMANDS_MANAGER_SIDEBAR_WIDTH))
            .h_full()
            .flex_none()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgba((theme.border << 8) | 0x66))
            .bg(rgba((theme.bg << 8) | 0x40))
            .child(content)
            .into_any_element()
    }

    fn render_quick_commands_manager_group_list(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let mut list = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(6.0))
            .pt(px(6.0))
            .pb(px(6.0));
        for category in &snapshot.categories {
            let is_active = category.id == snapshot.active_category;
            let count = snapshot
                .category_counts
                .get(category.id.as_str())
                .copied()
                .unwrap_or(0);
            let can_delete = category.id != FALLBACK_QUICK_COMMAND_CATEGORY_ID;
            let cat_for_edit = category.clone();
            let cat_id_for_delete = category.id.clone();
            let drag = QuickCommandCategoryDrag {
                id: category.id.clone(),
                name: category.name.clone(),
                position: gpui::Point::default(),
                tokens: self.tokens,
            };
            let cat_id_for_drop = category.id.clone();

            let row = div()
                .h(px(34.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(self.tokens.radii.sm))
                .border_1()
                .border_color(if is_active {
                    rgba((theme.accent << 8) | 0x66)
                } else {
                    rgba(0x00000000)
                })
                .bg(if is_active {
                    rgba((theme.accent << 8) | 0x22)
                } else {
                    rgba(0x00000000)
                })
                .hover(|style| style.bg(rgba((theme.bg_hover << 8) | 0xb3)))
                .cursor_pointer()
                .drag_over::<QuickCommandCategoryDrag>({
                    let target = cat_id_for_drop.clone();
                    move |row, drag, _, _| {
                        if drag.id != target {
                            row.bg(rgba((theme.accent << 8) | 0x33))
                        } else {
                            row
                        }
                    }
                })
                .on_drop(cx.listener({
                    let target = cat_id_for_drop.clone();
                    move |this, drag: &QuickCommandCategoryDrag, _, cx| {
                        this.terminal.update(cx, |terminal, _| {
                            terminal.quick_commands.store.move_category(&drag.id, &target);
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .id((
                                    gpui::ElementId::from("quick-commands-manager-cat-drag"),
                                    category.id.clone(),
                                ))
                                .size(px(16.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(self.tokens.radii.sm))
                                .cursor(gpui::CursorStyle::OpenHand)
                                .child(Self::render_lucide_icon(
                                    LucideIcon::Grip,
                                    14.0,
                                    rgb(theme.text_muted),
                                ))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation()
                                })
                                .on_drag(drag, |drag, position, _, cx| {
                                    let mut preview = drag.clone();
                                    preview.position = position;
                                    cx.new(|_| preview)
                                }),
                        )
                        .child(Self::render_lucide_icon(
                            quick_command_lucide_icon(category.icon),
                            13.0,
                            if is_active {
                                rgb(theme.accent)
                            } else {
                                rgb(theme.text_muted)
                            },
                        ))
                        .child(
                            div()
                                .flex_1()
                                .truncate()
                                .text_size(px(self.tokens.metrics.ui_text_sm))
                                .font_weight(if is_active {
                                    gpui::FontWeight::MEDIUM
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .text_color(if is_active {
                                    rgb(theme.accent)
                                } else {
                                    rgb(theme.text)
                                })
                                .child(category.name.clone()),
                        )
                        .on_mouse_down(MouseButton::Left, cx.listener({
                            let cat = cat_for_edit.clone();
                            move |this, _event, _window, cx| {
                                this.terminal.update(cx, |terminal, _cx| {
                                    terminal.quick_commands.select_category(&cat.id);
                                    terminal.quick_commands.start_category_edit(cat.clone());
                                });
                                cx.stop_propagation();
                                cx.notify();
                            }
                        })),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .flex_none()
                                .h(px(16.0))
                                .min_w(px(16.0))
                                .px(px(5.0))
                                .rounded_full()
                                .bg(rgba((theme.border << 8) | 0x40))
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(if is_active {
                                    rgb(theme.accent)
                                } else {
                                    rgb(theme.text_muted)
                                })
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(count.to_string()),
                        )
                        .when(can_delete, |actions| {
                            actions.child(self.quick_command_action_button(
                                LucideIcon::Trash2,
                                move |this, _event, _window, cx| {
                                    this.delete_manager_category(&cat_id_for_delete, cx);
                                    cx.stop_propagation();
                                    cx.notify();
                                },
                                cx,
                            ))
                        }),
                );

            list = list.child(row);
        }

        div()
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scrollbar()
            .child(list)
            .into_any_element()
    }

    fn render_quick_command_manager_inspector(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let content = if snapshot.manager_mode == QuickCommandManagerMode::Group {
            if snapshot.category_editor.is_some() {
                self.render_quick_command_manager_group_editor(snapshot, cx)
            } else {
                self.render_quick_command_manager_empty()
            }
        } else if snapshot.command_editor.is_some() {
            self.render_quick_command_editor(snapshot, cx)
        } else {
            self.render_quick_command_manager_empty()
        };
        div()
            .flex_1()
            .h_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .bg(rgba((theme.bg << 8) | 0x40))
            .child(content)
            .into_any_element()
    }

    fn render_quick_command_manager_empty(&self) -> AnyElement {
        let theme = self.tokens.ui;
        div()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .text_color(rgb(theme.text_muted))
            .child(Self::render_lucide_icon(
                LucideIcon::Pencil,
                22.0,
                rgb(theme.text_muted),
            ))
            .child(
                div()
                    .max_w(px(280.0))
                    .text_center()
                    .text_size(px(self.tokens.metrics.ui_text_sm))
                    .child(self.i18n.t("terminal.quick_commands.manager_empty")),
            )
            .into_any_element()
    }

    fn render_quick_command_manager_group_editor(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let draft = snapshot.category_editor.clone();
        let name = draft
            .as_ref()
            .map(|draft| draft.name.clone())
            .unwrap_or_default();
        let selected_icon = draft
            .as_ref()
            .map(|draft| draft.icon)
            .unwrap_or(QuickCommandIcon::Zap);

        let icons = [
            QuickCommandIcon::Docker,
            QuickCommandIcon::Zap,
            QuickCommandIcon::Cpu,
            QuickCommandIcon::Cloud,
            QuickCommandIcon::Git,
            QuickCommandIcon::Shield,
            QuickCommandIcon::Package,
            QuickCommandIcon::Rocket,
            QuickCommandIcon::Settings,
            QuickCommandIcon::Network,
            QuickCommandIcon::SunDim,
            QuickCommandIcon::Cat,
            QuickCommandIcon::FaceSlightlySmilingPlus,
            QuickCommandIcon::IceCreamCone,
            QuickCommandIcon::CloudHail,
            QuickCommandIcon::Radar,
            QuickCommandIcon::Leaf,
            QuickCommandIcon::Apple,
            QuickCommandIcon::Webhook,
            QuickCommandIcon::CirclePile,
            QuickCommandIcon::Palette,
            QuickCommandIcon::Sprout,
            QuickCommandIcon::PiggyBank,
            QuickCommandIcon::CloudSun,
            QuickCommandIcon::MoonStar,
            QuickCommandIcon::Snowflake,
            QuickCommandIcon::PawPrint,
            QuickCommandIcon::LoaderPinwheel,
        ];

        let mut grid = div().w(px(496.0)).flex().flex_wrap().gap(px(8.0));
        for icon in icons {
            let is_selected = selected_icon == icon;
            grid = grid.child(
                div()
                    .size(px(48.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(self.tokens.radii.md))
                    .border_1()
                    .border_color(if is_selected {
                        rgb(theme.accent)
                    } else {
                        rgba((theme.border << 8) | 0x40)
                    })
                    .bg(if is_selected {
                        rgba((theme.accent << 8) | 0x22)
                    } else {
                        rgba(0x00000000)
                    })
                    .hover(|style| {
                        style
                            .bg(rgba((theme.bg_hover << 8) | 0xb3))
                            .border_color(rgb(theme.accent))
                    })
                    .cursor_pointer()
                    .child(Self::render_lucide_icon(
                        quick_command_lucide_icon(icon),
                        24.0,
                        if is_selected {
                            rgb(theme.accent)
                        } else {
                            rgb(theme.text_muted)
                        },
                    ))
                    .on_mouse_down(MouseButton::Left, cx.listener(
                        move |this, _event, _window, cx| {
                            this.set_quick_command_category_editor_icon(icon, cx);
                            cx.stop_propagation();
                        },
                    )),
            );
        }

        div()
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scrollbar()
                    .p(px(16.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .child(
                        div()
                            .rounded(px(self.tokens.radii.md))
                            .border_1()
                            .border_color(rgba((theme.border << 8) | 0x66))
                            .bg(rgba((theme.bg << 8) | 0x33))
                            .p(px(16.0))
                            .flex()
                            .flex_col()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .text_size(px(self.tokens.metrics.ui_text_sm))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(theme.text))
                                    .child(self.i18n.t("terminal.quick_commands.group_name_placeholder")),
                            )
                            .child(self.render_quick_command_text_input(
                                QuickCommandInput::CategoryName,
                                name,
                                snapshot.focused_input,
                                String::new(),
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .rounded(px(self.tokens.radii.md))
                            .border_1()
                            .border_color(rgba((theme.border << 8) | 0x66))
                            .bg(rgba((theme.bg << 8) | 0x33))
                            .p(px(16.0))
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .text_size(px(self.tokens.metrics.ui_text_sm))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(theme.text))
                                    .child(self.i18n.t("terminal.quick_commands.category_icon")),
                            )
                            .child(grid),
                    ),
            )
            .into_any_element()
    }

    fn render_quick_command_rows(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        if snapshot.visible_commands.is_empty() {
            return div()
                .flex_1()
                .min_h(px(180.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .text_color(rgb(theme.text_muted))
                .child(Self::render_lucide_icon(
                    LucideIcon::Send,
                    20.0,
                    rgb(theme.text_muted),
                ))
                .child(self.render_display_text_with_role(
                    SelectableTextRole::PlainDocument,
                    "quick-commands-empty",
                    snapshot.query.as_str(),
                    if snapshot.query.trim().is_empty() {
                        self.i18n.t("terminal.quick_commands.empty_category")
                    } else {
                        self.i18n.t("terminal.quick_commands.empty_search")
                    },
                    theme.text_muted,
                    cx,
                ))
                .into_any_element();
        }

        let mut list = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(6.0))
            .pt(px(6.0))
            .pb(px(6.0));
        for command in snapshot.visible_commands.iter().cloned() {
            list = list.child(self.render_quick_command_row(
                command,
                snapshot.managing,
                snapshot.highlighted_command.as_deref(),
                cx,
            ));
        }
        div()
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scrollbar()
            .child(list)
            .into_any_element()
    }

    fn render_quick_command_row(
        &self,
        command: QuickCommand,
        managing: bool,
        highlighted_command: Option<&str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let risk = quick_command_risk_badge(&command.command);
        let command_for_insert = command.command.clone();
        let command_for_run = command.clone();
        let command_for_primary_action = command.clone();
        let command_id_for_delete = command.id.clone();
        let command_id_for_hover = command.id.clone();
        let command_id_for_drop = command.id.clone();
        let drag = QuickCommandDrag {
            id: command.id.clone(),
            name: command.name.clone(),
            position: gpui::Point::default(),
            tokens: self.tokens,
        };
        let description_for_tooltip = command.description.clone();
        let tooltip_id = format!("quick-command-description-{}", command.id);
        let tooltip_id_for_hover = tooltip_id.clone();
        let keep_open_for_insert = true;
        let is_selected_in_manager = managing && {
            let terminal = self.terminal.read(cx);
            terminal
                .quick_commands
                .store
                .command_editor
                .as_ref()
                .and_then(|draft| draft.id.as_deref())
                == Some(command.id.as_str())
        };
        let highlighted = is_selected_in_manager
            || highlighted_command == Some(command.id.as_str());
        let selection_group_id = crate::workspace::selectable_text::selectable_text_id(
            "quick-command-row",
            command.id.as_str(),
        );

        if managing {
            let row = div()
                .id(("quick-command-row", selection_group_id))
                .h(px(34.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_between()
                .rounded(px(self.tokens.radii.sm))
                .border_1()
                .border_color(if is_selected_in_manager {
                    rgba((theme.accent << 8) | 0x66)
                } else {
                    rgba(0x00000000)
                })
                .bg(if is_selected_in_manager {
                    rgba((theme.accent << 8) | 0x22)
                } else if highlighted {
                    rgba((theme.bg_hover << 8) | 0xb3)
                } else {
                    rgba(0x00000000)
                })
                .hover(move |style| {
                    style
                        .bg(rgba((theme.bg_hover << 8) | 0xb3))
                        .text_color(rgb(theme.text))
                })
                .on_mouse_move(
                    cx.listener(move |this, event: &gpui::MouseMoveEvent, _window, cx| {
                        if this.terminal.update(cx, |terminal, _cx| {
                            terminal
                                .quick_commands
                                .set_highlighted_command(command_id_for_hover.clone())
                        }) {
                            cx.notify();
                        }
                        if let Some(description) = description_for_tooltip.clone() {
                            this.queue_workspace_tooltip(
                                tooltip_id.clone(),
                                description,
                                f32::from(event.position.x) + 12.0,
                                f32::from(event.position.y) + 16.0,
                                cx,
                            );
                        }
                    }),
                )
                .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                    if !*hovered {
                        this.clear_workspace_tooltip(&tooltip_id_for_hover, cx);
                    }
                }))
                .drag_over::<QuickCommandDrag>({
                    let target = command_id_for_drop.clone();
                    move |row, drag, _, _| {
                        if drag.id != target {
                            row.bg(rgba((theme.accent << 8) | 0x33))
                        } else {
                            row
                        }
                    }
                })
                .on_drop(cx.listener({
                    let target = command_id_for_drop.clone();
                    move |this, drag: &QuickCommandDrag, _, cx| {
                        this.move_quick_command_to(&drag.id, &target, cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .id((
                                    gpui::ElementId::from("quick-commands-manager-cmd-drag"),
                                    command.id.clone(),
                                ))
                                .size(px(16.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(self.tokens.radii.sm))
                                .cursor(gpui::CursorStyle::OpenHand)
                                .child(Self::render_lucide_icon(
                                    LucideIcon::Grip,
                                    14.0,
                                    rgb(theme.text_muted),
                                ))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_drag(drag, |drag, position, _, cx| {
                                    let mut preview = drag.clone();
                                    preview.position = position;
                                    cx.new(|_| preview)
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .truncate()
                                .text_size(px(self.tokens.metrics.ui_text_sm))
                                .font_weight(if is_selected_in_manager {
                                    gpui::FontWeight::MEDIUM
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .text_color(if is_selected_in_manager {
                                    rgb(theme.accent)
                                } else {
                                    rgb(theme.text)
                                })
                                .child(self.render_row_safe_selectable_display_text_in_group(
                                    selection_group_id,
                                    "quick-command-row-cell",
                                    ("name", command.id.as_str()),
                                    0,
                                    command.name.clone(),
                                    if is_selected_in_manager { theme.accent } else { theme.text },
                                    None,
                                    cx,
                                )),
                        )
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _event, window, cx| {
                                this.start_quick_command_edit(
                                    command_for_primary_action.clone(),
                                    cx,
                                );
                                window.focus(&this.focus_handle, cx);
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        ),
                )
                .child(self.quick_command_action_button(
                    LucideIcon::Trash2,
                    move |this, _event, _window, cx| {
                        this.terminal.update(cx, |terminal, _cx| {
                            terminal.quick_commands.delete_command(&command_id_for_delete)
                        });
                        cx.stop_propagation();
                        cx.notify();
                    },
                    cx,
                ));
            return row.into_any_element();
        }

        div()
            .id(("quick-command-row", selection_group_id))
            .rounded(px(self.tokens.radii.md))
            .px(px(8.0))
            .py(px(8.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_color(rgb(theme.text_muted))
            .bg(if is_selected_in_manager {
                rgba((theme.accent << 8) | 0x26)
            } else if highlighted {
                rgba((theme.bg_hover << 8) | 0xb3)
            } else {
                rgba(0x00000000)
            })
            .when(is_selected_in_manager, |row| {
                row.border_1().border_color(rgba((theme.accent << 8) | 0x66))
            })
            .when(!is_selected_in_manager, |row| {
                row.border_1().border_color(rgba(0x00000000))
            })
            .hover(move |style| {
                style
                    .bg(rgba((theme.bg_hover << 8) | 0xb3))
                    .text_color(rgb(theme.text))
            })
            .on_mouse_move(
                cx.listener(move |this, event: &gpui::MouseMoveEvent, _window, cx| {
                    // Mouse hover and ArrowUp/ArrowDown share the same active
                    // row state, matching browser menu focus without changing
                    // row-safe selectable click bubbling.
                    if this.terminal.update(cx, |terminal, _cx| {
                        terminal
                            .quick_commands
                            .set_highlighted_command(command_id_for_hover.clone())
                    }) {
                        cx.notify();
                    }
                    if let Some(description) = description_for_tooltip.clone() {
                        this.queue_workspace_tooltip(
                            tooltip_id.clone(),
                            description,
                            f32::from(event.position.x) + 12.0,
                            f32::from(event.position.y) + 16.0,
                            cx,
                        );
                    }
                }),
            )
            .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                if !*hovered {
                    this.clear_workspace_tooltip(&tooltip_id_for_hover, cx);
                }
            }))
            .drag_over::<QuickCommandDrag>({
                let target = command_id_for_drop.clone();
                move |row, drag, _, _| {
                    if drag.id != target {
                        row.bg(rgba((theme.accent << 8) | 0x33))
                    } else {
                        row
                    }
                }
            })
            .on_drop(cx.listener({
                let target = command_id_for_drop.clone();
                move |this, drag: &QuickCommandDrag, _, cx| {
                    this.move_quick_command_to(&drag.id, &target, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .id((
                        gpui::ElementId::from("quick-commands-manager-cmd-drag"),
                        command.id.clone(),
                    ))
                    .flex_none()
                    .size(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(self.tokens.radii.sm))
                    .cursor(gpui::CursorStyle::OpenHand)
                    .child(Self::render_lucide_icon(
                        LucideIcon::Grip,
                        14.0,
                        rgb(theme.text_muted),
                    ))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_drag(drag, |drag, position, _, cx| {
                        let mut preview = drag.clone();
                        preview.position = position;
                        cx.new(|_| preview)
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, window, cx| {
                            if managing {
                                this.start_quick_command_edit(
                                    command_for_primary_action.clone(),
                                    cx,
                                );
                            } else {
                                this.insert_quick_command_into_command_bar(
                                    command_for_insert.clone(),
                                    keep_open_for_insert,
                                    cx,
                                );
                            }
                            if managing {
                                window.focus(&this.focus_handle, cx);
                            } else {
                                let document_id =
                                    this.terminal_command_composer.read(cx).active_document_id();
                                this.focus_terminal_command_composer_input(document_id, window, cx);
                            }
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(self.tokens.metrics.ui_text_sm))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(theme.text))
                                    .child(self.render_row_safe_selectable_display_text_in_group(
                                        selection_group_id,
                                        "quick-command-row-cell",
                                        ("name", command.id.as_str()),
                                        0,
                                        command.name.clone(),
                                        theme.text,
                                        None,
                                        cx,
                                    )),
                            )
                            .when_some(risk, |row, risk| {
                                row.child(
                                    status_pill(
                                        &self.tokens,
                                        quick_command_risk_badge_label(&self.i18n, risk)
                                            .to_uppercase(),
                                        StatusPillOptions::new(quick_command_risk_badge_tone(risk))
                                            .compact()
                                            .strong(),
                                    ),
                                )
                            })
                            .when_some(
                                (!command.availability.host_patterns.is_empty())
                                    .then(|| format!("{}: {}", self.i18n.t(match command.availability.match_field {
                                        QuickCommandMatchField::Host => "terminal.quick_commands.match_host",
                                        QuickCommandMatchField::ConnectionName => "terminal.quick_commands.match_connection",
                                        QuickCommandMatchField::All => "terminal.quick_commands.match_all",
                                    }), command.availability.host_patterns.join(", "))),
                                |row, pattern| {
                                row.child(
                                    status_pill(
                                        &self.tokens,
                                        pattern,
                                        StatusPillOptions::new(StatusTone::Neutral).compact(),
                                    )
                                    .font_family(settings_mono_font_family(
                                        self.settings_store.settings(),
                                    )),
                                )
                            }),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(self.tokens.metrics.ui_text_xs))
                            .font_family(settings_mono_font_family(
                                self.settings_store.settings(),
                            ))
                            .text_color(rgba((theme.accent << 8) | 0xd9))
                            .child(self.render_row_safe_selectable_display_text_in_group_with_alpha(
                                selection_group_id,
                                "quick-command-row-cell",
                                ("command", command.id.as_str()),
                                1,
                                command.command.clone(),
                                theme.accent,
                                0xd9 as f32 / 255.0,
                                None,
                                cx,
                            )),
                    )
            )
            .when(!managing, |row| {
                row.child(self.quick_command_action_button(
                    LucideIcon::Play,
                    move |this, _event, window, cx| {
                        this.run_quick_command_model(&command_for_run, window, cx);
                        cx.stop_propagation();
                    },
                    cx,
                ))
            })
            .when(managing, |row| {
                row.child(self.quick_command_action_button(
                    LucideIcon::Trash2,
                    move |this, _event, _window, cx| {
                        this.terminal.update(cx, |terminal, _cx| {
                            terminal.quick_commands.delete_command(&command_id_for_delete)
                        });
                        cx.stop_propagation();
                        cx.notify();
                    },
                    cx,
                ))
            })
            .into_any_element()
    }

    fn render_quick_command_editor(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let Some(draft) = snapshot.command_editor.as_ref() else {
            return div().into_any_element();
        };
        let can_save = quick_command_editor_can_save(draft);

        let mut protocols = div().flex().items_center().gap(px(4.0)).flex_wrap();
        for (protocol, label_key) in [
            (
                QuickCommandTargetProtocol::Local,
                "terminal.quick_commands.protocol_local",
            ),
            (
                QuickCommandTargetProtocol::Ssh,
                "terminal.quick_commands.protocol_ssh",
            ),
            (
                QuickCommandTargetProtocol::Mosh,
                "terminal.quick_commands.protocol_mosh",
            ),
            (
                QuickCommandTargetProtocol::Telnet,
                "terminal.quick_commands.protocol_telnet",
            ),
            (
                QuickCommandTargetProtocol::Serial,
                "terminal.quick_commands.protocol_serial",
            ),
            (
                QuickCommandTargetProtocol::Tmux,
                "terminal.quick_commands.protocol_tmux",
            ),
        ] {
            let active = draft.protocols.contains(&protocol);
            protocols = protocols.child(
                self.quick_command_text_button(
                    self.i18n.t(label_key),
                    true,
                    cx.listener(move |this, _event, _window, cx| {
                        this.terminal.update(cx, |terminal, _cx| {
                            terminal.quick_commands.toggle_command_protocol(protocol)
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .border_color(if active {
                    rgb(theme.accent)
                } else {
                    rgba((theme.border << 8) | 0x99)
                })
                .bg(if active {
                    rgba((theme.accent << 8) | 0x1a)
                } else {
                    rgba(0x00000000)
                }),
            );
        }
        let mut match_fields = div().flex().items_center().gap(px(4.0)).flex_wrap();
        for (field, label) in [
            (
                QuickCommandMatchField::Host,
                "terminal.quick_commands.match_host",
            ),
            (
                QuickCommandMatchField::ConnectionName,
                "terminal.quick_commands.match_connection",
            ),
            (
                QuickCommandMatchField::All,
                "terminal.quick_commands.match_all",
            ),
        ] {
            let active = draft.match_field == field;
            match_fields = match_fields.child(
                self.quick_command_text_button(
                    self.i18n.t(label),
                    true,
                    cx.listener(move |this, _, _, cx| {
                        this.terminal.update(cx, |terminal, _| {
                            if let Some(draft) =
                                terminal.quick_commands.store.command_editor.as_mut()
                            {
                                draft.match_field = field;
                            }
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .border_color(if active {
                    rgb(theme.accent)
                } else {
                    rgba((theme.border << 8) | 0x99)
                })
                .bg(if active {
                    rgba((theme.accent << 8) | 0x1a)
                } else {
                    rgba(0x00000000)
                }),
            );
        }
        let parameters = self.render_quick_command_parameter_editor(draft, snapshot, cx);
        let scroll = self.terminal.read(cx).quick_commands.editor_scroll.clone();
        let padding = if snapshot.managing { 16.0 } else { 8.0 };

        let form = div()
            .w_full()
            .min_w_0()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(padding))
            .when(snapshot.managing, |editor| {
                editor.child(
                    div()
                        .mb(px(4.0))
                        .text_size(px(self.tokens.metrics.ui_text_sm))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(theme.text))
                        .child(self.i18n.t(if draft.id.is_some() {
                            "terminal.quick_commands.edit_command"
                        } else {
                            "terminal.quick_commands.add_command"
                        })),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(section_header(
                        &self.tokens,
                        self.i18n.t("terminal.quick_commands.section_basic"),
                        SectionHeaderOptions::new().compact(),
                        None,
                        None,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .child(self.render_quick_command_text_input(
                                        QuickCommandInput::CommandName,
                                        draft.name.clone(),
                                        snapshot.focused_input,
                                        self.i18n.t("terminal.quick_commands.name_placeholder"),
                                        cx,
                                    )),
                            )
                            .child(self.render_quick_command_editor_category_selector(
                                draft, snapshot, cx,
                            )),
                    )
                    .child(self.render_quick_command_text_input(
                        QuickCommandInput::CommandText,
                        draft.command.clone(),
                        snapshot.focused_input,
                        self.i18n.t("terminal.quick_commands.command_placeholder"),
                        cx,
                    ))
                    .child(self.render_quick_command_syntax_header(draft.syntax_help_expanded, cx))
                    .when(draft.syntax_help_expanded, |section| {
                        section.child(self.render_quick_command_syntax_help(cx))
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(self.tokens.metrics.ui_text_xs))
                                    .text_color(rgb(theme.text_muted))
                                    .child(self.i18n.t("terminal.quick_commands.line_delay_label")),
                            )
                            .child(
                                div()
                                    .w(px(100.0))
                                    .child(self.render_quick_command_text_input(
                                        QuickCommandInput::CommandLineDelay,
                                        draft.line_delay_ms.clone(),
                                        snapshot.focused_input,
                                        "1000".to_string(),
                                        cx,
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(section_header(
                        &self.tokens,
                        self.i18n.t("terminal.quick_commands.parameters"),
                        SectionHeaderOptions::new().compact(),
                        None,
                        Some(
                            self.quick_command_text_button(
                                self.i18n.t("terminal.quick_commands.add_parameter"),
                                true,
                                cx.listener(|this, _event, _window, cx| {
                                    this.terminal.update(cx, |terminal, _cx| {
                                        terminal.quick_commands.add_command_parameter()
                                    });
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .into_any_element(),
                        ),
                    ))
                    .child(parameters),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(section_header(
                        &self.tokens,
                        self.i18n.t("terminal.quick_commands.section_availability"),
                        SectionHeaderOptions::new().compact(),
                        None,
                        None,
                    ))
                    .child(
                        div()
                            .rounded(px(self.tokens.radii.md))
                            .border_1()
                            .border_color(rgba((theme.border << 8) | 0x4d))
                            .bg(rgba((theme.bg << 8) | 0x33))
                            .p(px(10.0))
                            .flex()
                            .flex_col()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(self.tokens.metrics.ui_text_xs))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(theme.text_muted))
                                            .child(self.i18n.t("terminal.quick_commands.match_field")),
                                    )
                                    .child(match_fields),
                            )
                            .child(
                                div()
                                    .h(px(1.0))
                                    .w_full()
                                    .bg(rgba((theme.border << 8) | 0x33)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(self.tokens.metrics.ui_text_xs))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(theme.text_muted))
                                            .child(self.i18n.t("terminal.quick_commands.match_rules")),
                                    )
                                    .child(
                                        self.render_quick_command_text_input(
                                            QuickCommandInput::CommandHostPattern,
                                            draft.host_patterns.clone(),
                                            snapshot.focused_input,
                                            self.i18n.t("terminal.quick_commands.host_pattern_placeholder"),
                                            cx,
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .rounded(px(self.tokens.radii.md))
                            .border_1()
                            .border_color(rgba((theme.border << 8) | 0x4d))
                            .bg(rgba((theme.bg << 8) | 0x33))
                            .p(px(10.0))
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(self.tokens.metrics.ui_text_xs))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(theme.text_muted))
                                    .child(self.i18n.t("terminal.quick_commands.protocols")),
                            )
                            .child(protocols),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(section_header(
                        &self.tokens,
                        self.i18n.t("terminal.quick_commands.description_placeholder"),
                        SectionHeaderOptions::new().compact(),
                        None,
                        None,
                    ))
                    .child(self.render_quick_command_text_input(
                        QuickCommandInput::CommandDescription,
                        draft.description.clone(),
                        snapshot.focused_input,
                        self.i18n.t("terminal.quick_commands.description_placeholder"),
                        cx,
                    )),
            );

        quick_command_editor_frame(
            form.into_any_element(),
            div()
                .p(px(padding))
                .child(self.render_quick_editor_buttons(
                    can_save,
                    "terminal.quick_commands.save",
                    |this, cx| this.save_quick_command_editor(cx),
                    cx,
                ))
                .into_any_element(),
            &scroll,
            &self.tokens,
        )
        .when(!snapshot.managing, |editor| editor.max_h(px(420.0)))
        .into_any_element()
    }

    fn render_quick_command_editor_category_selector(
        &self,
        draft: &QuickCommandEditorDraft,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let is_open = snapshot.editor_category_menu_open;
        let current_category = snapshot
            .categories
            .iter()
            .find(|c| c.id == draft.category)
            .or_else(|| snapshot.categories.first());
        let current_cat_name = current_category
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "All".to_string());
        let current_cat_icon = current_category.map(|c| c.icon).unwrap_or(QuickCommandIcon::Zap);

        let button = div()
            .id("quick-command-editor-category-btn")
            .h(px(28.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(if is_open {
                rgb(theme.accent)
            } else {
                rgba((theme.border << 8) | 0x80)
            })
            .bg(if is_open {
                rgba((theme.accent << 8) | 0x22)
            } else {
                rgba((theme.bg << 8) | 0x66)
            })
            .hover(|style| {
                style
                    .bg(rgba((theme.bg_hover << 8) | 0xcc))
                    .border_color(rgb(theme.accent))
            })
            .cursor_pointer()
            .child(Self::render_lucide_icon(
                quick_command_lucide_icon(current_cat_icon),
                13.0,
                rgb(theme.accent),
            ))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(theme.text))
                    .child(current_cat_name),
            )
            .child(Self::render_lucide_icon(
                LucideIcon::ChevronDown,
                10.0,
                rgb(theme.text_muted),
            ))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.toggle_quick_command_editor_category_menu(cx);
                    cx.stop_propagation();
                }),
            );

        select_anchor_probe(
            SelectAnchorId::QuickCommandEditorCategory,
            button,
            {
                let workspace = cx.entity();
                move |anchor, _window, cx| {
                    let _ = workspace.update(cx, |this, _cx| {
                        this.select_anchors.insert(anchor.id, anchor);
                    });
                }
            },
        )
        .into_any_element()
    }

    fn render_quick_command_editor_category_menu(
        &self,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let theme = self.tokens.ui;
        let draft = snapshot.command_editor.as_ref()?;
        let anchor = self
            .select_anchors
            .get(&SelectAnchorId::QuickCommandEditorCategory)?;

        let mut list = div().flex().flex_col().gap(px(2.0));
        for category in &snapshot.categories {
            let cat_id = category.id.clone();
            let is_selected = cat_id == draft.category;
            list = list.child(
                div()
                    .h(px(26.0))
                    .px(px(8.0))
                    .rounded(px(self.tokens.radii.sm))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .bg(if is_selected {
                        rgba((theme.accent << 8) | 0x22)
                    } else {
                        rgba(0x00000000)
                    })
                    .hover(|s| s.bg(rgba((theme.bg_hover << 8) | 0xb3)))
                    .child(Self::render_lucide_icon(
                        quick_command_lucide_icon(category.icon),
                        12.0,
                        if is_selected {
                            rgb(theme.accent)
                        } else {
                            rgb(theme.text_muted)
                        },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_size(px(self.tokens.metrics.ui_text_xs))
                            .font_weight(if is_selected {
                                gpui::FontWeight::MEDIUM
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(if is_selected {
                                rgb(theme.accent)
                            } else {
                                rgb(theme.text)
                            })
                            .child(category.name.clone()),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, _window, cx| {
                            this.terminal.update(cx, |terminal, _cx| {
                                terminal.quick_commands.set_command_category(cat_id.clone());
                                terminal.quick_commands.editor_category_menu_open = false;
                            });
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    ),
            );
        }

        let popup = div()
            .id("quick-command-editor-category-popup")
            .w(px(160.0))
            .max_h(px(200.0))
            .overflow_y_scroll()
            .p(px(4.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x99))
            .bg(rgba((theme.bg_elevated << 8) | 0xfc))
            .shadow(oxideterm_gpui_ui::theme_overlay_shadow(&self.tokens))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(list);

        Some(
            popover_backdrop()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.terminal.update(cx, |t, _| {
                            t.quick_commands.editor_category_menu_open = false;
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _, _, cx| {
                        this.terminal.update(cx, |t, _| {
                            t.quick_commands.editor_category_menu_open = false;
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
                .child(
                    deferred(
                        anchored()
                            .anchor(Corner::TopRight)
                            .position(anchor.bounds.bottom_right())
                            .offset(point(px(0.0), px(4.0)))
                            .position_mode(AnchoredPositionMode::Window)
                            .child(popup),
                    )
                    .with_priority(oxideterm_gpui_ui::modal::TAURI_SELECT_LAYER_PRIORITY),
                )
                .into_any_element(),
        )
    }

    fn render_quick_command_parameter_editor(
        &self,
        draft: &QuickCommandEditorDraft,
        snapshot: &QuickCommandsRenderSnapshot,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        if draft.parameters.is_empty() {
            return div()
                .p(px(10.0))
                .rounded(px(self.tokens.radii.md))
                .border_1()
                .border_color(rgba((theme.border << 8) | 0x40))
                .bg(rgba((theme.bg << 8) | 0x22))
                .text_size(px(self.tokens.metrics.ui_text_xs))
                .text_color(rgb(theme.text_muted))
                .child(self.i18n.t("terminal.quick_commands.no_parameters"))
                .into_any_element();
        }

        let mut rows = div().flex().flex_col().gap(px(8.0));
        for (index, parameter) in draft.parameters.iter().enumerate() {
            let kind = parameter.kind;
            let required = parameter.required;

            // Persistent segmented tabs: [ 文本 | 选项 | 敏感值 ]
            let mut type_tabs = div()
                .flex()
                .items_center()
                .p(px(2.0))
                .rounded(px(self.tokens.radii.sm))
                .bg(rgba((theme.bg << 8) | 0x80))
                .border_1()
                .border_color(rgba((theme.border << 8) | 0x66));

            for target_kind in [
                QuickCommandParameterKind::Text,
                QuickCommandParameterKind::Choice,
                QuickCommandParameterKind::Secret,
            ] {
                let is_selected = kind == target_kind;
                let label = match target_kind {
                    QuickCommandParameterKind::Text => {
                        self.i18n.t("terminal.quick_commands.parameter_text")
                    }
                    QuickCommandParameterKind::Choice => {
                        self.i18n.t("terminal.quick_commands.parameter_choice")
                    }
                    QuickCommandParameterKind::Secret => {
                        self.i18n.t("terminal.quick_commands.parameter_secret")
                    }
                };
                type_tabs = type_tabs.child(
                    div()
                        .h(px(22.0))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(self.tokens.radii.xs))
                        .cursor_pointer()
                        .text_size(px(11.0))
                        .font_weight(if is_selected {
                            gpui::FontWeight::MEDIUM
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .text_color(if is_selected {
                            rgb(theme.accent)
                        } else {
                            rgb(theme.text_muted)
                        })
                        .bg(if is_selected {
                            rgba((theme.accent << 8) | 0x26)
                        } else {
                            rgba(0x00000000)
                        })
                        .hover(move |s| {
                            if is_selected {
                                s
                            } else {
                                s.bg(rgba((theme.bg_hover << 8) | 0x80))
                                    .text_color(rgb(theme.text))
                            }
                        })
                        .child(label)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _event, _window, cx| {
                                this.terminal.update(cx, |terminal, _cx| {
                                    terminal
                                        .quick_commands
                                        .set_command_parameter_kind(index, target_kind)
                                });
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        ),
                );
            }

            // Top Header: Tabs on left, Required toggle and Delete on right
            let top_bar = div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(type_tabs)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            self.quick_command_text_button(
                                self.i18n.t(if required {
                                    "terminal.quick_commands.parameter_required"
                                } else {
                                    "terminal.quick_commands.parameter_optional"
                                }),
                                true,
                                cx.listener(move |this, _event, _window, cx| {
                                    this.terminal.update(cx, |terminal, _cx| {
                                        terminal
                                            .quick_commands
                                            .toggle_command_parameter_required(index)
                                    });
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .border_color(if required {
                                rgb(theme.accent)
                            } else {
                                rgba((theme.border << 8) | 0x99)
                            })
                            .text_color(if required {
                                rgb(theme.accent)
                            } else {
                                rgb(theme.text_muted)
                            })
                            .bg(if required {
                                rgba((theme.accent << 8) | 0x1f)
                            } else {
                                rgba(0x00000000)
                            }),
                        )
                        .child(self.quick_command_icon_button(
                            LucideIcon::Trash2,
                            move |this, _event, _window, cx| {
                                this.terminal.update(cx, |terminal, _cx| {
                                    terminal.quick_commands.remove_command_parameter(index)
                                });
                                cx.stop_propagation();
                                cx.notify();
                            },
                            cx,
                        )),
                );

            rows = rows.child(
                div()
                    .rounded(px(self.tokens.radii.md))
                    .border_1()
                    .border_color(rgba((theme.border << 8) | 0x66))
                    .bg(rgba((theme.bg << 8) | 0x33))
                    .p(px(8.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(top_bar)
                    .child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(div().flex_1().child(self.render_quick_command_text_input(
                                QuickCommandInput::ParameterName(index),
                                parameter.name.clone(),
                                snapshot.focused_input,
                                self.i18n.t("terminal.quick_commands.parameter_name"),
                                cx,
                            )))
                            .child(div().flex_1().child(self.render_quick_command_text_input(
                                QuickCommandInput::ParameterLabel(index),
                                parameter.label.clone(),
                                snapshot.focused_input,
                                self.i18n.t("terminal.quick_commands.parameter_label"),
                                cx,
                            ))),
                    )
                    .when(kind == QuickCommandParameterKind::Choice, |row| {
                        row.child(self.render_quick_command_text_input(
                            QuickCommandInput::ParameterChoices(index),
                            parameter.choices.clone(),
                            snapshot.focused_input,
                            self.i18n.t("terminal.quick_commands.parameter_choices"),
                            cx,
                        ))
                    })
                    .when(kind != QuickCommandParameterKind::Secret, |row| {
                        row.child(self.render_quick_command_text_input(
                            QuickCommandInput::ParameterDefault(index),
                            parameter.default_value.clone(),
                            snapshot.focused_input,
                            self.i18n.t("terminal.quick_commands.parameter_default"),
                            cx,
                        ))
                    }),
            );
        }
        rows.into_any_element()
    }

    fn render_quick_command_syntax_header(
        &self,
        expanded: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let theme = self.tokens.ui;
        div()
            .h(px(28.0))
            .w_full()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(4.0))
            .rounded(px(self.tokens.radii.sm))
            .cursor_pointer()
            .hover(|row| row.bg(rgba((theme.bg_hover << 8) | 0xb3)))
            .child(self.render_animated_chevron(
                "quick-command-syntax-chevron",
                expanded,
                13.0,
                rgb(theme.text_muted),
            ))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(theme.text_muted))
                    .child(self.i18n.t("terminal.quick_commands.syntax_help_title")),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.terminal.update(cx, |terminal, _| {
                        if let Some(draft) = terminal.quick_commands.store.command_editor.as_mut() {
                            draft.syntax_help_expanded = !draft.syntax_help_expanded;
                        }
                    });
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
    }

    fn render_quick_command_syntax_help(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.tokens.ui;
        let mut panel = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .p(px(10.0))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgba((theme.border << 8) | 0x99))
            .bg(rgba((theme.bg << 8) | 0x40))
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .text_color(rgb(theme.text_muted))
                    .child(self.i18n.t("terminal.quick_commands.syntax_help_intro")),
            )
            .child(self.render_quick_command_syntax_group_title(
                "terminal.quick_commands.syntax_help_custom",
            ))
            .child(self.render_quick_command_syntax_row(
                "{{param.name}}",
                "terminal.quick_commands.syntax_help_custom_desc",
                true,
                cx,
            ))
            .child(self.render_quick_command_syntax_group_title(
                "terminal.quick_commands.syntax_help_context",
            ));

        for (token, description_key) in [
            ("{{ctx.host}}", "terminal.quick_commands.syntax_help_host"),
            (
                "{{ctx.username}}",
                "terminal.quick_commands.syntax_help_username",
            ),
            ("{{ctx.port}}", "terminal.quick_commands.syntax_help_port"),
            ("{{ctx.cwd}}", "terminal.quick_commands.syntax_help_cwd"),
            (
                "{{ctx.connection}}",
                "terminal.quick_commands.syntax_help_connection",
            ),
            ("{{ctx.group}}", "terminal.quick_commands.syntax_help_group"),
            (
                "{{ctx.selection}}",
                "terminal.quick_commands.syntax_help_selection",
            ),
        ] {
            panel = panel.child(self.render_quick_command_syntax_row(
                token,
                description_key,
                true,
                cx,
            ));
        }

        panel
            .child(
                div()
                    .h(px(1.0))
                    .w_full()
                    .my(px(2.0))
                    .bg(rgba((theme.border << 8) | 0x66)),
            )
            .child(self.render_quick_command_syntax_row(
                "|sh",
                "terminal.quick_commands.syntax_help_shell",
                false,
                cx,
            ))
            .child(self.render_quick_command_syntax_row(
                "\\{{",
                "terminal.quick_commands.syntax_help_escape",
                false,
                cx,
            ))
            .into_any_element()
    }

    fn render_quick_command_syntax_group_title(&self, key: &'static str) -> AnyElement {
        div()
            .mt(px(2.0))
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(self.tokens.ui.text))
            .child(self.i18n.t(key))
            .into_any_element()
    }

    fn render_quick_command_syntax_row(
        &self,
        token: &'static str,
        description_key: &'static str,
        insertable: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        let mut chip = div()
            .h(px(22.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .rounded(px(self.tokens.radii.sm))
            .border_1()
            .border_color(if insertable {
                rgba((theme.accent << 8) | 0x59)
            } else {
                rgba((theme.border << 8) | 0x99)
            })
            .bg(rgba((theme.bg << 8) | 0x80))
            .font_family(settings_mono_font_family(self.settings_store.settings()))
            .text_size(px(self.tokens.metrics.ui_text_xs))
            .text_color(rgb(if insertable {
                theme.accent
            } else {
                theme.text_muted
            }))
            .child(token.to_string());

        if insertable {
            chip = chip
                .cursor_pointer()
                .hover(|chip| chip.bg(rgba((theme.bg_hover << 8) | 0xb3)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        let editor = this.terminal.read(cx).quick_commands.command_input.clone();
                        if let Some(editor) = editor {
                            editor.update(cx, |editor, cx| editor.insert_text(token, cx));
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );
        }

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(chip)
            .child(
                div()
                    .text_size(px(self.tokens.metrics.ui_text_xs))
                    .text_color(rgb(theme.text_muted))
                    .child(self.i18n.t(description_key)),
            )
            .into_any_element()
    }

    fn render_quick_editor_buttons(
        &self,
        can_save: bool,
        save_key: &'static str,
        save: fn(&mut WorkspaceApp, &mut Context<WorkspaceApp>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.tokens.ui;
        div()
            .flex()
            .justify_end()
            .gap(px(8.0))
            .child(self.quick_command_text_button(
                self.i18n.t("terminal.quick_commands.cancel"),
                true,
                cx.listener(|this, _event, _window, cx| {
                    this.terminal
                        .update(cx, |terminal, _cx| terminal.quick_commands.cancel_editor());
                    cx.stop_propagation();
                    cx.notify();
                }),
            ))
            .child(
                self.quick_command_text_button(
                    self.i18n.t(save_key),
                    can_save,
                    cx.listener(move |this, _event, _window, cx| {
                        save(this, cx);
                        cx.stop_propagation();
                    }),
                )
                .bg(if can_save {
                    rgba((theme.accent << 8) | 0x26)
                } else {
                    rgba(0x00000000)
                }),
            )
            .into_any_element()
    }

    fn render_quick_command_text_input(
        &self,
        input: QuickCommandInput,
        value: String,
        focused_input: Option<QuickCommandInput>,
        placeholder: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if input == QuickCommandInput::CommandText {
            return self.render_quick_command_multiline_input(input, focused_input, cx);
        }
        self.render_quick_command_input(input, &value, focused_input, placeholder, false, cx)
    }

    fn render_quick_command_multiline_input(
        &self,
        input: QuickCommandInput,
        focused_input: Option<QuickCommandInput>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editor = self.terminal.read(cx).quick_commands.command_input.clone();
        let focus = editor
            .as_ref()
            .map(|editor| editor.read(cx).focus_handle(cx));
        div()
            .id("quick-command-text-editor")
            .w_full()
            .min_w_0()
            .h(px(QUICK_COMMAND_TEXTAREA_MIN_HEIGHT))
            .flex_none()
            .p(px(QUICK_COMMAND_TEXTAREA_VERTICAL_PADDING))
            .rounded(px(self.tokens.radii.md))
            .border_1()
            .border_color(rgb(if focused_input == Some(input) {
                self.tokens.ui.accent
            } else {
                self.tokens.ui.border
            }))
            .bg(rgba((self.tokens.ui.bg << 8) | 0x80))
            .overflow_hidden()
            .capture_any_mouse_down(cx.listener(move |this, _, window, cx| {
                this.terminal.update(cx, |terminal, _| {
                    terminal.quick_commands.set_focused_input(input);
                });
                this.clear_ime_selection();
                this.ime_marked_text = None;
                if let Some(focus) = focus.as_ref() {
                    window.focus(focus, cx);
                }
                cx.notify();
            }))
            .children(editor)
            .into_any_element()
    }

    fn render_quick_command_secret_input(
        &self,
        input: QuickCommandInput,
        value: &Zeroizing<String>,
        focused_input: Option<QuickCommandInput>,
        placeholder: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.render_quick_command_input(input, value, focused_input, placeholder, true, cx)
    }

    fn render_quick_command_input(
        &self,
        input: QuickCommandInput,
        value: &str,
        focused_input: Option<QuickCommandInput>,
        placeholder: String,
        secret: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let focused = focused_input == Some(input);
        let target = WorkspaceImeTarget::QuickCommand(input);
        let viewport = self.terminal.read(cx).quick_commands.input_viewport(input);
        let active_offset = self.ime_active_offset_for_target(target, cx);
        self.text_input_with_workspace_ime(
            target,
            text_input_with_viewport(
                &self.tokens,
                TextInputView {
                    value,
                    placeholder,
                    focused,
                    caret_visible: self.input_caret.visible(),
                    secret,
                    selected_all: false,
                    selected_range: self.ime_selected_range_for_target(target, cx),
                    marked_text: self.marked_text_for_target(target, cx),
                },
                &viewport,
                active_offset,
            )
            .h(px(32.0))
            .when(quick_command_input_uses_monospace(input), |field| {
                field.font_family(settings_mono_font_family(self.settings_store.settings()))
            }),
            move |this, cx| {
                this.close_terminal_command_overlays(cx);
                this.terminal.update(cx, |terminal, _cx| {
                    terminal.quick_commands.set_focused_input(input)
                });
                this.terminal_command_composer.update(cx, |composer, cx| {
                    composer.set_compact_focused(false, cx);
                });
            },
            cx,
        )
        .into_any_element()
    }

    pub(in crate::workspace) fn quick_command_text_editor_focused(
        &self,
        window: &gpui::Window,
        cx: &App,
    ) -> bool {
        let commands = &self.terminal.read(cx).quick_commands;
        commands.manager_open()
            && !commands.category_delete_pending()
            && commands
                .command_input
                .as_ref()
                .is_some_and(|editor| editor.read(cx).focus_handle(cx).is_focused(window))
    }

    fn initialize_quick_command_text_editor(&mut self, cx: &mut Context<Self>) {
        let value = self
            .terminal
            .read(cx)
            .quick_commands
            .store
            .command_editor
            .as_ref()
            .map(|draft| draft.command.clone())
            .unwrap_or_default();
        let tokens = self.tokens;
        let font_family = settings_mono_font_family(self.settings_store.settings()).to_string();
        let placeholder = self.i18n.t("terminal.quick_commands.command_placeholder");
        let labels = EditorContextMenuLabels {
            copy: self.i18n.t("menu.copy"),
            cut: self.i18n.t("fileManager.cut"),
            paste: self.i18n.t("menu.paste"),
            select_all: self.i18n.t("fileManager.selectAll"),
        };
        let workspace = cx.weak_entity();
        let editor = cx.new(|cx| {
            let mut editor = TextEditorView::new(value, &tokens, cx);
            editor.set_settings(
                EditorSettings {
                    soft_wrap: false,
                    indentation_markers: false,
                    highlight_special_chars: false,
                    placeholder: Some(placeholder),
                    ..EditorSettings::default()
                },
                cx,
            );
            editor.set_border_visible(false);
            editor.apply_runtime_settings(
                &tokens,
                font_family,
                tokens.metrics.ui_text_sm,
                QUICK_COMMAND_TEXTAREA_LINE_HEIGHT / tokens.metrics.ui_text_sm,
                false,
                true,
                cx,
            );
            editor.set_context_menu_labels(labels);
            editor.set_on_save(Box::new(move |_, _, cx| {
                let workspace = workspace.clone();
                let editor_id = cx.entity_id();
                // Save after the editor update so the workspace can read its current buffer.
                cx.defer(move |cx| {
                    let _ = workspace.update(cx, |this, cx| {
                        if this
                            .terminal
                            .read(cx)
                            .quick_commands
                            .command_input
                            .as_ref()
                            .is_some_and(|editor| editor.entity_id() == editor_id)
                        {
                            this.save_quick_command_editor(cx);
                        }
                    });
                });
                Ok(())
            }));
            editor
        });
        let mut version = editor.read(cx).buffer().version();
        let subscription = cx.observe(&editor, move |this, editor, cx| {
            let buffer = editor.read(cx).buffer();
            if buffer.version() == version {
                return;
            }
            version = buffer.version();
            let text = buffer.text();
            this.terminal.update(cx, |terminal, _| {
                if terminal.quick_commands.command_input.as_ref() == Some(&editor)
                    && let Some(draft) = terminal.quick_commands.store.command_editor.as_mut()
                {
                    draft.command = text;
                }
            });
            cx.notify();
        });
        self.terminal.update(cx, |terminal, _| {
            terminal.quick_commands.command_input = Some(editor);
            terminal.quick_commands.command_input_subscription = Some(subscription);
            terminal.quick_commands.editor_scroll = gpui::ScrollHandle::new();
        });
    }

    fn start_quick_command_create(&mut self, cx: &mut Context<Self>) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.start_command_create();
        });
        self.initialize_quick_command_text_editor(cx);
        cx.notify();
    }

    pub(in crate::workspace) fn start_quick_command_create_with_content(
        &mut self,
        command: String,
        cx: &mut Context<Self>,
    ) {
        self.prepare_modal_interaction_boundary(cx);
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.open_manager();
            terminal.quick_commands.start_command_create_with_content(command);
        });
        self.initialize_quick_command_text_editor(cx);
        self.ime_marked_text = None;
        cx.notify();
    }

    pub(in crate::workspace) fn start_quick_command_edit(
        &mut self,
        command: QuickCommand,
        cx: &mut Context<Self>,
    ) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.start_command_edit(command)
        });
        self.initialize_quick_command_text_editor(cx);
        cx.notify();
    }

    fn start_quick_command_category_create(&mut self, cx: &mut Context<Self>) {
        self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.start_category_create()
        });
        cx.notify();
    }

    fn save_quick_command_editor(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.terminal.read(cx).quick_commands.command_input.clone() {
            let text = editor.read(cx).buffer().text();
            self.terminal.update(cx, |terminal, _| {
                if let Some(draft) = terminal.quick_commands.store.command_editor.as_mut() {
                    draft.command = text;
                }
            });
        }
        if self.terminal.update(cx, |terminal, _cx| {
            terminal.quick_commands.save_command_editor()
        }) {
            cx.notify();
        }
    }

}

#[cfg(test)]
mod terminal_command_bar_quick_command_tests {
    use super::{
        QuickCommandConfirmationPolicy, QuickCommandEditorDraft, QuickCommandMatchField,
        QuickCommandParameterEditorDraft, QuickCommandParameterKind, QuickCommandRiskBadge,
        quick_command_editor_can_save, quick_command_risk_badge,
        quick_command_space_inserts_literal,
    };
    use gpui::{
        Context, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle, Styled, div,
        px,
    };

    #[test]
    fn quick_bar_filters_independently_and_keeps_empty_groups_selectable() {
        let directory = tempfile::tempdir().unwrap();
        let mut state =
            super::TerminalQuickCommandsState::load(&directory.path().join("settings.json"));
        let template = oxideterm_quick_commands::default_quick_commands().remove(0);
        state.store.commands = [("one", "system"), ("two", "files"), ("ssh-only", "system")]
            .into_iter()
            .map(|(id, category)| {
                let mut command = template.clone();
                command.id = id.into();
                command.category = category.into();
                command.availability = Default::default();
                if id == "ssh-only" {
                    command.availability.protocols = vec![super::QuickCommandTargetProtocol::Ssh];
                }
                command
            })
            .collect();
        state.store.active_category = "files".into();
        state.store.query = "no matches".into();
        let target = oxideterm_quick_commands::QuickCommandTargetContext {
            target_id: "ssh".into(),
            label: "SSH".into(),
            protocol: super::QuickCommandTargetProtocol::Ssh,
            values: Default::default(),
        };
        for (category, target, expected) in [
            (None, None, vec!["one", "two"]),
            (Some("system"), None, vec!["one"]),
            (Some("system"), Some(&target), vec!["one", "ssh-only"]),
            (Some("custom"), None, vec![]),
            (Some("deleted"), None, vec!["one", "two"]),
        ] {
            let (categories, commands) = state.quick_bar_snapshot(target, category);
            assert_eq!(
                commands.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
                expected,
                "{category:?}"
            );
            assert!(categories.iter().any(|c| c.id == "custom"));
        }
    }

    #[test]
    fn quick_bar_group_preference_survives_reload_and_clearing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let mut store = oxideterm_settings::SettingsStore::load_from_path(&path).unwrap();
        store.settings_mut().terminal.command_bar.quick_bar_category = Some("files".into());
        store.save().unwrap();
        let mut loaded = oxideterm_settings::SettingsStore::load_from_path(&path).unwrap();
        assert_eq!(
            loaded
                .settings()
                .terminal
                .command_bar
                .quick_bar_category
                .as_deref(),
            Some("files")
        );
        loaded
            .settings_mut()
            .terminal
            .command_bar
            .quick_bar_category = None;
        loaded.save().unwrap();
        assert_eq!(
            oxideterm_settings::SettingsStore::load_from_path(&path)
                .unwrap()
                .settings()
                .terminal
                .command_bar
                .quick_bar_category,
            None
        );
    }

    #[test]
    fn category_color_editor_validates_persists_and_resets_to_theme() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let mut state = super::TerminalQuickCommandsState::load(&path);
        let category = state
            .store
            .categories
            .iter()
            .find(|c| c.id == "system")
            .unwrap()
            .clone();
        state.start_category_edit(category);
        state.set_focused_input(super::QuickCommandInput::CategoryColor);
        state.replace_input(super::QuickCommandInput::CategoryColor, None, "#GG0000");
        assert!(!state.save_category_editor());
        assert_eq!(
            state.input_value(super::QuickCommandInput::CategoryColor),
            Some("#GG0000")
        );
        state.replace_input(
            super::QuickCommandInput::CategoryColor,
            Some(0..7),
            "#22D3EE",
        );
        assert!(state.save_category_editor());
        let loaded = oxideterm_quick_commands::load_snapshot(&path).unwrap();
        let category = loaded
            .categories
            .into_iter()
            .find(|c| c.id == "system")
            .unwrap();
        assert_eq!(category.icon_color.as_deref(), Some("#22d3ee"));
        state.start_category_edit(category);
        state.set_focused_input(super::QuickCommandInput::CategoryColor);
        state.replace_input(super::QuickCommandInput::CategoryColor, Some(0..7), "");
        assert!(state.save_category_editor());
        let loaded = oxideterm_quick_commands::load_snapshot(&path).unwrap();
        assert_eq!(
            loaded
                .categories
                .iter()
                .find(|c| c.id == "system")
                .unwrap()
                .icon_color,
            None
        );
    }

    struct EditorFormFixture {
        scroll: ScrollHandle,
        rows: usize,
    }

    impl Render for EditorFormFixture {
        fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
            let rows = self.rows;
            let form = div()
                .w_full()
                .flex()
                .flex_col()
                .flex_none()
                .children((0..self.rows).map(|index| {
                    div()
                        .h(px(64.0))
                        .flex_none()
                        .debug_selector(move || {
                            if index + 1 == rows {
                                "last-field".into()
                            } else {
                                format!("field-{index}")
                            }
                        })
                        .child(format!("Parameter {index}"))
                }));
            super::quick_command_editor_frame(
                form.into_any_element(),
                div()
                    .h(px(48.0))
                    .debug_selector(|| "footer".into())
                    .into_any_element(),
                &self.scroll,
                &oxideterm_theme::default_tokens(),
            )
        }
    }

    #[gpui::test]
    fn long_command_form_reaches_last_field_without_moving_footer(cx: &mut gpui::TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| EditorFormFixture {
            scroll: ScrollHandle::new(),
            rows: 20,
        });
        for height in [300.0, 180.0] {
            cx.simulate_resize(gpui::size(px(480.0), px(height)));
            cx.update(|window, app| {
                window.draw(app).clear(app);
            });
            let footer = cx.debug_bounds("footer").unwrap();
            assert_eq!(footer.bottom(), px(height));
            let scroll = view.read_with(cx, |view, _| view.scroll.clone());
            let max_offset = scroll.max_offset();
            assert!(max_offset.y > px(64.0 * 15.0));
            scroll.set_offset(gpui::point(px(0.0), -max_offset.y));
            for _ in 0..3 {
                cx.update(|window, app| {
                    window.refresh();
                    window.draw(app).clear(app);
                });
                assert_eq!(scroll.max_offset(), max_offset);
                assert_eq!(cx.debug_bounds("footer").unwrap(), footer);
                let last = cx.debug_bounds("last-field").unwrap();
                assert!(last.top() >= scroll.bounds().top());
                assert!((last.bottom() - scroll.bounds().bottom()).abs() < px(1.0));
            }
        }
    }

    #[test]
    fn dock_pinning_keeps_launcher_open_and_blur_preserves_confirmation_until_close() {
        let directory = tempfile::tempdir().unwrap();
        let mut commands =
            super::TerminalQuickCommandsState::load(&directory.path().join("settings.json"));
        let command = oxideterm_quick_commands::default_quick_commands().remove(0);
        for pinned in [false, true] {
            commands.open = true;
            commands.pinned = pinned;
            commands.set_focused_input(super::QuickCommandInput::Search);
            assert_eq!(
                commands.prepare_insertion("printf 'hello'".into(), pinned),
                "printf 'hello'"
            );
            assert_eq!(commands.is_open(), true);
            assert_eq!(commands.focused_input(), None);
            commands.request_execution(command.clone());
            commands.set_focused_input(super::QuickCommandInput::Search);
            commands.blur_input();
            assert!(commands.is_open());
            assert_eq!(commands.focused_input(), None);
            assert_eq!(
                commands.pending_execution.as_ref().unwrap().command.id,
                command.id
            );
            commands.set_focused_input(super::QuickCommandInput::ParameterDefault(0));
            commands.finish_execution();
            assert_eq!(commands.is_open(), true);
            assert_eq!(commands.focused_input(), None);
            assert!(commands.pending_execution.is_none());
            commands.request_execution(command.clone());
            commands.close();
            assert!(!commands.is_open());
            assert!(commands.pending_execution.is_none());
        }
    }

    #[test]
    fn quick_command_plain_space_is_literal_text() {
        assert!(quick_command_space_inserts_literal(false, false, false));
        assert!(!quick_command_space_inserts_literal(true, false, false));
        assert!(!quick_command_space_inserts_literal(false, true, false));
        assert!(!quick_command_space_inserts_literal(false, false, true));
    }

    #[test]
    fn quick_command_editor_validates_templates_and_secret_defaults() {
        let mut draft = QuickCommandEditorDraft {
            id: None,
            name: "Deploy".to_string(),
            command: "deploy {{param.sevrice}}".to_string(),
            category: "custom".to_string(),
            description: String::new(),
            host_patterns: String::new(),
            match_field: QuickCommandMatchField::Host,
            parameters: vec![QuickCommandParameterEditorDraft {
                name: "service".to_string(),
                label: "Service".to_string(),
                kind: QuickCommandParameterKind::Text,
                default_value: String::new(),
                choices: String::new(),
                required: true,
            }],
            protocols: Vec::new(),
            confirmation: QuickCommandConfirmationPolicy::Inherit,
            line_delay_ms: "1000".to_string(),
            syntax_help_expanded: false,
            created_at: 1,
            sort_order: 0,
        };

        assert!(!quick_command_editor_can_save(&draft));
        draft.command = "deploy {{param.service|sh}}".to_string();
        assert!(quick_command_editor_can_save(&draft));
        draft.parameters[0].kind = QuickCommandParameterKind::Secret;
        draft.parameters[0].default_value = "must-not-persist".to_string();
        assert!(!quick_command_editor_can_save(&draft));
        draft.parameters[0].default_value.clear();
        assert!(quick_command_editor_can_save(&draft));
    }

    #[test]
    fn parameterized_commands_show_dynamic_risk_until_expansion() {
        assert_eq!(
            quick_command_risk_badge("echo {{param.value}}"),
            Some(QuickCommandRiskBadge::Dynamic)
        );
    }
}
