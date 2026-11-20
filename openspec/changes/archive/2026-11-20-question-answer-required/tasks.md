# Tasks

## 1. Submit needs an answer

- [x] 1.1 `QuestionState::can_submit` requires a pick or non-blank Other text
      for every choice field with an Other companion
- [x] 1.2 The focused choice is drawn with an outline, not a fill

## 2. Logging

- [x] 2.1 Log a question's arrival, refusal and outcome with its request id
      and counts only

## 3. Tests

- [x] 3.1 One question with nothing picked, or blank Other text, cannot be
      submitted; a pick or Other text can (fails on develop)
- [x] 3.2 With two questions, answering one is not enough (fails on develop)
