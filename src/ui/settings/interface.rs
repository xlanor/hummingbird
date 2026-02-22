use cntp_i18n::{I18N_MANAGER, Locale, tr};
use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, px,
};

use crate::{
    settings::{SettingsGlobal, save_settings},
    ui::components::{
        checkbox::checkbox,
        dropdown::{DropdownOption, DropdownState, dropdown},
        label::label,
        section_header::section_header,
    },
    ui::theme::Theme,
};

#[derive(Clone)]
pub struct LanguageOption {
    pub code: &'static str,
    pub display_name: SharedString,
}

fn get_available_languages() -> Vec<LanguageOption> {
    vec![
        LanguageOption {
            code: "",
            display_name: tr!("LANGUAGE_SYSTEM_DEFAULT", "System Default").into(),
        },
        LanguageOption {
            code: "en",
            display_name: "English".into(),
        },
        LanguageOption {
            code: "vi",
            display_name: "Tiếng Việt".into(),
        },
        LanguageOption {
            code: "el",
            display_name: "Ελληνικά".into(),
        },
    ]
}

fn update_language(code: &str) {
    if let Ok(mut manager) = I18N_MANAGER.write() {
        if code.is_empty() {
            manager.locale = Locale::current();
        } else {
            manager.locale = Locale::new_from_locale_identifier(code);
        }
    }
}

pub struct InterfaceSettings {
    settings: Entity<crate::settings::Settings>,
    language_dropdown: Entity<DropdownState>,
}

impl InterfaceSettings {
    pub fn new(cx: &mut App) -> Entity<Self> {
        let settings = cx.global::<SettingsGlobal>().model.clone();
        let interface = settings.read(cx).interface.clone();

        let languages = get_available_languages();
        let dropdown_options: Vec<DropdownOption> = languages
            .iter()
            .map(|lang| DropdownOption::new(lang.code, lang.display_name.clone()))
            .collect();

        let selected_index = languages
            .iter()
            .position(|l| l.code == interface.language)
            .unwrap_or(0);

        let focus_handle = cx.focus_handle();
        let language_dropdown = dropdown(cx, dropdown_options, selected_index, focus_handle);

        language_dropdown.update(cx, |state, _| {
            state.set_width(px(250.0));
        });

        let settings_for_handler = settings.clone();
        language_dropdown.update(cx, |state, _| {
            state.set_on_change(move |_idx, option, _window, cx| {
                let code = option.id.to_string();

                settings_for_handler.update(cx, |settings, cx| {
                    settings.interface.language = code;
                    save_settings(cx, settings);
                });
            });
        });

        cx.new(|cx| {
            cx.observe(&settings, |_, _, cx| cx.notify()).detach();
            Self {
                settings,
                language_dropdown,
            }
        })
    }

    fn update_interface(
        &self,
        cx: &mut App,
        update: impl FnOnce(&mut crate::settings::interface::InterfaceSettings),
    ) {
        self.settings.update(cx, move |settings, cx| {
            update(&mut settings.interface);

            save_settings(cx, settings);
            cx.notify();
        });
    }
}

impl Render for InterfaceSettings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _theme = cx.global::<Theme>();
        let interface = self.settings.read(cx).interface.clone();

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(section_header(tr!("INTERFACE")))
            .child(
                label("language-selector", tr!("LANGUAGE", "Language"))
                    .subtext(tr!(
                        "LANGUAGE_SUBTEXT",
                        "Select your preferred language for the application. Changes to the \
                        language will take effect after restarting the application."
                    ))
                    .w_full()
                    .child(self.language_dropdown.clone()),
            )
            .child(
                label(
                    "interface-full-width-library",
                    tr!("INTERFACE_FULL_WIDTH_LIBRARY", "Full-width library"),
                )
                .subtext(tr!(
                    "INTERFACE_FULL_WIDTH_LIBRARY_SUBTEXT",
                    "Allows the library to take up the full width of the screen."
                ))
                .cursor_pointer()
                .w_full()
                .has_checkbox()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_interface(cx, |interface| {
                        interface.full_width_library = !interface.full_width_library;
                    });
                }))
                .child(checkbox(
                    "interface-full-width-library-check",
                    interface.full_width_library,
                )),
            )
    }
}
