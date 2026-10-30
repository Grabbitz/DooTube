//! Notices.

use super::*;

impl AppDelegate {
    pub(crate) fn show_notice(&self, text: &str) {
        self.present_notice(text, Some(5.0));
    }

    pub(crate) fn show_sticky_notice(&self, text: &str) {
        self.present_notice(text, None);
    }

    pub(crate) fn present_notice(&self, text: &str, hide_after: Option<f64>) {
        let Some(notice) = self.ivars().notice.get() else {
            return;
        };
        unsafe {
            let _: () = msg_send![
                <NSObject as ClassType>::class(),
                cancelPreviousPerformRequestsWithTarget: self,
                selector: sel!(hideNotice:),
                object: Option::<&AnyObject>::None
            ];
        }
        notice.show(text);
        if let Some(delay) = hide_after {
            unsafe {
                let _: () = msg_send![
                    self,
                    performSelector: sel!(hideNotice:),
                    withObject: Option::<&AnyObject>::None,
                    afterDelay: delay
                ];
            }
        }
    }
}
