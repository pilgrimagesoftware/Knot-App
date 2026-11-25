use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSFont, NSFontManager};
use objc2_foundation::NSString;
use parking_lot::Mutex;

pub(crate) use super::FontPanelTarget as Target;

/// The row that opened the panel and the font it was opened on - the base
/// `convertFont:` converts from, and the target a choice belongs to.
static SESSION: Mutex<Option<(Target, String, i64)>> = Mutex::new(None);
/// What the user chose, recorded by `changeFont:` and taken by
/// [`poll_selection`].
static CHOSEN: Mutex<Option<(String, f64)>> = Mutex::new(None);

thread_local! {
    /// Owns the receiver for the process's lifetime: the font manager's
    /// target is a weak reference, so dropping this would leave it
    /// dangling. Main-thread-only, like the object itself.
    static RECEIVER: OnceCell<Retained<FontPanelReceiver>> = const { OnceCell::new() };
}

fn size_key(size: f64) -> i64 {
    (size * 100.0).round() as i64
}

define_class!(
    // SAFETY:
    // - `NSObject` has no subclassing requirements.
    // - `FontPanelReceiver` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KnotFontPanelReceiver"]
    struct FontPanelReceiver;

    impl FontPanelReceiver {
        /// The font manager's action, sent when the user changes anything
        /// in the panel. Converting the session's font is what tells the
        /// manager the change was honored; reading `selectedFont` here
        /// instead is explicitly discouraged by AppKit, and returns
        /// nothing useful until after this returns.
        #[unsafe(method(changeFont:))]
        fn change_font(&self, sender: &NSFontManager) {
            let Some((_, family, size)) = SESSION.lock().clone()
            else {
                return;
            };
            let size = size as f64 / 100.0;
            // A family AppKit cannot resolve still has to convert from
            // something real, or `convertFont:` has nothing to apply the
            // user's choice to.
            let base = NSFont::fontWithName_size(&NSString::from_str(&family), size)
                .unwrap_or_else(|| NSFont::systemFontOfSize(size));
            let converted = sender.convertFont(&base);
            let Some(family) = converted.familyName()
            else {
                return;
            };
            *CHOSEN.lock() = Some((family.to_string(), converted.pointSize()));
        }
    }
);

impl FontPanelReceiver {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

/// Opens the system font panel pre-selected to `current_family` at
/// `current_size`, for `target`. No-op off the main thread.
pub fn open(target: Target, current_family: &str, current_size: f64) {
    let Some(mtm) = MainThreadMarker::new()
    else {
        return;
    };
    *SESSION.lock() = Some((target, current_family.to_string(), size_key(current_size)));
    *CHOSEN.lock() = None;
    let manager = NSFontManager::sharedFontManager(mtm);
    RECEIVER.with(|receiver| {
                let receiver = receiver.get_or_init(|| FontPanelReceiver::new(mtm));
                // SAFETY: the receiver outlives the process, so the
                // manager's unowned reference to it stays valid.
                unsafe { manager.setTarget(Some(receiver as &AnyObject)) };
            });
    if let Some(font) = NSFont::fontWithName_size(&NSString::from_str(current_family), current_size)
    {
        manager.setSelectedFont_isMultiple(&font, false);
    }
    if let Some(panel) = manager.fontPanel(true) {
        panel.makeKeyAndOrderFront(None);
    }
}

/// Takes the choice `changeFont:` last recorded, as
/// `(target, family, size)`, leaving it as the base the next conversion
/// starts from. `None` until the user changes something in the panel.
pub fn poll_selection() -> Option<(Target, String, f64)> {
    let (target, ..) = SESSION.lock().clone()?;
    let (family, size) = CHOSEN.lock().take()?;
    *SESSION.lock() = Some((target, family.clone(), size_key(size)));
    Some((target, family, size))
}
