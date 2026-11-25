use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

/// No-op off the main thread.
pub fn open() {
    let Some(mtm) = MainThreadMarker::new()
    else {
        return;
    };
    NSApplication::sharedApplication(mtm).orderFrontCharacterPalette(None);
}
