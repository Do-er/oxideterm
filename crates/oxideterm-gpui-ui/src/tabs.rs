use gpui::{Div, InteractiveElement, ParentElement, Styled, div, px, rgb, rgba};
use oxideterm_theme::ThemeTokens;

pub fn segmented_tabs(tokens: &ThemeTokens) -> Div {
    tabs_list(tokens)
}

pub fn tabs_list(tokens: &ThemeTokens) -> Div {
    div()
        .h(px(tokens.metrics.ui_tabs_list_height))
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .p(px(tokens.metrics.ui_tabs_list_padding))
        .rounded(px(tokens.radii.sm))
        .border_1()
        .border_color(rgb(tokens.ui.border))
        .bg(rgb(tokens.ui.bg_panel))
        .text_color(rgb(tokens.ui.text_muted))
}

pub fn segmented_tab(tokens: &ThemeTokens, label: String, selected: bool) -> Div {
    tabs_trigger(tokens, label, selected)
}

pub fn tabs_trigger(tokens: &ThemeTokens, label: String, selected: bool) -> Div {
    let theme = tokens.ui;
    let mut tab = div()
        .flex_1()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .whitespace_nowrap()
        .cursor_pointer()
        .rounded(px(tokens.radii.xs))
        .border_1()
        .px(px(tokens.metrics.ui_tabs_trigger_padding_x))
        .text_size(px(tokens.metrics.ui_text_sm));

    if selected {
        tab = tab
            .bg(rgb(theme.bg))
            .border_color(rgba((theme.border << 8) | 0x99))
            .text_color(rgb(theme.text))
            .font_weight(gpui::FontWeight::SEMIBOLD);
    } else {
        tab = tab
            .bg(rgba(0x00000000))
            .border_color(rgba(0x00000000))
            .text_color(rgb(theme.text_muted))
            .font_weight(gpui::FontWeight::MEDIUM)
            .hover(|style| {
                style
                    .text_color(rgb(theme.text))
                    .bg(rgba((theme.bg_hover << 8) | 0x80))
            });
    }

    tab.child(label)
}

pub fn tabs_content(tokens: &ThemeTokens, content: impl gpui::IntoElement) -> Div {
    div().mt(px(tokens.spacing.two)).child(content)
}
