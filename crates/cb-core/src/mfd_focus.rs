//! Which page a cockpit MFD shows across MFD focus (openspec change `mfd-menus`, survey C14). The pilot focuses the
//! screens by holding free look while seated; a screen's page and the readout its menu remembers (`home`) last only
//! while that focus does. Owner, 2026-10-26: "mfds return to default as soon as you click back on them, leaving mfd
//! focus". So letting go of free look, or leaving the seat, puts every screen back on the page it was built with, and
//! nothing is remembered between focuses or between sorties. Pure, so the rule is tested without Bevy; `cb-app::mfd`
//! keeps one of these on each screen.

/// One screen's pages: the page it is built with, the page it shows, and the readout its open menu remembers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenPages {
    /// The page the screen starts every sortie and every focus on.
    pub default: usize,
    /// The page it shows now.
    pub page: usize,
    /// The readout its menu was opened from, which BACK gives back; none while it shows a readout.
    pub home: Option<usize>,
}

impl ScreenPages {
    /// A screen built on page `default`, showing it.
    pub fn new(default: usize) -> Self {
        Self {
            default,
            page: default,
            home: None,
        }
    }

    /// The pilot left MFD focus: back to the default page, forgetting any menu. True when the page changed.
    pub fn leave_focus(&mut self) -> bool {
        let changed = self.page != self.default;
        self.page = self.default;
        self.home = None;
        changed
    }
}

/// Is the pilot focused on the MFDs: seated, with free look held.
pub fn focused(seated: bool, free_look: bool) -> bool {
    seated && free_look
}

/// Did MFD focus end between two frames (`was` then `now`, each from `focused`)?
pub fn focus_left(was: bool, now: bool) -> bool {
    was && !now
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screen_starts_on_its_default_page() {
        let s = ScreenPages::new(2);
        assert_eq!((s.page, s.home), (2, None));
    }

    #[test]
    fn leaving_focus_puts_a_screen_back_on_its_default_page_and_forgets_its_menu() {
        let mut s = ScreenPages::new(2);
        // Clicked into its menu from MISSION, then picked a different readout.
        s.page = 5;
        s.home = Some(2);
        assert!(s.leave_focus(), "the page changed");
        assert_eq!(s, ScreenPages::new(2));
        s.page = 1;
        assert!(s.leave_focus());
        assert_eq!(s.page, 2, "a readout picked in focus does not outlast it");
    }

    #[test]
    fn leaving_focus_on_the_default_page_changes_nothing() {
        let mut s = ScreenPages::new(4);
        assert!(!s.leave_focus());
        assert_eq!(s, ScreenPages::new(4));
    }

    #[test]
    fn focus_is_seated_with_free_look_and_it_ends_when_either_goes() {
        assert!(focused(true, true));
        assert!(!focused(true, false), "free look let go");
        assert!(!focused(false, true), "out of the seat");
        assert!(focus_left(focused(true, true), focused(true, false)));
        assert!(focus_left(focused(true, true), focused(false, true)));
        assert!(!focus_left(false, false), "never focused, nothing to leave");
        assert!(!focus_left(true, true), "still focused");
        assert!(!focus_left(false, true), "entering focus is not leaving it");
    }
}
