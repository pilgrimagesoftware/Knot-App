//! A form the agent is waiting on (`acp-panel-ui`'s "Question cards"): the
//! request, what the user has filled in so far, and where the keyboard is.
//!
//! Pure state, so it lives in [`super::PanelState`] beside the request and
//! survives re-renders and switching panels. The card's text inputs write
//! into it ([`QuestionState::set_text`]); everything that decides - what
//! Submit sends, whether it may, where a key moves focus - reads it.

use knot_acp::{AnswerValue, FieldKind, FormAnswer, QuestionRequest};

/// What one field holds so far.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// The picked choice's index.
    Single(Option<usize>),
    /// One flag per choice.
    Multi(Vec<bool>),
    /// The raw text of a text or number field.
    Text(String),
    /// `None` until the user (or the schema's default) sets it.
    Boolean(Option<bool>),
}

/// One place the keyboard can be on the card: a choice of a choice field,
/// or a whole text, number or boolean field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stop {
    pub field:  usize,
    pub choice: Option<usize>,
}

/// A key the card handles itself (`acp-panel-ui`'s "Question card keys").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKey {
    Up,
    Down,
    Tab,
    BackTab,
    Space,
    Return,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestionState {
    pub request: QuestionRequest,
    values:      Vec<FieldValue>,
    focus:       usize,
}

impl QuestionState {
    /// The form with the schema's defaults filled in and focus on the first
    /// choice of the first field (or the first field when it has none).
    pub fn new(request: QuestionRequest) -> Self {
        let values = request.fields
                            .iter()
                            .map(|field| initial_value(&field.kind))
                            .collect();
        Self { request,
               values,
               focus: 0 }
    }

    pub fn value(&self, field: usize) -> Option<&FieldValue> {
        self.values.get(field)
    }

    /// Every stop on the card, in order.
    pub fn stops(&self) -> Vec<Stop> {
        self.request
            .fields
            .iter()
            .enumerate()
            .flat_map(|(field, form_field)| match &form_field.kind {
                FieldKind::Single { choices, .. } | FieldKind::Multi { choices, .. }
                    if !choices.is_empty() =>
                {
                    (0..choices.len()).map(|choice| Stop { field,
                                                           choice: Some(choice) })
                                      .collect::<Vec<_>>()
                }
                FieldKind::Single { .. }
                | FieldKind::Multi { .. }
                | FieldKind::Text { .. }
                | FieldKind::Number { .. }
                | FieldKind::Boolean { .. } => vec![Stop { field,
                                                           choice: None }],
            })
            .collect()
    }

    /// Where the keyboard is. `None` only for a form with no fields.
    pub fn focus(&self) -> Option<Stop> {
        self.stops().get(self.focus).copied()
    }

    /// Moves focus to `stop` - a click on a choice, or an input focused by
    /// the pointer.
    pub fn focus_stop(&mut self, stop: Stop) {
        if let Some(index) = self.stops().iter().position(|candidate| *candidate == stop) {
            self.focus = index;
        }
    }

    /// The text field `<key>_custom` directly after choice field `field`:
    /// that question's "Other" answer, by `claude-agent-acp`'s convention.
    pub fn other_of(&self, field: usize) -> Option<usize> {
        let question = self.request.fields.get(field)?;
        let next = self.request.fields.get(field + 1)?;
        let is_choice = matches!(question.kind,
                                 FieldKind::Single { .. } | FieldKind::Multi { .. });
        (is_choice
         && matches!(next.kind, FieldKind::Text { .. })
         && next.key == format!("{}_custom", question.key)).then_some(field + 1)
    }

    /// Whether `field` is drawn as the previous field's "Other".
    pub fn is_other(&self, field: usize) -> bool {
        field.checked_sub(1)
             .and_then(|previous| self.other_of(previous))
        == Some(field)
    }

    pub fn pick(&mut self, field: usize, choice: usize) {
        if let Some(FieldValue::Single(picked)) = self.values.get_mut(field) {
            *picked = Some(choice);
        }
    }

    pub fn toggle(&mut self, field: usize, choice: usize) {
        if let Some(FieldValue::Multi(flags)) = self.values.get_mut(field)
           && let Some(flag) = flags.get_mut(choice)
        {
            *flag = !*flag;
        }
    }

    pub fn toggle_boolean(&mut self, field: usize) {
        if let Some(FieldValue::Boolean(flag)) = self.values.get_mut(field) {
            *flag = Some(!flag.unwrap_or(false));
        }
    }

    pub fn set_text(&mut self, field: usize, text: String) {
        if let Some(FieldValue::Text(current)) = self.values.get_mut(field) {
            *current = text;
        }
    }

    /// Applies a card key. Returns `true` when anything changed.
    pub fn key(&mut self, key: CardKey) -> bool {
        let stops = self.stops();
        let Some(stop) = stops.get(self.focus).copied()
        else {
            return false;
        };
        let before = (self.focus, self.values.clone());
        // In a text or number field, Up and Down move between fields, as
        // Shift-Tab and Tab do: there are no choices to step through.
        let in_text = matches!(self.values.get(stop.field), Some(FieldValue::Text(_)));
        let key = match key {
            CardKey::Up if in_text => CardKey::BackTab,
            CardKey::Down if in_text => CardKey::Tab,
            key => key,
        };
        match key {
            CardKey::Up => self.focus = self.focus.saturating_sub(1),
            CardKey::Down => self.focus = (self.focus + 1).min(stops.len() - 1),
            CardKey::Tab => self.move_to_field(&stops, stop.field + 1),
            CardKey::BackTab => {
                let field = if self.field_start(&stops, stop.field) == self.focus {
                    stop.field.saturating_sub(1)
                }
                else {
                    stop.field
                };
                self.move_to_field(&stops, field);
            }
            CardKey::Space => self.activate(stop),
            CardKey::Return => {
                if let (Some(choice), Some(FieldValue::Single(_))) =
                    (stop.choice, self.values.get(stop.field))
                {
                    self.pick(stop.field, choice);
                }
                let next = self.other_of(stop.field)
                               .map_or(stop.field + 1, |other| other + 1);
                self.move_to_field(&stops, next);
            }
        }
        before != (self.focus, self.values.clone())
    }

    fn activate(&mut self, stop: Stop) {
        match (stop.choice, self.values.get(stop.field)) {
            (Some(choice), Some(FieldValue::Single(_))) => self.pick(stop.field, choice),
            (Some(choice), Some(FieldValue::Multi(_))) => self.toggle(stop.field, choice),
            (None, Some(FieldValue::Boolean(_))) => self.toggle_boolean(stop.field),
            _ => {}
        }
    }

    /// Focus on `field`'s first stop; stays put past the last field.
    fn move_to_field(&mut self, stops: &[Stop], field: usize) {
        if let Some(index) = stops.iter().position(|stop| stop.field == field) {
            self.focus = index;
        }
    }

    fn field_start(&self, stops: &[Stop], field: usize) -> usize {
        stops.iter()
             .position(|stop| stop.field == field)
             .unwrap_or(0)
    }

    /// Each field's answer, `None` when empty. A number field whose text is
    /// not a number answers its raw text, which no number field accepts.
    fn answer_values(&self) -> Vec<Option<AnswerValue>> {
        self.request
            .fields
            .iter()
            .zip(&self.values)
            .map(|(field, value)| match (value, &field.kind) {
                (FieldValue::Single(picked), FieldKind::Single { choices, .. }) => {
                    picked.and_then(|index| choices.get(index))
                          .map(|choice| AnswerValue::Text(choice.value.clone()))
                }
                (FieldValue::Multi(flags), FieldKind::Multi { choices, .. }) => {
                    let picked: Vec<String> = choices.iter()
                                                     .zip(flags)
                                                     .filter(|(_, flag)| **flag)
                                                     .map(|(choice, _)| choice.value.clone())
                                                     .collect();
                    (!picked.is_empty()).then_some(AnswerValue::Choices(picked))
                }
                (FieldValue::Text(text), FieldKind::Number { integer, .. }) => {
                    let text = text.trim();
                    (!text.is_empty()).then(|| parse_number(text, *integer).map_or_else(|| AnswerValue::Text(text.to_owned()), AnswerValue::Number))
                }
                (FieldValue::Text(text), _) => {
                    (!text.is_empty()).then(|| AnswerValue::Text(text.clone()))
                }
                (FieldValue::Boolean(flag), _) => flag.map(AnswerValue::Boolean),
                (FieldValue::Single(_) | FieldValue::Multi(_), _) => None,
            })
            .collect()
    }

    /// Whether Submit may be pressed: every field accepts what it holds.
    pub fn can_submit(&self) -> bool {
        self.request
            .fields
            .iter()
            .zip(self.answer_values())
            .all(|(field, value)| field.accepts(value.as_ref()))
    }

    /// Submit's answer: every filled field, in form order.
    pub fn submitted(&self) -> FormAnswer {
        FormAnswer::Submitted(self.request
                                  .fields
                                  .iter()
                                  .zip(self.answer_values())
                                  .filter_map(|(field, value)| {
                                      value.map(|value| (field.key.clone(), value))
                                  })
                                  .collect())
    }
}

fn initial_value(kind: &FieldKind) -> FieldValue {
    match kind {
        FieldKind::Single { choices, default } => {
            FieldValue::Single(default.as_ref().and_then(|default| {
                                                   choices.iter().position(|choice| {
                                                                     choice.value == *default
                                                                 })
                                               }))
        }
        FieldKind::Multi { choices, default, .. } => {
            FieldValue::Multi(choices.iter()
                                     .map(|choice| default.contains(&choice.value))
                                     .collect())
        }
        FieldKind::Text { default, .. } => FieldValue::Text(default.clone().unwrap_or_default()),
        FieldKind::Number { default, integer, .. } => {
            FieldValue::Text(default.map(|number| {
                                        if *integer {
                                            format!("{}", number as i64)
                                        }
                                        else {
                                            number.to_string()
                                        }
                                    })
                                    .unwrap_or_default())
        }
        FieldKind::Boolean { default } => FieldValue::Boolean(*default),
    }
}

fn parse_number(text: &str, integer: bool) -> Option<serde_json::Number> {
    if integer {
        text.parse::<i64>().ok().map(serde_json::Number::from)
    }
    else {
        text.parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
    }
}
