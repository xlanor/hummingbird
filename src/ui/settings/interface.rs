use cntp_i18n::{I18N_MANAGER, Locale, tr};
use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, px,
};

use crate::{
    settings::{SettingsGlobal, save_settings},
    ui::components::{
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
            Self { language_dropdown }
        })
    }
}

impl Render for InterfaceSettings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _theme = cx.global::<Theme>();

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
    }
}
