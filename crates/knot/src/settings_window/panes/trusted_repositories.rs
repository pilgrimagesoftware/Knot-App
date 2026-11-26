//! The Coding tab's "Trusted repositories" group: every repository the user
//! has trusted to run the programs its git settings name, each with a way to
//! withdraw that trust (`crate::repository_trust`).

use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::WindowExt;
use gpui_kit::div;

use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    pub(super) fn render_trusted_repositories(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let trusted = crate::settings_global::read(cx).trusted_repositories
                                                      .clone();
        let settings_window = cx.entity();
        let rows = trusted.into_iter().enumerate().map(|(index, entry)| {
            let path = entry.path.clone();
            let settings_window = settings_window.clone();
            h_flex().gap_2()
                    .items_center()
                    .child(Self::mono_text(cx, entry.path.clone()).flex_1().min_w_0())
                    .child(crate::controls::icon_button(("trusted-repository-revoke", index),
                                                        "icons/x.svg",
                                                        knot_core::l10n::t("repository_trust.revoke"),
                                                        true)
                           .on_click(move |_, window, app| {
                               let settings_window = settings_window.clone();
                               let path = path.clone();
                               let question =
                                   knot_core::l10n::t_with("repository_trust.revoke_confirm",
                                                           &[("repository", &path)]);
                               window.open_alert_dialog(app, move |alert, _, _| {
                                   let settings_window = settings_window.clone();
                                   let path = path.clone();
                                   alert.title(knot_core::l10n::t("repository_trust.revoke"))
                                        .description(question.clone())
                                        .confirm()
                                        .on_ok(move |_, _, app| {
                                            settings_window.update(app, |view, cx| {
                                                               view.revoke_trust(&path, cx);
                                                           });
                                            true
                                        })
                               });
                           }))
        });
        let rows: Vec<_> = rows.collect();
        let list = if rows.is_empty() {
            v_flex().child(div().text_sm()
                                .child(knot_core::l10n::t("repository_trust.settings_empty")))
        }
        else {
            v_flex().gap_1().children(rows)
        };
        crate::controls::group(knot_core::l10n::t("repository_trust.settings_group"))
            .child(Self::row("", list.flex_1().min_w_0()))
            .child(Self::hint(cx, knot_core::l10n::t("repository_trust.settings_hint")))
    }

    /// Withdraws trust for the repository at `path`, for this session and in
    /// the saved settings.
    fn revoke_trust(&mut self, path: &str, cx: &mut Context<Self>) {
        if let Some(registry) = crate::repository_trust::registry() {
            registry.revoke(path);
        }
        let saved = crate::settings_global::write_persisting(cx, |settings| {
            settings.trusted_repositories
                    .retain(|entry| entry.path != path);
            settings.persist_preferences()
        });
        if let Err(error) = saved {
            eprintln!("saving trust removal for {path} failed: {error}");
        }
        cx.notify();
    }
}
