//! What the Library section draws: the location picker, and a Personas and
//! a Prompts group for the chosen location's index.
//!
//! Contract: `openspec/changes/import-from-knot-library/specs/import-ui/spec.
//! md`.
//!
//! A held item is listed, not hidden, and its row says why - "Already in
//! Knot" or "Deleted in Knot" - rather than disappearing, which would read
//! as Knot never having found it.

use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::div;
use knot_core::import::{ItemStatus, classify};
use knot_library::{Index, IndexItem, ItemKind};

use super::library::{BUILTIN_LIBRARY_NAME, LibraryIndex, LocationRef};
use super::window::ImportWindow;
use crate::controls::{group, icon_button};

impl ImportWindow {
    /// The Library section: the location picker, then whatever the chosen
    /// location's index says.
    pub(super) fn render_library_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        group(knot_core::l10n::t("import.library_title")).child(self.render_location_picker(cx))
                                                         .child(self.render_library_body(cx))
    }

    fn render_location_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = crate::settings_global::read(cx);
        let import_window = cx.entity();
        let selected = self.library.selected;

        let mut row = h_flex().gap_2().flex_wrap();
        row = row.child(Self::location_button(("import-library-location", 0_usize),
                                              BUILTIN_LIBRARY_NAME,
                                              selected == LocationRef::BuiltIn,
                                              LocationRef::BuiltIn,
                                              import_window.clone()));
        for (index, saved) in settings.library_locations.iter().enumerate() {
            row = row.child(Self::location_button(("import-library-location", index + 1),
                                                  saved.name.clone(),
                                                  selected == LocationRef::Saved(saved.id),
                                                  LocationRef::Saved(saved.id),
                                                  import_window.clone()));
        }

        let is_builtin = selected == LocationRef::BuiltIn;
        h_flex().justify_between()
                .items_center()
                .child(row)
                .child(h_flex().gap_1()
                               .child(icon_button("import-library-add",
                                                  "icons/plus.svg",
                                                  knot_core::l10n::t("import.library_add"),
                                                  false).on_click({
                                   let import_window = import_window.clone();
                                   move |_, window, app| {
                                       import_window.update(app, |view, cx| {
                                                        view.open_add_location_dialog(window, cx);
                                                    });
                                   }
                               }))
                               .child(icon_button("import-library-rename",
                                                  "icons/pencil.svg",
                                                  knot_core::l10n::t("import.library_rename"),
                                                  false).disabled(is_builtin)
                                                        .on_click({
                                                            let import_window =
                                                                import_window.clone();
                                                            move |_, window, app| {
                                                                import_window.update(app, |view, cx| {
                                                             view.open_rename_location_dialog(window, cx);
                                                         });
                                                            }
                                                        }))
                               .child(icon_button("import-library-remove",
                                                  "icons/trash-2.svg",
                                                  knot_core::l10n::t("import.library_remove"),
                                                  true).disabled(is_builtin)
                                                       .on_click({
                                                           let import_window = import_window.clone();
                                                           move |_, window, app| {
                                                               import_window.update(app, |view, cx| {
                                                                   view.confirm_remove_location(window, cx);
                                                               });
                                                           }
                                                       })))
    }

    fn location_button(id: impl Into<gpui_kit::ElementId>,
                       label: impl Into<gpui_kit::SharedString>, selected: bool,
                       target: LocationRef, import_window: gpui_kit::Entity<ImportWindow>)
                       -> Button {
        let mut button = Button::new(id).label(label).small();
        button = if selected {
            button.primary()
        }
        else {
            button.ghost()
        };
        button.on_click(move |_, window, app| {
                  import_window.update(app, |view, cx| {
                                   if view.library.selected != target {
                                       view.library.selected = target;
                                       view.fetch_library_index(window, cx);
                                   }
                               });
              })
    }

    fn render_library_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.library.index {
            LibraryIndex::Loading => {
                Self::library_message("import.library_loading", cx).into_any_element()
            }
            LibraryIndex::Unreachable => self.render_library_error("import.library_unreachable",
                                                                   cx)
                                             .into_any_element(),
            LibraryIndex::Unsupported => self.render_library_error("import.library_unsupported",
                                                                   cx)
                                             .into_any_element(),
            LibraryIndex::Loaded(index) => self.render_library_index(index, cx).into_any_element(),
        }
    }

    fn render_library_error(&self, key: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let import_window = cx.entity();
        v_flex().gap_2()
                .child(Self::library_message(key, cx))
                .child(h_flex().child(icon_button("import-library-retry",
                                                  "icons/rotate-ccw.svg",
                                                  knot_core::l10n::t("import.library_retry"),
                                                  false).on_click(move |_, window, app| {
                                                            import_window.update(app, |view, cx| {
                                                                view.fetch_library_index(window,
                                                                                         cx);
                                                            });
                                                        })))
    }

    fn library_message(key: &str, cx: &Context<Self>) -> impl IntoElement {
        div().text_sm()
             .text_color(cx.theme().muted_foreground)
             .child(knot_core::l10n::t(key))
    }

    fn render_library_index(&self, index: &Index, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = crate::settings_global::read(cx);
        let personas: Vec<&IndexItem> = index.items
                                             .iter()
                                             .filter(|item| item.kind == ItemKind::Persona)
                                             .collect();
        let prompts: Vec<&IndexItem> = index.items
                                            .iter()
                                            .filter(|item| item.kind == ItemKind::Prompt)
                                            .collect();

        v_flex().gap_3()
                .child(self.render_library_item_group("import.library_personas_title",
                                                      &personas,
                                                      &self.library.persona_selection,
                                                      &settings,
                                                      true,
                                                      cx))
                .child(self.render_library_item_group("import.library_prompts_title",
                                                      &prompts,
                                                      &self.library.prompt_selection,
                                                      &settings,
                                                      false,
                                                      cx))
                .child(self.render_library_import_row(cx))
    }

    fn render_library_item_group(&self, title_key: &str, items: &[&IndexItem],
                                 selection: &std::collections::BTreeSet<String>,
                                 settings: &knot_core::Settings, is_persona: bool,
                                 cx: &mut Context<Self>)
                                 -> impl IntoElement {
        let import_window = cx.entity();
        if items.is_empty() {
            return group(knot_core::l10n::t(title_key))
                .child(Self::library_message("import.library_none", cx))
                .into_any_element();
        }

        v_flex().gap_2()
                .children(items.iter().enumerate().map(|(row_index, item)| {
                    let status = classify(settings, item);
                    let id = item.id.clone();
                    let checked = selection.contains(&id);
                    let import_window = import_window.clone();
                    let row = h_flex().gap_2()
                                      .items_center()
                                      .child(Checkbox::new(("import-library-item", row_index))
                        .label(item.title.clone())
                        .checked(checked)
                        .disabled(status != Some(ItemStatus::Importable))
                        .on_click(move |checked, _, app| {
                            let id = id.clone();
                            let checked = *checked;
                            import_window.update(app, |view, cx| {
                                             let selection = if is_persona {
                                                 &mut view.library.persona_selection
                                             }
                                             else {
                                                 &mut view.library.prompt_selection
                                             };
                                             if checked {
                                                 selection.insert(id);
                                             }
                                             else {
                                                 selection.remove(&id);
                                             }
                                             cx.notify();
                                         });
                        }));
                    match status {
                        Some(ItemStatus::AlreadyHeld) => {
                            row.child(Self::held_reason("import.library_already_in_knot", cx))
                               .into_any_element()
                        }
                        Some(ItemStatus::DeletedBuiltin) => {
                            row.child(Self::held_reason("import.library_deleted_in_knot", cx))
                               .into_any_element()
                        }
                        _ => row.into_any_element(),
                    }
                }))
                .into_any_element()
    }

    fn held_reason(key: &str, cx: &Context<Self>) -> impl IntoElement {
        div().text_xs()
             .text_color(cx.theme().muted_foreground)
             .child(knot_core::l10n::t(key))
    }

    fn render_library_import_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let import_window = cx.entity();
        let selected = self.library.persona_selection.len() + self.library.prompt_selection.len();
        let label = if selected == 0 {
            knot_core::l10n::t("import.import_selected")
        }
        else {
            knot_core::l10n::t_with("import.import_count", &[("count", &selected.to_string())])
        };

        h_flex().justify_end()
                .child(Button::new("import-library").label(label)
                                                    .primary()
                                                    .small()
                                                    .disabled(selected == 0)
                                                    .on_click(move |_, window, app| {
                                                        import_window.update(app, |view, cx| {
                                                view.import_selected_library_items(window, cx);
                                            });
                                                    }))
    }
}
