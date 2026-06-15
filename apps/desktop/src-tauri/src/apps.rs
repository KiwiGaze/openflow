//! Frontmost-app detection for the opt-in history row's app name. Best-effort:
//! a read failure just means the history row has no app name — dictation never
//! blocks.

/// The frontmost application's `(bundle_id, display_name)`, or None.
#[cfg(target_os = "macos")]
pub fn frontmost_app() -> Option<(String, String)> {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    // SAFETY: NSWorkspace is registered (AppKit is linked by the webview), and
    // the autorelease pool keeps the returned objects alive while we read them.
    unsafe fn nsstring(value: *mut AnyObject) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let utf8: *const c_char = msg_send![value, UTF8String];
        if utf8.is_null() {
            return None;
        }
        Some(CStr::from_ptr(utf8).to_string_lossy().into_owned())
    }

    objc2::rc::autoreleasepool(|_| unsafe {
        let workspace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace.is_null() {
            return None;
        }
        let app: *mut AnyObject = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            return None;
        }
        let bundle: *mut AnyObject = msg_send![app, bundleIdentifier];
        let bundle_id = nsstring(bundle)?;
        let name: *mut AnyObject = msg_send![app, localizedName];
        let display = nsstring(name).unwrap_or_else(|| bundle_id.clone());
        Some((bundle_id, display))
    })
}

#[cfg(not(target_os = "macos"))]
pub fn frontmost_app() -> Option<(String, String)> {
    None
}
