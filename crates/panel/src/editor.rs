// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Stroke history for the panel. Kept free of GTK so undo, redo, delete and
//! clear can be unit-tested without a display server.

pub type Stroke = Vec<(f32, f32)>;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct InkEditor {
    strokes: Vec<Stroke>,
    redo: Vec<Stroke>,
}

impl InkEditor {
    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes
    }

    pub fn can_undo(&self) -> bool {
        !self.strokes.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn can_delete_last(&self) -> bool {
        !self.strokes.is_empty()
    }

    pub fn can_clear(&self) -> bool {
        !self.strokes.is_empty() || !self.redo.is_empty()
    }

    /// Append a completed stroke. A new stroke invalidates the redo history.
    pub fn push_stroke(&mut self, stroke: Stroke) -> bool {
        if stroke.len() < 2 {
            return false;
        }
        self.strokes.push(stroke);
        self.redo.clear();
        true
    }

    /// Move the last completed stroke to the redo stack.
    pub fn undo(&mut self) -> bool {
        let Some(stroke) = self.strokes.pop() else {
            return false;
        };
        self.redo.push(stroke);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(stroke) = self.redo.pop() else {
            return false;
        };
        self.strokes.push(stroke);
        true
    }

    /// Remove the last completed stroke permanently and drop its redo entry.
    pub fn delete_last(&mut self) -> bool {
        if self.strokes.pop().is_none() {
            return false;
        }
        self.redo.clear();
        true
    }

    pub fn clear(&mut self) -> bool {
        if self.strokes.is_empty() && self.redo.is_empty() {
            return false;
        }
        self.strokes.clear();
        self.redo.clear();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(x: f32) -> Stroke {
        vec![(x, 0.0), (x, 1.0)]
    }

    #[test]
    fn undo_then_redo_round_trips() {
        let mut editor = InkEditor::default();
        assert!(editor.push_stroke(stroke(1.0)));
        assert!(editor.push_stroke(stroke(2.0)));
        assert_eq!(editor.strokes().len(), 2);

        assert!(editor.undo());
        assert_eq!(editor.strokes(), &[stroke(1.0)]);
        assert!(editor.can_redo());
        assert!(editor.redo());
        assert_eq!(editor.strokes(), &[stroke(1.0), stroke(2.0)]);
        assert!(!editor.can_redo());
    }

    #[test]
    fn a_new_stroke_clears_redo() {
        let mut editor = InkEditor::default();
        editor.push_stroke(stroke(1.0));
        editor.push_stroke(stroke(2.0));
        assert!(editor.undo());
        assert!(editor.can_redo());

        assert!(editor.push_stroke(stroke(3.0)));
        assert!(!editor.can_redo());
        assert_eq!(editor.strokes(), &[stroke(1.0), stroke(3.0)]);
    }

    #[test]
    fn delete_last_is_not_undoable() {
        let mut editor = InkEditor::default();
        editor.push_stroke(stroke(1.0));
        editor.push_stroke(stroke(2.0));
        editor.push_stroke(stroke(3.0));
        assert!(editor.undo());
        assert!(editor.can_redo());

        assert!(editor.delete_last());
        assert_eq!(editor.strokes(), &[stroke(1.0)]);
        assert!(!editor.can_redo());
        assert!(editor.can_undo());
    }

    #[test]
    fn clear_removes_strokes_and_redo() {
        let mut editor = InkEditor::default();
        editor.push_stroke(stroke(1.0));
        editor.push_stroke(stroke(2.0));
        editor.undo();
        assert!(editor.clear());
        assert!(editor.strokes().is_empty());
        assert!(!editor.can_redo());
        assert!(!editor.clear());
    }

    #[test]
    fn empty_actions_report_no_change() {
        let mut editor = InkEditor::default();
        assert!(!editor.undo());
        assert!(!editor.redo());
        assert!(!editor.delete_last());
        assert!(!editor.clear());
        assert!(!editor.push_stroke(vec![(1.0, 2.0)]));
    }
}
