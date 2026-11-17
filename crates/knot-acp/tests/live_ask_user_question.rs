//! Against the real `claude-agent-acp` (#430): Claude's AskUserQuestion
//! reaches this client as a form, and each way of answering it lands.
//!
//! Ignored by default - it needs the adapter on `PATH`, a signed-in Claude
//! Code, and spends a few model turns. Run with
//! `cargo test -p knot-acp --test live_ask_user_question -- --ignored
//! --nocapture`.

use std::time::Duration;

use knot_acp::{AcpClient, AnswerValue, FieldKind, FormAnswer, SessionEvent, SessionUpdate};
use tokio::process::Command;

const PROMPT: &str = "Use the AskUserQuestion tool now to ask me one single-select question, \
                      'Which color?', with the options Red, Green and Blue. After you get my \
                      answer, reply with exactly ANSWER=<what I chose>, or ANSWER=SKIPPED if I \
                      skipped it, and nothing else.";

/// Asks, answers with `answer` (built from the question), and returns the
/// reply text, or `None` if the turn ended without one.
async fn ask(answer: impl FnOnce(&knot_acp::QuestionRequest) -> FormAnswer) -> Option<String> {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let (client, mut events) =
        AcpClient::connect(Command::new("claude-agent-acp")).await
                                                            .expect("the adapter starts");
    let session = client.session_new(&scratch.path().to_string_lossy(), None)
                        .await
                        .expect("a session")
                        .session_id;
    let prompting = client.clone();
    let prompt_session = session.clone();
    tokio::spawn(async move { prompting.session_prompt(&prompt_session, PROMPT).await });

    let mut answer = Some(answer);
    let mut reply = String::new();
    loop {
        let event = tokio::time::timeout(Duration::from_secs(180), events.recv()).await
                                                                               .expect("the turn moves")
                                                                               .expect("the stream is open");
        match event {
            SessionEvent::Question(request) => {
                let field = &request.fields[0];
                assert!(matches!(field.kind, FieldKind::Single { .. }),
                        "a single choice: {field:?}");
                if let Some(answer) = answer.take() {
                    client.answer_question(&request, answer(&request));
                }
            }
            SessionEvent::Update(SessionUpdate::TextDelta { text }) => reply.push_str(&text),
            SessionEvent::Update(SessionUpdate::TurnEnd { .. }) => break,
            _ => {}
        }
    }
    client.close().await;
    eprintln!("reply: {reply}");
    Some(reply)
}

#[tokio::test]
#[ignore = "needs claude-agent-acp and a signed-in Claude Code"]
async fn a_submitted_answer_reaches_claude() {
    let reply = ask(|request| {
                    FormAnswer::Submitted(vec![(request.fields[0].key.clone(),
                                                AnswerValue::Text("Green".to_owned()))])
                }).await
                  .expect("a reply");
    assert!(reply.contains("ANSWER=Green"), "{reply}");
}

#[tokio::test]
#[ignore = "needs claude-agent-acp and a signed-in Claude Code"]
async fn a_skipped_question_reaches_claude_as_skipped() {
    let reply = ask(|_| FormAnswer::Skipped).await.expect("a reply");
    assert!(reply.contains("SKIPPED"), "{reply}");
}

/// Stop while the question is pending: the client answers the form
/// `cancel` and the turn ends cancelled rather than waiting on it.
#[tokio::test]
#[ignore = "needs claude-agent-acp and a signed-in Claude Code"]
async fn stop_while_asked_ends_the_turn_cancelled() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let (client, mut events) =
        AcpClient::connect(Command::new("claude-agent-acp")).await
                                                            .expect("the adapter starts");
    let session = client.session_new(&scratch.path().to_string_lossy(), None)
                        .await
                        .expect("a session")
                        .session_id;
    let prompting = client.clone();
    let prompt_session = session.clone();
    tokio::spawn(async move { prompting.session_prompt(&prompt_session, PROMPT).await });
    let mut resolved = false;
    let stop_reason = loop {
        let event = tokio::time::timeout(Duration::from_secs(180), events.recv()).await
                                                                               .expect("the turn moves")
                                                                               .expect("the stream is open");
        match event {
            SessionEvent::Question(_) => {
                client.session_cancel(&session).await.expect("cancel sent")
            }
            SessionEvent::InputResolved { .. } => resolved = true,
            SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason }) => break stop_reason,
            _ => {}
        }
    };
    client.close().await;
    assert!(resolved, "the form was resolved");
    assert_eq!(stop_reason.as_str(), "cancelled");
}
