use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::PathPromptOptions;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::WindowExt;
use gpui_kit::component::input::Input;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::div;

use crate::adapter_updates::AdapterUpdates;
use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    /// The name to show for an agent type.
    ///
    /// The roster in `knot_core::agent_type` answers this, so a type added
    /// there is named everywhere at once. An id this build does not know
    /// shows as itself rather than as Claude: the point of #224 is that a
    /// value nothing recognizes must not be indistinguishable from the
    /// default.
    pub(crate) fn agent_type_label(agent_type: &str) -> &str {
        knot_core::agent_type::label(agent_type)
    }

    /// A representative icon per agent type, so a sidebar row is
    /// identifiable at a glance rather than by its one-character avatar.
    ///
    /// Picked to echo each vendor's own mark where the icon set has one
    /// (Claude's asterisk, Gemini's sparkle, GitHub's for Copilot). Keyed
    /// on the same ids as `knot_core::agent_type`'s roster; the fallback
    /// covers the user's own custom commands, which have no mark to echo,
    /// and any type this build does not know.
    ///
    /// Uses the full `gpui_kit::assets` (Lucide) set rather than GPUI
    /// Component's smaller built-in `IconName`, which has no brace,
    /// terminal or sparkle glyph.
    pub(crate) fn agent_type_icon(agent_type: &str) -> gpui_kit::assets::IconName {
        use gpui_kit::assets::IconName;
        match agent_type {
            "claude" => IconName::Asterisk,
            "codex" => IconName::Braces,
            "opencode" => IconName::Terminal,
            "gemini" => IconName::Sparkles,
            "copilot" => IconName::Github,
            "antigravity" => IconName::Rocket,
            "shell" => IconName::SquareTerminal,
            _ => IconName::Bot,
        }
    }

    fn choose_source_folder(&mut self, cx: &mut Context<Self>) {
        let receiver =
            cx.prompt_for_paths(PathPromptOptions { files:       false,
                                                    directories: true,
                                                    multiple:    false,
                                                    prompt:
                                                        Some("Choose Source Folder".into()), });
        let settings_window = cx.entity();
        cx.spawn(async move |_this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              let Some(path) = paths.into_iter().next()
              else {
                  return;
              };
              cx.update(|app| {
                    settings_window.update(app, |view, cx| {
                                       crate::settings_global::write(cx, |settings| {
                                           settings.source_base_folder =
                                               path.to_string_lossy().into_owned();
                                       });
                                       view.persist(cx);
                                       cx.notify();
                                   });
                });
          })
          .detach();
    }

    fn clear_source_folder(&mut self, cx: &mut Context<Self>) {
        crate::settings_global::write(cx, |settings| settings.source_base_folder.clear());
        self.persist(cx);
        cx.notify();
    }

    fn select_agent_type(&mut self, agent_type: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_agent_type = agent_type.to_string();
        let value = crate::settings_global::read(cx).agent_options
                                                    .get(agent_type)
                                                    .cloned()
                                                    .unwrap_or_default();
        cx.update_entity(&self.agent_options_input, |input, input_cx| {
              input.set_value(value, window, input_cx);
          });
        cx.notify();
    }

    pub(crate) fn save_agent_options(&mut self, cx: &mut Context<Self>) {
        let value = self.agent_options_input.read(cx).value().to_string();
        let agent_type = self.selected_agent_type.clone();
        crate::settings_global::write(cx, |settings| {
            settings.agent_options
                    .insert(agent_type.clone(), value.clone());
        });
        self.persist(cx);
    }

    /// One tab per agent type - every type but the user's own custom
    /// commands, which have no options or adapter here (the Swift
    /// reference's `availableAgents` list, read off the roster). A tab
    /// carries a dot while its adapter has an update waiting.
    fn render_agent_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let kinds: Vec<_> = knot_core::agent_type::ALL.iter()
                                                      .filter(|kind| !kind.is_custom)
                                                      .collect();
        let selected = kinds.iter()
                            .position(|kind| kind.id == self.selected_agent_type)
                            .unwrap_or(0);
        let updates = cx.try_global::<AdapterUpdates>();
        let tabs: Vec<Tab> =
            kinds.iter()
                 .map(|kind| {
                     let tab = Tab::new().label(kind.label);
                     if updates.is_some_and(|updates| updates.update_available(kind.id)) {
                         tab.suffix(super::coding_adapter::update_dot(cx))
                     }
                     else {
                         tab
                     }
                 })
                 .collect();
        let ids: Vec<&'static str> = kinds.iter().map(|kind| kind.id).collect();
        let settings_window = cx.entity();
        TabBar::new("coding-agent-tabs").selected_index(selected)
                                        .children(tabs)
                                        .on_click(move |index, window, app| {
                                            let Some(id) = ids.get(*index).copied()
                                            else {
                                                return;
                                            };
                                            settings_window.update(app, |view, cx| {
                                                               view.select_agent_type(id, window,
                                                                                      cx);
                                                           });
                                        })
    }

    pub(crate) fn render_coding(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let source_base_folder = crate::settings_global::read(cx).source_base_folder.clone();
        let folder_label = if source_base_folder.is_empty() {
            "Not configured".to_string()
        }
        else {
            source_base_folder
        };
        let agent_type_label = Self::agent_type_label(&self.selected_agent_type);

        v_flex()
            .gap_3()
            .child(
                crate::controls::group(knot_core::l10n::t("settings.coding.source_folder")).child(
                    Self::row(
                        knot_core::l10n::t("settings.coding.folder"),
                        h_flex()
                            .flex_1()
                            .justify_between()
                            .child(div().text_sm().child(folder_label))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        crate::controls::icon_button(
                                            "coding-choose-source-folder",
                                            "icons/folder-open.svg",
                                            "Choose source folder",
                                            false,
                                        )
                                        .on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.choose_source_folder(cx);
                                                })
                                            }
                                        }),
                                    )
                                    .child(
                                        crate::controls::icon_button(
                                            "coding-clear-source-folder",
                                            "icons/x.svg",
                                            "Clear source folder",
                                            true,
                                        )
                                        .on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, window, app| {
                                                let settings_window = settings_window.clone();
                                                window.open_alert_dialog(
                                                    app,
                                                    move |alert, _, _| {
                                                        let settings_window =
                                                            settings_window.clone();
                                                        alert
                                                    .title(knot_core::l10n::t(
                                                        "settings.coding.clear_source_folder",
                                                    ))
                                                    .description(knot_core::l10n::t(
                                                        "settings.coding.clear_source_folder_body",
                                                    ))
                                                    .confirm()
                                                    .on_ok(move |_, _, app| {
                                                        settings_window.update(app, |view, cx| {
                                                            view.clear_source_folder(cx);
                                                        });
                                                        true
                                                    })
                                                    },
                                                );
                                            }
                                        }),
                                    ),
                            ),
                    ),
                ),
            )
            .child(self.render_agent_tabs(cx))
            .child(
                // One group per agent: its options and its adapter together,
                // named for the agent the tabs above select (#367).
                crate::controls::group(agent_type_label.to_string())
                    .child(Self::row(
                        knot_core::l10n::t("settings.coding.options"),
                        Input::new(&self.agent_options_input).flex_1(),
                    ))
                    .children(self.adapter_rows(cx)),
            )
    }
}

#[cfg(test)]
mod tests;
