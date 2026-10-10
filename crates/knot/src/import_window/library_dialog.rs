//! The dialog that adds a saved library location, the name-only dialog that
//! renames one, and the confirmation that removes one.
//!
//! Contract: `openspec/changes/import-from-knot-library/specs/import-ui/spec.
//! md`.
//!
//! Adding checks the location's own form only, not whether it can be
//! reached - an unreachable location shows the same error state as an
//! offline Knot-Library, with a retry, once it is the chosen one.
//!
//! Hosted by `window.open_dialog`/`window.open_alert_dialog`, same as the
//! workspace-name dialog and its delete confirmation - see
//! `workspace_manager/dialog.rs` for why nothing drawn here that depends on
//! live state is cached across frames.

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::dialog::Cancel;
use gpui_kit::component::dialog::Confirm;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputState;
use gpui_kit::div;
use gpui_kit::px;
use knot_library::Location;
use uuid::Uuid;

use super::library::LocationRef;
use super::window::ImportWindow;

/// Which kind the add dialog's form currently offers a field for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DialogLocationKind {
    GitHub,
    Web,
    Folder,
}

/// Entities and transient state the add/rename dialogs need across frames.
/// Built once, in [`ImportWindow::new`], since each input needs a window to
/// construct.
pub(super) struct LibraryDialogState {
    pub(super) name_input:   Entity<InputState>,
    pub(super) repo_input:   Entity<InputState>,
    pub(super) branch_input: Entity<InputState>,
    pub(super) web_input:    Entity<InputState>,
    pub(super) kind:         DialogLocationKind,
    pub(super) folder:       Option<std::path::PathBuf>,
    /// `Some(id)` while renaming that saved location; `None` for Add.
    pub(super) renaming:     Option<Uuid>,
}

impl LibraryDialogState {
    pub(super) fn new(window: &mut Window, cx: &mut gpui_kit::Context<ImportWindow>) -> Self {
        Self { name_input: cx.new(|cx| {
                               InputState::new(window, cx)
                                   .placeholder(knot_core::l10n::t("import.library_dialog_name_placeholder"))
                           }),
               repo_input: cx.new(|cx| {
                               InputState::new(window, cx)
                                   .placeholder(knot_core::l10n::t("import.library_dialog_repo_placeholder"))
                           }),
               branch_input: cx.new(|cx| {
                                 InputState::new(window, cx).placeholder(knot_core::l10n::t(
                    "import.library_dialog_branch_placeholder",
                ))
                             }),
               web_input: cx.new(|cx| {
                              InputState::new(window, cx)
                                  .placeholder(knot_core::l10n::t("import.library_dialog_web_placeholder"))
                          }),
               kind: DialogLocationKind::GitHub,
               folder: None,
               renaming: None }
    }

    /// The location the form currently describes, or `None` while it fails
    /// its own form check.
    fn location(&self, cx: &App) -> Option<Location> {
        let location = match self.kind {
            DialogLocationKind::GitHub => {
                let repo = self.repo_input.read(cx).value().trim().to_string();
                let branch = self.branch_input.read(cx).value().trim().to_string();
                Location::GitHub { repo,
                                   branch: (!branch.is_empty()).then_some(branch) }
            }
            DialogLocationKind::Web => {
                Location::Web { base: self.web_input.read(cx).value().trim().to_string(), }
            }
            DialogLocationKind::Folder => Location::Folder { path: self.folder.clone()?, },
        };
        location.validate().ok()?;
        Some(location)
    }
}

impl ImportWindow {
    pub(super) fn open_add_location_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.library_dialog.renaming = None;
        self.library_dialog.kind = DialogLocationKind::GitHub;
        self.library_dialog.folder = None;
        let name_input = self.library_dialog.name_input.clone();
        let repo_input = self.library_dialog.repo_input.clone();
        let branch_input = self.library_dialog.branch_input.clone();
        let web_input = self.library_dialog.web_input.clone();
        for input in [&name_input, &repo_input, &branch_input, &web_input] {
            cx.update_entity(input, |input, input_cx| {
                  input.set_value("", window, input_cx);
              });
        }
        let import_window = cx.entity();
        window.open_dialog(cx, move |dialog, _window, app| {
                  Self::build_add_dialog(dialog, &import_window, app)
              });
        cx.update_entity(&name_input, |input, input_cx| {
              input.focus(window, input_cx);
          });
        cx.notify();
    }

    pub(super) fn open_rename_location_dialog(&mut self, window: &mut Window,
                                              cx: &mut Context<Self>) {
        let LocationRef::Saved(id) = self.library.selected
        else {
            return;
        };
        let settings = crate::settings_global::read(cx);
        let Some(saved) = settings.library_locations.iter().find(|l| l.id == id)
        else {
            return;
        };
        self.library_dialog.renaming = Some(id);
        let name_input = self.library_dialog.name_input.clone();
        cx.update_entity(&name_input, |input, input_cx| {
              input.set_value(saved.name.clone(), window, input_cx);
          });
        let import_window = cx.entity();
        window.open_dialog(cx, move |dialog, _window, app| {
                  Self::build_rename_dialog(dialog, &import_window, app)
              });
        cx.update_entity(&name_input, |input, input_cx| {
              input.focus(window, input_cx);
          });
        cx.notify();
    }

    pub(super) fn confirm_remove_location(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let LocationRef::Saved(id) = self.library.selected
        else {
            return;
        };
        let settings = crate::settings_global::read(cx);
        let Some(name) = settings.library_locations
                                 .iter()
                                 .find(|l| l.id == id)
                                 .map(|l| l.name.clone())
        else {
            return;
        };
        let import_window = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let import_window = import_window.clone();
                  alert.title(knot_core::l10n::t("import.library_remove_title"))
                       .description(knot_core::l10n::t_with("import.library_remove_body",
                                                            &[("name", &name)]))
                       .confirm()
                       .on_ok(move |_, window, app| {
                           import_window.update(app, |view, cx| {
                                            view.remove_selected_location(window, cx);
                                        });
                           true
                       })
              });
        cx.notify();
    }

    fn remove_selected_location(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let LocationRef::Saved(id) = self.library.selected
        else {
            return;
        };
        let _ = crate::settings_global::handle(cx).write_persisting(|settings| {
                                                      settings.remove_library_location(id)
                                                  });
        self.library.selected = LocationRef::BuiltIn;
        self.fetch_library_index(window, cx);
    }

    fn build_add_dialog(dialog: Dialog, import_window: &Entity<Self>, app: &mut App) -> Dialog {
        let state_handle = import_window.read(app);
        let name_blank = state_handle.library_dialog
                                     .name_input
                                     .read(app)
                                     .value()
                                     .trim()
                                     .is_empty();
        let kind = state_handle.library_dialog.kind;
        let location_ok = state_handle.library_dialog.location(app).is_some();

        dialog.w(px(480.0))
              .close_button(false)
              .overlay_closable(false)
              .title(knot_core::l10n::t("import.library_dialog_add_title"))
              .content({
                  let import_window = import_window.clone();
                  move |content, _window, app| {
                      content.child(Self::dialog_form(&import_window, kind, app))
                  }
              })
              .footer(Self::dialog_footer(false, name_blank || !location_ok))
              .on_ok({
                  let import_window = import_window.clone();
                  move |_, window, app| Self::confirm_add_dialog(&import_window, window, app)
              })
              .on_cancel(|_, window, app| {
                  window.dispatch_action(Box::new(Cancel), app);
                  true
              })
    }

    fn build_rename_dialog(dialog: Dialog, import_window: &Entity<Self>, app: &mut App) -> Dialog {
        let name_input = import_window.read(app).library_dialog.name_input.clone();
        let name_blank = name_input.read(app).value().trim().is_empty();

        dialog.w(px(420.0))
              .close_button(false)
              .overlay_closable(false)
              .title(knot_core::l10n::t("import.library_dialog_rename_title"))
              .content({
                  let input = name_input.clone();
                  move |content, _window, _app| content.child(Input::new(&input))
              })
              .footer(Self::dialog_footer(true, name_blank))
              .on_ok({
                  let import_window = import_window.clone();
                  move |_, window, app| Self::confirm_rename_dialog(&import_window, window, app)
              })
              .on_cancel(|_, window, app| {
                  window.dispatch_action(Box::new(Cancel), app);
                  true
              })
    }

    /// The add dialog's body: the name field, a kind picker, the kind's own
    /// field, and the trust note.
    fn dialog_form(import_window: &Entity<Self>, kind: DialogLocationKind, app: &mut App)
                   -> impl IntoElement {
        let dialog_state = &import_window.read(app).library_dialog;
        let name_input = dialog_state.name_input.clone();
        let repo_input = dialog_state.repo_input.clone();
        let branch_input = dialog_state.branch_input.clone();
        let web_input = dialog_state.web_input.clone();
        let folder = dialog_state.folder.clone();

        let kind_row = h_flex().gap_2()
                               .child(Self::kind_button("import-library-kind-github",
                                                        "import.library_dialog_kind_github",
                                                        kind == DialogLocationKind::GitHub,
                                                        DialogLocationKind::GitHub,
                                                        import_window.clone()))
                               .child(Self::kind_button("import-library-kind-web",
                                                        "import.library_dialog_kind_web",
                                                        kind == DialogLocationKind::Web,
                                                        DialogLocationKind::Web,
                                                        import_window.clone()))
                               .child(Self::kind_button("import-library-kind-folder",
                                                        "import.library_dialog_kind_folder",
                                                        kind == DialogLocationKind::Folder,
                                                        DialogLocationKind::Folder,
                                                        import_window.clone()));

        let field = match kind {
            DialogLocationKind::GitHub => h_flex().gap_2()
                                                  .child(Input::new(&repo_input))
                                                  .child(Input::new(&branch_input))
                                                  .into_any_element(),
            DialogLocationKind::Web => Input::new(&web_input).into_any_element(),
            DialogLocationKind::Folder => {
                let import_window = import_window.clone();
                h_flex().gap_2()
                        .items_center()
                        .child(div().flex_1().text_sm().child(
                    folder.map(|p| p.display().to_string())
                          .unwrap_or_else(|| knot_core::l10n::t("import.library_dialog_choose_folder")),
                ))
                        .child(Button::new("import-library-choose-folder")
                                   .label(knot_core::l10n::t("import.library_dialog_choose_folder"))
                                   .small()
                                   .on_click(move |_, window, app| {
                                       import_window.update(app, |view, cx| {
                                                        view.choose_library_folder(window, cx);
                                                    });
                                   }))
                        .into_any_element()
            }
        };

        v_flex().gap_3()
                .child(Input::new(&name_input))
                .child(kind_row)
                .child(field)
                .child(div().text_xs()
                            .text_color(app.theme().muted_foreground)
                            .child(knot_core::l10n::t("import.library_dialog_trust_note")))
    }

    fn kind_button(id: &'static str, label_key: &'static str, selected: bool,
                   target: DialogLocationKind, import_window: Entity<Self>)
                   -> Button {
        let mut button = Button::new(id).label(knot_core::l10n::t(label_key)).small();
        button = if selected {
            button.primary()
        }
        else {
            button.ghost()
        };
        button.on_click(move |_, _, app| {
                  import_window.update(app, |view, cx| {
                                   view.library_dialog.kind = target;
                                   cx.notify();
                               });
              })
    }

    fn choose_library_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
                             files:       false,
                             directories: true,
                             multiple:    false,
                             prompt:      Some(knot_core::l10n::t("import.library_dialog_choose_folder")
                                                    .into()),
                         });
        cx.spawn_in(window, async move |this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              let Some(path) = paths.into_iter().next()
              else {
                  return;
              };
              let _ = this.update_in(cx, |view, _window, cx| {
                              view.library_dialog.folder = Some(path);
                              cx.notify();
                          });
          })
          .detach();
    }

    fn dialog_footer(renaming: bool, disabled: bool) -> impl IntoElement {
        DialogFooter::new()
            .child(Button::new("cancel-library-dialog").label(knot_core::l10n::t("import.library_dialog_cancel"))
                                                        .on_click(|_: &ClickEvent, window, app| {
                                                            window.dispatch_action(Box::new(Cancel), app);
                                                        }))
            .child(Button::new("confirm-library-dialog").label(knot_core::l10n::t(if renaming {
                                                            "import.library_dialog_save"
                                                        }
                                                        else {
                                                            "import.library_dialog_add"
                                                        }))
                                                        .primary()
                                                        .disabled(disabled)
                                                        .on_click(|_: &ClickEvent, window, app| {
                                                            window.dispatch_action(Box::new(Confirm { secondary: false }), app);
                                                        }))
    }

    fn confirm_add_dialog(import_window: &Entity<Self>, window: &mut Window, app: &mut App)
                          -> bool {
        let dialog_state = &import_window.read(app).library_dialog;
        let name = dialog_state.name_input.read(app).value().trim().to_string();
        let Some(location) = dialog_state.location(app)
        else {
            return false;
        };
        if name.is_empty() {
            return false;
        }
        let result = crate::settings_global::handle(app).write_persisting(|settings| {
                                                            settings.add_library_location(name,
                                                                                          location)
                                                        });
        let Ok(id) = result
        else {
            return false;
        };
        import_window.update(app, |view, cx| {
                         view.library.selected = LocationRef::Saved(id);
                         view.fetch_library_index(window, cx);
                     });
        true
    }

    fn confirm_rename_dialog(import_window: &Entity<Self>, window: &mut Window, app: &mut App)
                             -> bool {
        let id = import_window.read(app).library_dialog.renaming;
        let Some(id) = id
        else {
            return false;
        };
        let name_input = import_window.read(app).library_dialog.name_input.clone();
        let name = name_input.read(app).value().trim().to_string();
        if name.is_empty() {
            return false;
        }
        let result = crate::settings_global::handle(app).write_persisting(|settings| {
                                                            settings.rename_library_location(id,
                                                                                             name)
                                                        });
        if result.is_err() {
            return false;
        }
        import_window.update(app, |view, cx| {
                         view.redraw_every_window(cx);
                     });
        let _ = window;
        true
    }
}
