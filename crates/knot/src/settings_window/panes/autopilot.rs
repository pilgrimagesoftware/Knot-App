use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use knot_core::AiProvider;
use knot_core::AutopilotAction;

use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    pub(crate) fn ai_provider_label(provider: AiProvider) -> &'static str {
        match provider {
            AiProvider::OpenAi => "OpenAI",
            AiProvider::Anthropic => "Anthropic",
            AiProvider::Google => "Google",
        }
    }

    /// The catalog keys for `action`'s picker label and its description.
    ///
    /// Exhaustive: a new [`AutopilotAction`] fails to compile here until it
    /// has both, and `settings_labels` checks every one resolves.
    pub(crate) fn autopilot_action_keys(action: AutopilotAction) -> (&'static str, &'static str) {
        match action {
            AutopilotAction::Mark => {
                ("settings.autopilot.action.mark", "settings.autopilot.action.mark_description")
            }
            AutopilotAction::Ask => {
                ("settings.autopilot.action.ask", "settings.autopilot.action.ask_description")
            }
            AutopilotAction::Continue => ("settings.autopilot.action.continue",
                                          "settings.autopilot.action.continue_description"),
            AutopilotAction::Custom => {
                ("settings.autopilot.action.custom", "settings.autopilot.action.custom_description")
            }
        }
    }

    pub(crate) fn autopilot_action_label(action: AutopilotAction) -> String {
        knot_core::l10n::t(Self::autopilot_action_keys(action).0)
    }

    pub(crate) fn autopilot_action_description(action: AutopilotAction) -> String {
        knot_core::l10n::t(Self::autopilot_action_keys(action).1)
    }

    fn select_ai_provider(&mut self, provider: AiProvider, cx: &mut Context<Self>) {
        crate::settings_global::write(cx, |settings| settings.ai_provider = provider);
        self.persist(cx);
        cx.notify();
    }

    fn select_autopilot_action(&mut self, action: AutopilotAction, cx: &mut Context<Self>) {
        crate::settings_global::write(cx, |settings| settings.autopilot_action = action);
        self.persist(cx);
        cx.notify();
    }

    pub(crate) fn save_ai_api_key(&mut self, cx: &mut Context<Self>) {
        let key = self.ai_api_key_input.read(cx).value().to_string();
        crate::settings_global::write(cx, |settings| settings.ai_api_key = key.clone());
        self.persist(cx);
    }

    pub(crate) fn save_autopilot_custom_prompt(&mut self, cx: &mut Context<Self>) {
        let prompt = self.autopilot_custom_prompt_input
                         .read(cx)
                         .value()
                         .to_string();
        crate::settings_global::write(cx, |settings| {
            settings.autopilot_custom_prompt = prompt.clone();
        });
        self.persist(cx);
    }

    pub(crate) fn render_autopilot(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let autopilot_enabled = crate::settings_global::read(cx).autopilot_enabled;
        let ai_provider = crate::settings_global::read(cx).ai_provider;
        let autopilot_action = crate::settings_global::read(cx).autopilot_action;
        let provider_label = Self::ai_provider_label(ai_provider);
        let model_name = knot_autopilot::ai_model_for(ai_provider);
        let action_label = Self::autopilot_action_label(autopilot_action);
        let is_custom_action = autopilot_action == AutopilotAction::Custom;

        v_flex()
            .gap_3()
            .child(
                crate::controls::group(knot_core::l10n::t("settings.autopilot.enable_group"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.autopilot.enable"),
                        Switch::new("autopilot-enabled")
                            .checked(autopilot_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, cx| {
                                        crate::settings_global::write(cx, |settings| {
                                            settings.autopilot_enabled = checked;
                                        });
                                        view.persist(cx);
                                    })
                                }
                            }),
                    ))
                    .child(Self::hint(
                        cx,
                        knot_core::l10n::t("settings.autopilot.blurb"),
                    )),
            )
            .child(
                crate::controls::group(knot_core::l10n::t("settings.autopilot.provider_group"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.autopilot.provider"),
                        // Driven off `AiProvider::ALL`, so a new provider
                        // reaches the picker with its variant rather than a
                        // second list.
                        Self::dropdown(
                            "autopilot-provider-picker",
                            provider_label,
                            AiProvider::ALL
                                .iter()
                                .copied()
                                .map(|value| (Self::ai_provider_label(value).into(), value))
                                .collect(),
                            settings_window.clone(),
                            |view, value, _, cx| view.select_ai_provider(*value, cx),
                        ),
                    ))
                    .child(Self::row(
                        knot_core::l10n::t("settings.autopilot.api_key"),
                        Input::new(&self.ai_api_key_input)
                            .font_family(cx.theme().mono_font_family.clone())
                            .flex_1(),
                    ))
                    .child(Self::text_row(
                        knot_core::l10n::t("settings.autopilot.model"),
                        Self::mono_text(cx, model_name).text_color(cx.theme().muted_foreground),
                    )),
            )
            .child(
                crate::controls::group(knot_core::l10n::t("settings.autopilot.action_group"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.autopilot.on_input"),
                        Self::dropdown(
                            "autopilot-action-picker",
                            action_label,
                            AutopilotAction::ALL
                                .iter()
                                .copied()
                                .map(|value| (Self::autopilot_action_label(value).into(), value))
                                .collect(),
                            settings_window.clone(),
                            |view, value, _, cx| {
                                view.select_autopilot_action(*value, cx);
                            },
                        ),
                    ))
                    .children(is_custom_action.then(|| {
                        Self::row(
                            knot_core::l10n::t("settings.autopilot.custom_prompt"),
                            Input::new(&self.autopilot_custom_prompt_input).flex_1(),
                        )
                        .into_any_element()
                    }))
                    .children((!is_custom_action).then(|| {
                        Self::hint(cx, Self::autopilot_action_description(autopilot_action))
                            .into_any_element()
                    })),
            )
    }
}
