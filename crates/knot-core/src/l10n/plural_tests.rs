//! Every sentence whose words agree with a count has a form for one and a
//! form for more (#437), and [`super::t_count`] picks between them.
//!
//! The catalog is read straight from `locales/en.yml` so a new `%{count}`
//! string cannot be added without either a `_one`/`_other` pair or a line
//! here saying why it needs none. The flattener is enough for this file's
//! shape - nested keys, one quoted value per line - and
//! `the_catalog_flattens_to_known_keys` fails if that shape changes.

use std::collections::BTreeMap;

use super::{t, t_count, t_with};

const CATALOG: &str = include_str!("../../locales/en.yml");

/// `%{count}` keys that are not `_one`/`_other` pairs, and why each reads
/// correctly at any count. Adding a key here is a claim that no word in it
/// agrees with the number.
const UNPAIRED: &[(&str, &str)] =
    &[("import.import_count", "a button label, the count with no noun"),
      ("import.result.added", "the count stands alone before the names"),
      ("import.result.skipped", "the count stands alone in parentheses"),
      ("import.result.unreadable", "the count stands alone before the names"),
      ("changes_view.count.at_least", "a bare number with a plus"),
      ("changes_view.count.item", "the noun arrives already pluralized via plural_noun"),
      ("pull_requests.counts_total", "a participle, not a noun: \"1 recorded\""),
      ("pull_requests.count_open", "an adjective: \"1 open\""),
      ("pull_requests.count_merged", "a participle: \"1 merged\""),
      ("pull_requests.count_closed", "a participle: \"1 closed\""),
      ("pull_requests.count_pending", "a participle: \"1 checking\""),
      ("pull_requests.count_not_found", "a participle: \"1 not found\""),
      ("pull_requests.filter_toggle", "a label then a bare number"),
      ("pull_requests.bulk_remove_body", "the count follows a colon, with no noun"),
      ("mcp.summary_attention_more",
       "the verb agrees with every server named or counted, at least two"),
      ("processes.summary_more", "names then \"+N more\", no noun or verb"),
      ("git_panel.ahead", "an adverb: \"1 ahead\""),
      ("git_panel.behind", "an adverb: \"1 behind\""),
      ("git_panel.section_count", "a bare number"),
      ("git_panel.additions", "a signed number"),
      ("git_panel.deletions", "a signed number")];

/// `en.yml` as dotted keys and their values.
fn catalog() -> BTreeMap<String, String> {
    let mut path: Vec<(usize, &str)> = Vec::new();
    let mut keys = BTreeMap::new();
    for line in CATALOG.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        let Some((key, value)) = trimmed.split_once(':')
        else {
            continue;
        };
        while path.last().is_some_and(|(depth, _)| *depth >= indent) {
            path.pop();
        }
        let value = value.trim();
        if value.is_empty() {
            path.push((indent, key));
            continue;
        }
        let dotted = path.iter()
                         .map(|(_, part)| *part)
                         .chain([key])
                         .collect::<Vec<_>>()
                         .join(".");
        keys.insert(dotted, value.to_owned());
    }
    keys
}

#[test]
fn the_catalog_flattens_to_known_keys() {
    let keys = catalog();
    assert!(keys.len() > 500,
            "the flattener stopped reading the catalog");
    assert!(keys.contains_key("app.name"));
    assert!(keys.contains_key("mcp.summary_no_attention_one"));
    assert!(keys.contains_key("git_panel.section.staged"));
}

#[test]
fn every_counted_sentence_has_both_forms() {
    let keys = catalog();
    for (key, value) in &keys {
        // A sentence pair is `_one`/`_other`, read through `t_count`. A noun
        // pair for `pluralize` / `plural_noun` is `_one`/`_many`
        // (`mcp.count_one` and `mcp.count_many`): the caller passes
        // both keys, so either partner completes it.
        if let Some(base) = key.strip_suffix("_one") {
            assert!(keys.contains_key(&format!("{base}_other"))
                    || keys.contains_key(&format!("{base}_many")),
                    "{key} has no _other (or, for a noun, _many) form");
            continue;
        }
        // Only a counted `_other` is a plural form; `panel.turn_stopped_other`
        // is "stopped for some other reason".
        if let Some(base) = key.strip_suffix("_other")
           && value.contains("%{count}")
        {
            assert!(keys.contains_key(&format!("{base}_one")),
                    "{key} has no _one form");
            continue;
        }
        if value.contains("%{count}") {
            assert!(UNPAIRED.iter().any(|(unpaired, _)| unpaired == key),
                    "{key} takes %{{count}} but has no _one/_other forms; split it, or add it \
                     to UNPAIRED with the reason no word agrees with the number");
        }
    }
}

#[test]
fn the_allowlist_names_only_real_keys() {
    let keys = catalog();
    for (key, _) in UNPAIRED {
        assert!(keys.get(*key)
                    .is_some_and(|value| value.contains("%{count}")),
                "{key} is on UNPAIRED but is gone or no longer takes %{{count}}");
    }
}

#[test]
fn one_and_more_pick_their_own_form() {
    for base in ["mcp.summary_no_attention",
                 "panel.summary.calls",
                 "issues.truncated"]
    {
        assert_eq!(t_count(base, 1, &[]),
                   t_with(&format!("{base}_one"), &[("count", "1")]));
        assert_eq!(t_count(base, 2, &[]),
                   t_with(&format!("{base}_other"), &[("count", "2")]));
        assert_ne!(t(&format!("{base}_one")), t(&format!("{base}_other")));
    }
}

#[test]
fn zero_takes_the_plural_form() {
    assert_eq!(t_count("panel.summary.calls", 0, &[]),
               t_with("panel.summary.calls_other", &[("count", "0")]));
}

#[test]
fn arguments_survive_beside_the_count() {
    let text = t_count("mcp.summary_attention", 1, &[("names", "github")]);
    assert!(text.contains("github"), "got {text}");
    assert!(!text.contains("%{"),
            "a placeholder was left unfilled: {text}");
}
