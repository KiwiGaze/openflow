//! macOS permission checks: Accessibility (synthetic keystrokes) and
//! Microphone (capture). Other platforms report permissive defaults so the
//! codebase stays compilable cross-platform.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsState {
    pub microphone: &'static str,
    pub accessibility: bool,
}

pub fn check() -> PermissionsState {
    PermissionsState {
        microphone: microphone_status(),
        accessibility: accessibility_trusted(false),
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::{CFString, CFStringRef};

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
        static kAXTrustedCheckOptionPrompt: CFStringRef;
    }

    /// `prompt = true` shows the system dialog pointing at System Settings.
    pub fn accessibility_trusted(prompt: bool) -> bool {
        unsafe {
            let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
            let options = CFDictionary::from_CFType_pairs(&[(
                key.as_CFType(),
                CFBoolean::from(prompt).as_CFType(),
            )]);
            AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) != 0
        }
    }

    pub fn microphone_status() -> &'static str {
        use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};
        unsafe {
            let Some(media_type) = AVMediaTypeAudio else {
                return "unknown";
            };
            match AVCaptureDevice::authorizationStatusForMediaType(media_type) {
                AVAuthorizationStatus::Authorized => "granted",
                AVAuthorizationStatus::Denied | AVAuthorizationStatus::Restricted => "denied",
                AVAuthorizationStatus::NotDetermined => "undetermined",
                _ => "unknown",
            }
        }
    }

    /// Triggers the system microphone prompt if status is undetermined.
    pub fn request_microphone() {
        use block2::RcBlock;
        use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio};
        unsafe {
            let Some(media_type) = AVMediaTypeAudio else {
                return;
            };
            let handler = RcBlock::new(|granted: objc2::runtime::Bool| {
                log::info!("microphone permission response: {}", granted.as_bool());
            });
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler);
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::{accessibility_trusted, microphone_status, request_microphone};

#[cfg(not(target_os = "macos"))]
pub fn accessibility_trusted(_prompt: bool) -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn microphone_status() -> &'static str {
    "unknown"
}

#[cfg(not(target_os = "macos"))]
pub fn request_microphone() {}

/// Deep link into System Settings → Privacy & Security → Accessibility.
pub const ACCESSIBILITY_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

/// Deep link into System Settings → Privacy & Security → Microphone.
pub const MICROPHONE_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";
