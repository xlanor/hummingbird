use std::path::PathBuf;

use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    PathPromptOptions, Render, SharedString, Styled, Window, div, px,
};

use crate::{
    settings::{Settings, SettingsGlobal, save_settings},
    ui::{
        components::{
            button::{ButtonIntent, ButtonStyle, button},
            icons::{CIRCLE_PLUS, FOLDER_SEARCH, TRASH, icon},
            section_header::section_header,
        },
        theme::Theme,
    },
};

pub struct LibrarySettings {
    settings: Entity<Settings>,
}

impl LibrarySettings {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let settings = cx.global::<SettingsGlobal>().model.clone();
            cx.observe(&settings, |_, _, cx| cx.notify()).detach();

            Self {
                settings: cx.global::<SettingsGlobal>().model.clone(),
            }
        })
    }

    fn add_folder(&self, cx: &mut App) {
        let path_future = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Select a folder to scan...".into()),
        });

        let settings = self.settings.clone();

        cx.spawn(async move |cx| {
            if let Ok(Ok(Some(mut paths))) = path_future.await {
                if let Some(path) = paths.pop() {
                    let path = path.canonicalize().unwrap_or(path);

                    settings
                        .update(cx, move |settings, cx| {
                            let mut updated = false;

                            if !settings.scanning.paths.contains(&path) {
                                settings.scanning.paths.push(path);
                                updated = true;
                            }

                            if updated {
                                save_settings(cx, settings);
                                cx.notify();
                            }
                        })
                        .expect("settings model could not be updated");
                }
            }
        })
        .detach();
    }

    fn remove_folder(settings: Entity<Settings>, path: PathBuf, cx: &mut App) {
        settings.update(cx, move |settings, cx| {
            let before_len = settings.scanning.paths.len();
            settings.scanning.paths.retain(|p| p != &path);

            if settings.scanning.paths.len() != before_len {
                save_settings(cx, settings);
                cx.notify();
            }
        });
    }
}

impl Render for LibrarySettings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let paths = self.settings.read(cx).scanning.paths.clone();

        let list = if paths.is_empty() {
            div()
                .mt(px(12.0))
                .text_sm()
                .text_color(theme.text_secondary)
                .child("No folders are currently scanned.")
        } else {
            let rows = paths.iter().enumerate().map(|(idx, path)| {
                let path_clone = path.clone();
                let settings = self.settings.clone();
                let path_text: SharedString = path.to_string_lossy().to_string().into();

                div()
                    .id(format!("library-scan-path-{idx}"))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .pl(px(12.0))
                    .pr(px(8.0))
                    .py(px(8.0))
                    .rounded(px(4.0))
                    .bg(theme.background_secondary)
                    .child(
                        icon(FOLDER_SEARCH)
                            .size(px(16.0))
                            .text_color(theme.text_secondary),
                    )
                    .child(
                        div()
                            .flex_grow()
                            .overflow_hidden()
                            .text_ellipsis()
                            .text_sm()
                            .child(path_text),
                    )
                    .child(
                        button()
                            .style(ButtonStyle::Minimal)
                            .intent(ButtonIntent::Secondary)
                            .child(icon(TRASH).size(px(14.0)))
                            .id(format!("library-scan-remove-{idx}"))
                            .on_click(move |_, _, cx| {
                                LibrarySettings::remove_folder(
                                    settings.clone(),
                                    path_clone.clone(),
                                    cx,
                                );
                            }),
                    )
            });

            div().flex().flex_col().gap(px(8.0)).children(rows)
        };

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                section_header("Scanning")
                    .subtitle("Changes apply on your next scan. Duplicate folders are ignored.")
                    .child(
                        button()
                            .style(ButtonStyle::Regular)
                            .intent(ButtonIntent::Primary)
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.0))
                                    .child(icon(CIRCLE_PLUS).my_auto().size(px(14.0)))
                                    .child("Add Folder"),
                            )
                            .id("library-settings-add-folder")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.add_folder(cx);
                            })),
                    ),
            )
            .child(list)
    }
}
