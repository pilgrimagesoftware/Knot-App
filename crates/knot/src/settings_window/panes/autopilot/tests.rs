//! The Autopilot tab's API key is a secret (#392; `settings-ui`, "Autopilot
//! tab"): the field masks what it shows and keeps the key it stores exactly
//! as entered.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::Entity;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::component::Root;
use tempfile::TempDir;

use crate::settings_window::SettingsTab;
use crate::settings_window::SettingsWindow;
use crate::settings_window::open_settings_window;

const KEY: &str = "sk-test-0123456789";

/// A settings window on its Autopilot tab, with `KEY` already saved.
fn autopilot_tab(cx: &mut TestAppContext) -> (VisualTestContext, Entity<SettingsWindow>, TempDir) {
    let dir = TempDir::new().expect("a temporary settings root");
    let handle: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    let (window, view) = cx.update(|cx| {
                               gpui_kit::init(cx);
                               let mut settings = knot_core::Settings::with_store_root(dir.path());
                               settings.ai_api_key = KEY.to_owned();
                               crate::settings_global::install(settings, cx);
                               open_settings_window(&handle, cx);
                               let window = handle.borrow().expect("the settings window opened");
                               let view = window.downcast::<Root>()
                                                .expect("a Root")
                                                .read(cx)
                                                .expect("open")
                                                .view()
                                                .clone()
                                                .downcast::<SettingsWindow>()
                                                .expect("the settings view");
                               (window, view)
                           });
    let mut window = VisualTestContext::from_window(window, cx);
    view.update(&mut window, |view, cx| {
            view.selected_tab = SettingsTab::Autopilot;
            cx.notify();
        });
    window.run_until_parked();
    (window, view, dir)
}

#[gpui_kit::test]
fn the_api_key_is_masked_and_stored_as_entered(cx: &mut TestAppContext) {
    let (mut window, view, _dir) = autopilot_tab(cx);

    view.update(&mut window, |view, cx| {
            let input = view.ai_api_key_input.read(cx);
            assert!(input.presentation().is_masked(),
                    "the API key field is drawn masked");
            assert_eq!(input.value().as_ref(),
                       KEY,
                       "masking changes what is drawn, not the value");
            view.save_ai_api_key(cx);
        });

    let stored = cx.update(|cx| crate::settings_global::read(cx).ai_api_key.clone());
    assert_eq!(stored, KEY,
               "saving a masked field stores the key unchanged");
}
