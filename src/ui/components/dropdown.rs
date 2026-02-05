use std::time::Instant;

use cntp_i18n::tr;
use gpui::{
    App, AppContext, Corner, ElementId, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyBinding, KeyDownEvent, ParentElement, Pixels, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, actions, anchored, deferred, div,
    prelude::FluentBuilder, px,
};

use crate::ui::{
    components::icons::{CHECK, CHEVRON_DOWN, icon},
    theme::Theme,
};

actions!(
    dropdown,
    [
        Close,
        SelectNext,
        SelectPrev,
        Confirm,
        SelectFirst,
        SelectLast
    ]
);

pub fn bind_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Close, None),
        KeyBinding::new("down", SelectNext, None),
        KeyBinding::new("up", SelectPrev, None),
        KeyBinding::new("tab", SelectNext, None),
        KeyBinding::new("shift-tab", SelectPrev, None),
        KeyBinding::new("enter", Confirm, None),
        KeyBinding::new("space", Confirm, None),
        KeyBinding::new("home", SelectFirst, None),
        KeyBinding::new("end", SelectLast, None),
    ]);
}

#[derive(Clone, Debug)]
pub struct DropdownOption {
    pub id: SharedString,
    pub label: SharedString,
}

impl DropdownOption {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

#[derive(Clone, Default)]
struct TypeToFindState {
    buffer: String,
    last_keystroke: Option<Instant>,
}

impl TypeToFindState {
    const TIMEOUT_MS: u128 = 500;

    fn push_char(&mut self, ch: char) {
        let now = Instant::now();

        if let Some(last) = self.last_keystroke
            && now.duration_since(last).as_millis() > Self::TIMEOUT_MS
        {
            self.buffer.clear();
        }

        self.buffer.push(ch);
        self.last_keystroke = Some(now);
    }

    fn find_match(&self, options: &[DropdownOption]) -> Option<usize> {
        if self.buffer.is_empty() {
            return None;
        }

        let search = self.buffer.to_lowercase();
        options
            .iter()
            .position(|opt| opt.label.to_lowercase().starts_with(&search))
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.last_keystroke = None;
    }
}

type OnChangeHandler = Box<dyn Fn(usize, &DropdownOption, &mut Window, &mut App) + 'static>;

pub struct DropdownState {
    options: Vec<DropdownOption>,
    selected_index: usize,
    highlighted_index: usize,
    is_open: bool,
    type_to_find: TypeToFindState,
    on_change: Option<OnChangeHandler>,
    focus_handle: FocusHandle,
    width: Option<Pixels>,
}

impl DropdownState {
    pub fn new(
        options: Vec<DropdownOption>,
        selected_index: usize,
        focus_handle: FocusHandle,
    ) -> Self {
        Self {
            options,
            selected_index,
            highlighted_index: selected_index,
            is_open: false,
            type_to_find: TypeToFindState::default(),
            on_change: None,
            focus_handle,
            width: None,
        }
    }

    pub fn set_on_change(
        &mut self,
        handler: impl Fn(usize, &DropdownOption, &mut Window, &mut App) + 'static,
    ) {
        self.on_change = Some(Box::new(handler));
    }

    pub fn set_width(&mut self, width: Pixels) {
        self.width = Some(width);
    }

    fn toggle_open(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open {
            self.highlighted_index = self.selected_index;
            self.type_to_find.clear();
        }
    }

    fn close(&mut self) {
        self.is_open = false;
        self.type_to_find.clear();
    }

    fn select_next(&mut self) {
        if self.highlighted_index < self.options.len().saturating_sub(1) {
            self.highlighted_index += 1;
        }
    }

    fn select_prev(&mut self) {
        if self.highlighted_index > 0 {
            self.highlighted_index -= 1;
        }
    }

    fn select_first(&mut self) {
        self.highlighted_index = 0;
    }

    fn select_last(&mut self) {
        self.highlighted_index = self.options.len().saturating_sub(1);
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(option) = self.options.get(self.highlighted_index) {
            self.selected_index = self.highlighted_index;
            if let Some(handler) = self.on_change.take() {
                let idx = self.highlighted_index;
                let option = option.clone();
                handler(idx, &option, window, cx);
                self.on_change = Some(handler);
            }
        }
        self.close();
    }

    fn handle_key(&mut self, ch: char) {
        self.type_to_find.push_char(ch);
        if let Some(match_idx) = self.type_to_find.find_match(&self.options) {
            self.highlighted_index = match_idx;
        }
    }
}

impl Focusable for DropdownState {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DropdownState {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let display_text = if self.selected_index < self.options.len() {
            self.options[self.selected_index].label.clone()
        } else {
            tr!("DROPDOWN_PLACEHOLDER", "Select...").into()
        };

        let is_open = self.is_open;
        let width = self.width;

        let button = div()
            .id(("dropdown-button", cx.entity_id()))
            .px(px(9.0))
            .py(px(5.0))
            .bg(theme.button_secondary)
            .border_1()
            .border_color(theme.button_secondary_border)
            .rounded(px(4.0))
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .when_some(width, |this, w| this.w(w))
            .when(width.is_none(), |this| this.min_w(px(150.0)))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text)
                    .overflow_hidden()
                    .child(display_text),
            )
            .child(
                icon(CHEVRON_DOWN)
                    .size(px(16.0))
                    .text_color(theme.text_secondary),
            )
            .hover(|this| {
                this.bg(theme.button_secondary_hover)
                    .border_color(theme.button_secondary_border_hover)
            })
            .active(|this| {
                this.bg(theme.button_secondary_active)
                    .border_color(theme.button_secondary_border_active)
            })
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_open();
                cx.notify();
            }));

        let popup = if is_open {
            let options = self.options.clone();
            let selected_index = self.selected_index;
            let highlighted_index = self.highlighted_index;

            let popup_content = div()
                .id("dropdown-popup")
                .occlude()
                .w(width.unwrap_or(px(150.0)))
                .max_h(px(300.0))
                .overflow_y_scroll()
                .bg(theme.elevated_background)
                .border_1()
                .border_color(theme.elevated_border_color)
                .rounded(px(6.0))
                .shadow_md()
                .p(px(3.0))
                .mt(px(4.0))
                .track_focus(&self.focus_handle)
                .key_context("Dropdown")
                .on_action(cx.listener(|this, _: &Close, _, cx| {
                    this.close();
                    cx.notify();
                }))
                .on_action(cx.listener(|this, _: &SelectNext, _, cx| {
                    this.select_next();
                    cx.notify();
                }))
                .on_action(cx.listener(|this, _: &SelectPrev, _, cx| {
                    this.select_prev();
                    cx.notify();
                }))
                .on_action(cx.listener(|this, _: &SelectFirst, _, cx| {
                    this.select_first();
                    cx.notify();
                }))
                .on_action(cx.listener(|this, _: &SelectLast, _, cx| {
                    this.select_last();
                    cx.notify();
                }))
                .on_action(cx.listener(|this, _: &Confirm, window, cx| {
                    this.confirm(window, cx);
                    cx.notify();
                }))
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                    if let Some(key_char) = &ev.keystroke.key_char
                        && let Some(ch) = key_char.chars().next()
                        && ch.is_alphanumeric()
                    {
                        this.handle_key(ch);
                        cx.notify();
                    }
                }))
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.close();
                    cx.notify();
                }))
                .children(options.iter().enumerate().map(|(idx, option)| {
                    let is_selected = idx == selected_index;
                    let is_highlighted = idx == highlighted_index;
                    let label = option.label.clone();

                    div()
                        .id(ElementId::Name(format!("option-{}", idx).into()))
                        .px(px(6.0))
                        .py(px(5.0))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .text_sm()
                        .when(is_highlighted, |this| {
                            this.bg(theme.menu_item_hover)
                                .border_1()
                                .border_color(theme.menu_item_border_hover)
                        })
                        .when(!is_highlighted, |this| this.border_1())
                        .child(
                            div()
                                .w(px(18.0))
                                .h(px(18.0))
                                .pt(px(0.5))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(is_selected, |this| {
                                    this.child(
                                        icon(CHECK).size(px(18.0)).text_color(theme.text_secondary),
                                    )
                                }),
                        )
                        .child(div().text_color(theme.text).child(label))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.highlighted_index = idx;
                            this.confirm(window, cx);
                            cx.notify();
                        }))
                }));

            Some(
                anchored()
                    .anchor(Corner::TopLeft)
                    .child(deferred(popup_content)),
            )
        } else {
            None
        };

        div()
            .id("dropdown-container")
            .relative()
            .child(button)
            .children(popup)
    }
}

pub fn dropdown(
    cx: &mut App,
    options: Vec<DropdownOption>,
    selected_index: usize,
    focus_handle: FocusHandle,
) -> Entity<DropdownState> {
    cx.new(|_| DropdownState::new(options, selected_index, focus_handle))
}
