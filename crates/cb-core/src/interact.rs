//! What Use does when more than one thing is in reach (survey W40; owner, 2026-10-26: "You should be able to scroll
//! wheel select multiple options"). Every interaction the pilot can start with Use (greeting someone, a base
//! terminal, getting into or out of a car, stepping into the mech) is offered here each step; the prompt lists them,
//! the scroll wheel moves the selection, and Use does the selected one. With one option the prompt and Use behave as
//! before: that option, no list. When several come into reach at once the greeting is selected first (W40: "The
//! greeting"). Pure: `cb-app` says what is in reach and routes the press to the one interaction chosen.

/// One thing Use can do. The order is the order they are listed and the first selected when several come into
/// reach together: the greeting first (survey W40), then the terminal, the car, and the mech.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Interaction {
    /// Greet the person in front of the pilot (civilians).
    Greet,
    /// Open a hangar bay's terminal (air-bases).
    Terminal,
    /// Get into the car at whose driver's door the pilot stands (car-driving).
    GetIn,
    /// Get out of the stopped car (car-driving).
    GetOut,
    /// Step into the stopped mech's cockpit (pilot-boarding).
    Board,
}

impl Interaction {
    /// What the prompt calls it.
    pub fn label(self) -> &'static str {
        match self {
            Interaction::Greet => "Greet",
            Interaction::Terminal => "Use the terminal: jobs, shop, refit",
            Interaction::GetIn => "Get in",
            Interaction::GetOut => "Get out",
            Interaction::Board => "Step into the cockpit",
        }
    }
}

/// The interactions in reach and the one selected. The selection is kept by what it is, not by its place in the
/// list, so one coming into or going out of reach does not move it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selector {
    options: Vec<Interaction>,
    selected: Option<Interaction>,
}

impl Selector {
    /// This step's interactions in reach, in any order and with repeats. The selection stays where it was while it is
    /// still offered; otherwise the first in `Interaction`'s order is selected (the greeting, when it is offered).
    pub fn offer(&mut self, in_reach: &[Interaction]) {
        let mut options = in_reach.to_vec();
        options.sort_unstable();
        options.dedup();
        if !self.selected.is_some_and(|s| options.contains(&s)) {
            self.selected = options.first().copied();
        }
        self.options = options;
    }

    /// Moves the selection `steps` places down the list (negative: up), wrapping round; the mouse wheel's steps.
    pub fn scroll(&mut self, steps: i32) {
        let n = self.options.len() as i32;
        let Some(at) = self.selected.and_then(|s| self.options.iter().position(|o| *o == s)) else {
            return;
        };
        if n < 2 || steps == 0 {
            return;
        }
        self.selected = Some(self.options[(at as i32 + steps).rem_euclid(n) as usize]);
    }

    /// The interactions in reach, in the order listed.
    pub fn options(&self) -> &[Interaction] {
        &self.options
    }

    /// The interaction Use does now, if anything is in reach.
    pub fn selected(&self) -> Option<Interaction> {
        self.selected
    }

    /// What Use does this step: the selection when Use was pressed, else nothing.
    pub fn pressed(&self, use_pressed: bool) -> Option<Interaction> {
        self.selected.filter(|_| use_pressed)
    }

    /// The prompt's lines: one option reads `[E]  Get in` as it always has; several are listed, the selected one
    /// marked with the Use key and the others indented under it, then how to choose.
    pub fn lines(&self, use_key: &str, choose: &str) -> Vec<String> {
        match self.options.as_slice() {
            [] => Vec::new(),
            [one] => vec![format!("[{use_key}]  {}", one.label())],
            many => many
                .iter()
                .map(|o| {
                    if Some(*o) == self.selected {
                        format!("> [{use_key}]  {}", o.label())
                    } else {
                        format!("{}{}", " ".repeat(use_key.len() + 6), o.label())
                    }
                })
                .chain(std::iter::once(format!("({choose}: choose)")))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Interaction::*;

    #[test]
    fn with_one_thing_in_reach_use_does_it_and_the_prompt_reads_as_before() {
        let mut s = Selector::default();
        s.offer(&[GetIn]);
        assert_eq!(s.pressed(true), Some(GetIn));
        assert_eq!(s.pressed(false), None, "nothing happens without the press");
        assert_eq!(s.lines("E", "Mouse wheel"), vec!["[E]  Get in".to_string()]);
        s.scroll(3);
        assert_eq!(s.selected(), Some(GetIn), "the wheel has nothing to move");
    }

    #[test]
    fn with_nothing_in_reach_use_does_nothing_and_no_prompt_shows() {
        let mut s = Selector::default();
        s.offer(&[Board]);
        s.offer(&[]);
        assert_eq!(s.pressed(true), None);
        assert!(s.lines("E", "Mouse wheel").is_empty());
    }

    #[test]
    fn when_a_greeting_and_the_mech_are_both_in_reach_the_greeting_wins() {
        // Survey W40, answered 2026-10-26: "The greeting".
        let mut s = Selector::default();
        s.offer(&[Board, Greet]);
        assert_eq!(s.options(), &[Greet, Board]);
        assert_eq!(s.pressed(true), Some(Greet));
    }

    #[test]
    fn the_wheel_moves_the_selection_and_use_does_the_selected_one() {
        // "You should be able to scroll wheel select multiple options".
        let mut s = Selector::default();
        s.offer(&[Greet, GetIn, Board]);
        s.scroll(1);
        assert_eq!(s.pressed(true), Some(GetIn));
        s.scroll(1);
        assert_eq!(s.pressed(true), Some(Board));
        s.scroll(1);
        assert_eq!(s.pressed(true), Some(Greet), "it wraps round");
        s.scroll(-1);
        assert_eq!(s.pressed(true), Some(Board), "and back up");
        let lines = s.lines("E", "Mouse wheel");
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[2], "> [E]  Step into the cockpit");
        assert_eq!(lines[0], "       Greet");
        assert_eq!(lines[3], "(Mouse wheel: choose)");
    }

    #[test]
    fn the_selection_stays_on_its_choice_while_others_come_and_go() {
        let mut s = Selector::default();
        s.offer(&[Greet, Board]);
        s.scroll(1);
        assert_eq!(s.selected(), Some(Board));
        s.offer(&[Board, Board]);
        assert_eq!(s.options(), &[Board], "repeats are one option");
        s.offer(&[Greet, Terminal, Board]);
        assert_eq!(s.selected(), Some(Board), "a person stepping back into reach does not take it");
        s.offer(&[Greet, Terminal]);
        assert_eq!(s.selected(), Some(Greet), "the mech gone, the first is selected");
    }
}
