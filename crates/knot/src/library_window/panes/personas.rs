use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::div;
use gpui_kit::px;
use knot_library::ContributionKind;
use uuid::Uuid;

use super::super::persona_editor::open_persona_editor;
use super::super::submit::{Submission, open_submit_dialog};
use crate::library_window::LibraryWindow;

/// An agent a persona is assigned to (#365).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersonaUser {
    pub(crate) id:   Uuid,
    pub(crate) name: String,
}

impl LibraryWindow {
    /// Truncates `instructions` to `max_chars`, appending an ellipsis when
    /// truncated so a persona list row stays a single line.
    pub(crate) fn persona_preview(instructions: &str, max_chars: usize) -> String {
        let truncated: String = instructions.chars().take(max_chars).collect();
        if instructions.chars().count() > max_chars {
            format!("{truncated}…")
        }
        else {
            truncated
        }
    }

    /// The agents each persona is assigned to, in roster order. A persona
    /// still in use can't be deleted: the agents referencing it would keep a
    /// `persona_id` pointing at nothing, which reads as "no persona"
    /// everywhere without ever saying the instructions were dropped. Naming
    /// them, rather than counting them, is what lets the user find and
    /// change them (#365).
    pub(crate) fn persona_users(agents: &[knot_agents::Agent]) -> BTreeMap<Uuid, Vec<PersonaUser>> {
        let mut users: BTreeMap<Uuid, Vec<PersonaUser>> = BTreeMap::new();
        for agent in agents {
            if let Some(persona) = agent.persona_id {
                users.entry(persona)
                     .or_default()
                     .push(PersonaUser { id:   agent.id,
                                         name: agent.name.clone(), });
            }
        }
        users
    }

    /// `persona_users` against the live store - the window's own
    /// `Settings` snapshot is from the moment it opened and misses agents
    /// created since.
    fn live_persona_users(&self) -> BTreeMap<Uuid, Vec<PersonaUser>> {
        Self::persona_users(self.store.lock().agents())
    }

    /// The delete button's tooltip, which doubles as the reason it is
    /// disabled when the persona is assigned to agents: it names them.
    pub(crate) fn persona_delete_tooltip(users: &[PersonaUser]) -> String {
        if users.is_empty() {
            return knot_core::l10n::t("settings.personas.delete");
        }
        let names: Vec<&str> = users.iter().map(|user| user.name.as_str()).collect();
        knot_core::l10n::t_with("settings.personas.in_use_by",
                                &[("agents", &names.join(", "))])
    }

    /// Opens agent `id`'s workspace window with it selected, or raises it -
    /// a persona's "Used by" link (#365).
    fn show_agent(&self, id: Uuid, cx: &mut Context<Self>) {
        crate::workspace_window::WorkspaceWindow::open_agent(Arc::clone(&self.store),
                                                             Arc::clone(&self.messages),
                                                             id,
                                                             crate::workspace_window::OpenRoute::Library,
                                                             cx);
    }

    fn delete_persona(&mut self, id: Uuid, cx: &mut Context<Self>) {
        // Checked again here, not just on the button: the dialog that
        // gets us here is opened from a rendered row, and an agent could
        // have taken the persona in between.
        if self.live_persona_users().contains_key(&id) {
            cx.notify();
            return;
        }
        if let Err(error) =
            crate::settings_global::write_persisting(cx, |settings| settings.remove_persona(id))
        {
            eprintln!("failed to remove persona: {error}");
        }
        cx.notify();
    }

    fn restore_default_personas(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = crate::settings_global::write_persisting(cx, |settings| {
            settings.restore_default_personas()
        }) {
            eprintln!("failed to restore default personas: {error}");
        }
        cx.notify();
    }

    /// One persona's row: its name, a preview of its instructions, the
    /// agents using it, and the edit and delete actions.
    ///
    /// `users` are the agents referencing it, which is what disables delete -
    /// the button stays visible and names them in its tooltip rather than
    /// disappearing - and each is a link to that agent.
    fn persona_row(&self, persona: knot_core::Persona, index: usize, users: Vec<PersonaUser>,
                   cx: &mut Context<Self>)
                   -> impl IntoElement + use<> {
        let library_window = cx.entity();
        let id = persona.id;
        let preview = Self::persona_preview(&persona.instructions, 80);
        let in_use = !users.is_empty();
        let tooltip = Self::persona_delete_tooltip(&users);
        let muted = cx.theme().muted_foreground;
        let used_by = in_use.then(|| {
                                h_flex().flex_wrap()
                                        .items_center()
                                        .gap_1()
                                        .text_sm()
                                        .child(div().text_color(muted)
                                                    .child(knot_core::l10n::t("settings.personas.used_by")))
                                        .children(users.into_iter().enumerate().map(|(n, user)| {
                                            let library_window = library_window.clone();
                                            Button::new(gpui_kit::ElementId::Name(format!("persona-{index}-user-{n}").into()))
                                                .label(user.name)
                                                .link()
                                                .xsmall()
                                                .tooltip(knot_core::l10n::t("settings.personas.show_agent"))
                                                .debug_selector(move || format!("persona-{index}-user-{n}"))
                                                .on_click(move |_, _, app| {
                                                    library_window.update(app, |view, cx| {
                                                        view.show_agent(user.id, cx)
                                                    });
                                                })
                                        }))
                            });
        h_flex()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(div().child(persona.name.clone()))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(preview),
                                    )
                                    .children(used_by),
                            )
                            .child(
                                h_flex()
                                    .flex_shrink_0()
                                    .gap_1()
                                    .child(
                                        crate::controls::icon_button(
                                            ("persona-submit", index),
                                            "icons/git-pull-request-arrow.svg",
                                            knot_core::l10n::t("library.submit.persona_tooltip"),
                                            false,
                                        )
                                        .debug_selector(move || format!("persona-submit-{index}"))
                                        .on_click({
                                            let submission = Submission {
                                                kind: ContributionKind::Persona,
                                                id,
                                                name: persona.name.clone(),
                                                body: persona.instructions.clone(),
                                                description: None,
                                            };
                                            move |_, window, app| {
                                                open_submit_dialog(submission.clone(), window, app)
                                            }
                                        }),
                                    )
                                    .child(
                                        crate::controls::icon_button(
                                            ("persona-edit", index),
                                            "icons/pencil.svg",
                                            knot_core::l10n::t("settings.personas.edit"),
                                            false,
                                        )
                                        .on_click({
                                            let parent = library_window.downgrade();
                                            let persona = persona.clone();
                                            move |_, _, app| {
                                                open_persona_editor(
                                                    parent.clone(),
                                                    Some(persona.clone()),
                                                    app,
                                                );
                                            }
                                        }),
                                    )
                                    .child(
                                        // Disabled rather than hidden while
                                        // agents still reference it, with the
                                        // count as the tooltip so the button
                                        // says why it won't work.
                                        crate::controls::icon_button(
                                            ("persona-delete", index),
                                            "icons/trash.svg",
                                            tooltip,
                                            true,
                                        )
                                        .disabled(in_use)
                                        .on_click({
                                            let library_window = library_window.clone();
                                            let name = persona.name.clone();
                                            move |_, window, app| {
                                                let library_window = library_window.clone();
                                                window.open_alert_dialog(app, {
                                                    let name = name.clone();
                                                    move |alert, _, _| {
                                                        let library_window =
                                                            library_window.clone();
                                                        alert
                                                        .title(knot_core::l10n::t("settings.personas.delete_persona"))
                                                        .description(knot_core::l10n::t_with(
                                                            "settings.personas.delete_body",
                                                            &[("name", &name)],
                                                        ))
                                                        .confirm()
                                                        .on_ok(move |_, _, app| {
                                                            library_window.update(app, |view, cx| {
                                                                view.delete_persona(id, cx);
                                                            });
                                                            true
                                                        })
                                                    }
                                                });
                                            }
                                        }),
                                    ),
                            )
    }

    pub(crate) fn render_personas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let library_window = cx.entity();
        let personas: Vec<knot_core::Persona> = crate::settings_global::read(cx).active_personas()
                                                                                .into_iter()
                                                                                .cloned()
                                                                                .collect();

        let mut users = self.live_persona_users();

        let list = if personas.is_empty() {
            div().text_sm()
                 .text_color(cx.theme().muted_foreground)
                 .child(knot_core::l10n::t("settings.personas.none_defined"))
                 .into_any_element()
        }
        else {
            v_flex().gap_3()
                    .children(personas.into_iter().enumerate().map(|(index, persona)| {
                                                                  let using =
                                                                      users.remove(&persona.id)
                                                                           .unwrap_or_default();
                                                                  self.persona_row(persona, index,
                                                                                   using, cx)
                                                              }))
                    .into_any_element()
        };
        // Bounded so the list scrolls in place instead of pushing the group's
        // title/action row (which must stay visible) off the top of the
        // window - same cap philosophy as the window's own per-pane height.
        let list = crate::capped_scroll::capped_scroll("personas-list", px(420.), list);

        v_flex().gap_3().child(
            crate::controls::group(knot_core::l10n::t("settings.personas.personas"))
                .child(
                    // Grouped at the left like the Prompts pane's toolbar,
                    // not spread across the group (#300).
                    h_flex()
                        .gap_1()
                        .child(
                            crate::controls::icon_button(
                                "personas-add",
                                "icons/plus.svg",
                                knot_core::l10n::t("settings.personas.add"),
                                false,
                            )
                            .debug_selector(|| "personas-add".into())
                            .on_click({
                                let parent = library_window.downgrade();
                                move |_, _, app| {
                                    open_persona_editor(parent.clone(), None, app);
                                }
                            }),
                        )
                        .child(
                            crate::controls::icon_button(
                                "personas-restore-defaults",
                                "icons/rotate-ccw.svg",
                                knot_core::l10n::t("settings.personas.restore_defaults"),
                                false,
                            )
                            .debug_selector(|| "personas-restore-defaults".into())
                            .on_click({
                                let library_window = library_window.clone();
                                move |_, window, app| {
                                    let library_window = library_window.clone();
                                    window.open_alert_dialog(app, move |alert, _, _| {
                                        let library_window = library_window.clone();
                                        alert
                                            .title(knot_core::l10n::t("settings.personas.restore_defaults"))
                                            .description(
                                                knot_core::l10n::t("settings.personas.restore_defaults_body"),
                                            )
                                            .confirm()
                                            .on_ok(move |_, _, app| {
                                                library_window.update(app, |view, cx| {
                                                    view.restore_default_personas(cx);
                                                });
                                                true
                                            })
                                    });
                                }
                            }),
                        )
                        .child(
                            crate::controls::icon_button(
                                "personas-import-from-library",
                                "icons/download.svg",
                                knot_core::l10n::t("settings.personas.import_from_library"),
                                false,
                            )
                            .debug_selector(|| "personas-import-from-library".into())
                            .on_click(|_, window, app| {
                                window.dispatch_action(Box::new(crate::app_bootstrap::OpenImport), app);
                            }),
                        ),
                )
                .child(list),
        )
    }
}
