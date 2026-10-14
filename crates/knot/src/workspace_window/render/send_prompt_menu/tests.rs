//! Unit tests for [`super`].

use super::*;

fn target(name: &str, running: bool) -> SendTarget {
    SendTarget { id: Uuid::new_v4(),
                 name: name.to_owned(),
                 running }
}

#[test]
fn the_agents_are_listed_alphabetically() {
    let names: Vec<_> = send_targets(vec![target("Bo", true),
                                          target("ada", true),
                                          target("Cy", true)]).into_iter()
                                                              .map(|target| target.name)
                                                              .collect();
    assert_eq!(names, ["ada", "Bo", "Cy"]);
}

/// A stopped agent stays in the list, in name order, and can be chosen
/// (#208); its label marks it inactive and still names it.
#[test]
fn a_stopped_agent_is_listed_and_marked_inactive() {
    let targets = send_targets(vec![target("Bo", true), target("Ada", false)]);
    assert_eq!(targets.iter()
                      .map(|target| (target.name.as_str(), target.running))
                      .collect::<Vec<_>>(),
               [("Ada", false), ("Bo", true)]);

    assert_eq!(targets[1].label(), "Bo");
    let inactive = targets[0].label();
    assert_eq!(inactive,
               knot_core::l10n::t_with("changes_view.send_target_inactive", &[("name", "Ada")]));
    assert!(inactive.contains("Ada"), "{inactive}");
    assert!(!inactive.contains("%{"), "{inactive}");
    assert_ne!(inactive, "changes_view.send_target_inactive",
               "the key must resolve");
}

#[test]
fn each_prompt_names_its_item() {
    let url = "https://github.com/acme/widget/issues/42";
    let issue = WorkItemRef::Issue(url.to_owned()).prompt_text();
    assert!(issue.contains(url), "{issue}");
    assert!(!issue.contains("%{"), "{issue}");
    assert_eq!(issue,
               knot_core::l10n::t_with("changes_view.prompt.issue", &[("url", url)]));

    let change = WorkItemRef::Change("add-login".to_owned()).prompt_text();
    assert!(change.contains("add-login"), "{change}");
    assert!(!change.contains("%{"), "{change}");
}
