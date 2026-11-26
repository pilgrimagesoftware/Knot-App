//! The banner that asks whether to trust a repository whose own git
//! configuration names programs (`crate::repository_trust`).
//!
//! Knot runs no git in such a repository until the user answers, so the
//! banner names the repository and every configured program, written as
//! configured, with control characters made visible - what the user agrees
//! to is exactly what would run. It draws the registry's first queued
//! repository; the registry is in memory, so drawing it reads no file.

use gpui_kit::AnyElement;
use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use knot_git::RepositoryPrograms;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The banner for the next repository waiting to be trusted, if any.
    pub(in crate::workspace_window) fn trust_banner(&self, cx: &mut Context<Self>)
                                                    -> Option<AnyElement> {
        let programs = self.repository_trust.as_ref()?.next_pending()?;
        let message = knot_core::l10n::t_with("repository_trust.banner",
                                              &[("repository",
                                                 &programs.repository.to_string_lossy())]);
        let mono = cx.theme().mono_font_family.clone();
        let settings = v_flex().gap_0p5()
                               .font_family(mono)
                               .text_xs()
                               .text_color(cx.theme().muted_foreground)
                               .children(programs.settings
                                                 .iter()
                                                 .map(|setting| div().child(setting.to_string())));
        let trusted = programs.clone();
        let declined = programs.repository.clone();
        Some(h_flex().id("repository-trust-banner")
                     .debug_selector(|| "repository-trust-banner".into())
                     .flex_shrink_0()
                     .w_full()
                     .gap_2()
                     .px_3()
                     .py_1p5()
                     .items_start()
                     .border_b_1()
                     .border_color(cx.theme().border)
                     .bg(cx.theme().muted)
                     .text_sm()
                     .child(Icon::new(IconName::ShieldAlert).small()
                                                            .text_color(cx.theme().warning))
                     .child(v_flex().flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(div().child(message))
                                    .child(settings))
                     .child(Button::new("repository-trust-accept")
                            .label(knot_core::l10n::t("repository_trust.trust"))
                            .small()
                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                view.trust_repository(&trusted, cx);
                            })))
                     .child(Button::new("repository-trust-decline")
                            .label(knot_core::l10n::t("repository_trust.not_now"))
                            .ghost()
                            .small()
                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                if let Some(registry) = &view.repository_trust {
                                    registry.decline(&declined);
                                }
                                cx.notify();
                            })))
                     .into_any_element())
    }

    /// Trusts `programs` for this session and saves it. A save that fails
    /// leaves the trust in place until Knot quits, and is logged.
    fn trust_repository(&mut self, programs: &RepositoryPrograms, cx: &mut Context<Self>) {
        let Some(registry) = &self.repository_trust
        else {
            return;
        };
        let entry = registry.trust(programs);
        let saved = crate::settings_global::write_persisting(cx, |settings| {
            settings.trusted_repositories
                    .retain(|existing| existing.path != entry.path);
            settings.trusted_repositories.push(entry.clone());
            settings.persist_preferences()
        });
        if let Err(error) = saved {
            eprintln!("saving trust for {} failed: {error}", entry.path);
        }
        cx.notify();
    }

    /// Whether the trust queue changed since this window last looked. Every
    /// window keeps its own mark, so each one repaints for a change.
    pub(in crate::workspace_window) fn trust_queue_moved(&mut self) -> bool {
        let Some(registry) = &self.repository_trust
        else {
            return false;
        };
        let generation = registry.generation();
        let moved = generation != self.trust_generation_drawn;
        self.trust_generation_drawn = generation;
        moved
    }
}

#[cfg(test)]
mod tests;
