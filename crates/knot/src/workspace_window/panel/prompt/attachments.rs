//! Attaching context to a pending message: the native file picker, an
//! image pasted off the clipboard, and removing an attached path.

use gpui_kit::App;
use gpui_kit::ClipboardEntry;
use gpui_kit::Context;
use gpui_kit::ImageFormat;
use gpui_kit::PathPromptOptions;
use gpui_kit::Window;
use uuid::Uuid;

use crate::composer_style::Palette;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Opens the native file/image picker and attaches the chosen paths to
    /// `id`'s pending message, per `knot-ui-conventions`' native-picker
    /// rule (`cx.prompt_for_paths` over an in-app file browser).
    pub(in crate::workspace_window) fn add_panel_context(&mut self, id: Uuid,
                                                         cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions { files:       true,
                                                             directories: false,
                                                             multiple:    true,
                                                             prompt:      Some("Attach".into()), });
        let this = cx.entity();
        cx.spawn(async move |_this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              cx.update(|app| {
                    this.update(app, |view, cx| {
                            for path in paths {
                                view.panel_pending_context
                                    .entry(id)
                                    .or_default()
                                    .push(path.clone());
                                view.queue_attachment_reference(id, path);
                            }
                            view.restyle_panel_attachments(id, Palette::of(cx), cx);
                            cx.notify();
                        });
                });
          })
          .detach();
    }

    /// Reads an image off the system clipboard (e.g. a pasted screenshot)
    /// and attaches it to `id`'s pending message the same way
    /// `add_panel_context` attaches a picked file, since ACP's
    /// `session/prompt` here only carries text plus attached paths - see
    /// `render_panel_input_area`'s `capture_action::<Paste>` wiring.
    /// Returns `false` (leaving the paste to the textarea's own text-paste
    /// handling) when the clipboard holds no image.
    pub(in crate::workspace_window) fn paste_clipboard_image_context(&mut self, id: Uuid,
                                                                     cx: &mut App)
                                                                     -> bool {
        let Some(item) = cx.read_from_clipboard()
        else {
            return false;
        };
        let mut attached = false;
        for entry in item.entries {
            let ClipboardEntry::Image(image) = entry
            else {
                continue;
            };
            let extension = match image.format {
                ImageFormat::Png => "png",
                ImageFormat::Jpeg => "jpg",
                ImageFormat::Webp => "webp",
                ImageFormat::Gif => "gif",
                ImageFormat::Svg => "svg",
                ImageFormat::Bmp => "bmp",
                _ => "png",
            };
            let path = std::env::temp_dir().join(format!("knot-paste-{}.{extension}", image.id));
            if std::fs::write(&path, &image.bytes).is_ok() {
                self.panel_pending_context
                    .entry(id)
                    .or_default()
                    .push(path.clone());
                self.queue_attachment_reference(id, path);
                attached = true;
            }
        }
        if attached {
            self.restyle_panel_attachments(id, Palette::of(cx), cx);
        }
        attached
    }

    /// Removes one attached path from `id`'s pending context by index.
    /// Deletes the chip rather than the row: the deletion reports as an
    /// ordinary edit, and the reconciliation that follows every edit is
    /// what drops the row. One direction, so the strip and the buffer
    /// cannot disagree about which of them was right.
    ///
    /// The row is removed directly only when there is no chip to delete -
    /// a buffer that never received one, which is what an attachment
    /// dismissed before its deferred insertion ran looks like.
    pub(in crate::workspace_window) fn remove_panel_context(&mut self, id: Uuid, index: usize,
                                                            window: &mut Window, cx: &mut App) {
        let Some(path) = self.panel_pending_context
                             .get(&id)
                             .and_then(|paths| paths.get(index))
                             .cloned()
        else {
            return;
        };
        if self.remove_attachment_reference(id, &path, window, cx) {
            return;
        }
        if let Some(paths) = self.panel_pending_context.get_mut(&id)
           && index < paths.len()
        {
            paths.remove(index);
        }
        if let Some(queued) = self.panel_pending_attachments.get_mut(&id) {
            queued.retain(|waiting| waiting != &path);
        }
        self.restyle_panel_attachments(id, Palette::of(cx), cx);
    }
}
