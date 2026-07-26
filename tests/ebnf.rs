//! Internal consistency of the normative EBNF.
//!
//! This checks the grammar's rule graph. It does not parse `.nost`.

mod common;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, PartialEq)]
enum Event {
    Ident(String),
    Define,
    EndRule,
}

/// Scans the EBNF metasyntax into define/reference events and collects the
/// `@roots` directive.
///
/// Quoted terminals and `? ... ?` special sequences are skipped wholesale, so
/// English prose inside a special sequence is never mistaken for a rule
/// reference, and a terminal such as `"node"` is never mistaken for a definition.
fn scan(src: &str) -> (Vec<Event>, Vec<String>) {
    let mut events = Vec::new();
    let mut roots = Vec::new();
    let mut ident = String::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;

    let flush = |ident: &mut String, events: &mut Vec<Event>| {
        if !ident.is_empty() {
            events.push(Event::Ident(std::mem::take(ident)));
        }
    };

    while i < chars.len() {
        let c = chars[i];

        // Comment: `(* ... *)`. The @roots directive lives inside one.
        if c == '(' && chars.get(i + 1) == Some(&'*') {
            flush(&mut ident, &mut events);
            let start = i + 2;
            let mut end = start;
            while end < chars.len() && !(chars[end] == '*' && chars.get(end + 1) == Some(&')')) {
                end += 1;
            }
            let body: String = chars[start..end.min(chars.len())].iter().collect();
            if let Some(rest) = body.split("@roots").nth(1) {
                for word in rest.split_whitespace() {
                    if word.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_') {
                        roots.push(word.to_string());
                    } else {
                        break;
                    }
                }
            }
            i = (end + 2).min(chars.len());
            continue;
        }

        // Quoted terminal, either quoting style, with no escape processing.
        if c == '"' || c == '\'' {
            flush(&mut ident, &mut events);
            let quote = c;
            i += 1;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            i += 1;
            continue;
        }

        // Special sequence: `? ... ?`.
        if c == '?' {
            flush(&mut ident, &mut events);
            i += 1;
            while i < chars.len() && chars[i] != '?' {
                i += 1;
            }
            i += 1;
            continue;
        }

        if c.is_ascii_lowercase() || c == '_' || (c.is_ascii_digit() && !ident.is_empty()) {
            ident.push(c);
            i += 1;
            continue;
        }

        flush(&mut ident, &mut events);
        match c {
            '=' => events.push(Event::Define),
            ';' => events.push(Event::EndRule),
            _ => {}
        }
        i += 1;
    }
    flush(&mut ident, &mut events);
    (events, roots)
}

fn rule_graph(src: &str) -> (BTreeMap<String, BTreeSet<String>>, Vec<String>) {
    let (events, roots) = scan(src);
    let mut rules: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut name: Option<String> = None;
    let mut defined = false;
    let mut refs: BTreeSet<String> = BTreeSet::new();

    for event in events {
        match event {
            Event::Ident(id) => {
                if !defined && name.is_none() {
                    name = Some(id);
                } else if defined {
                    refs.insert(id);
                } else {
                    panic!("unexpected identifier {id} before `=` in a rule");
                }
            }
            Event::Define => {
                assert!(name.is_some(), "`=` with no rule name before it");
                assert!(!defined, "a second `=` inside one rule");
                defined = true;
            }
            Event::EndRule => {
                let rule = name.take().expect("`;` with no rule name");
                assert!(defined, "rule {rule} has no `=`");
                let previous = rules.insert(rule.clone(), std::mem::take(&mut refs));
                assert!(previous.is_none(), "rule {rule} is defined twice");
                defined = false;
            }
        }
    }
    assert!(name.is_none(), "the last rule is missing its `;`");
    (rules, roots)
}

#[test]
fn every_reference_is_defined_and_every_rule_is_reachable() {
    let src = common::read("grammar/nost.ebnf");
    let (rules, roots) = rule_graph(&src);

    assert!(
        rules.len() > 20,
        "expected a substantial grammar, found {} rules",
        rules.len()
    );
    assert!(
        !roots.is_empty(),
        "grammar/nost.ebnf declares no @roots directive"
    );

    for root in &roots {
        assert!(
            rules.contains_key(root),
            "@roots names undefined rule {root}"
        );
    }

    let mut undefined: Vec<String> = Vec::new();
    for (rule, refs) in &rules {
        for reference in refs {
            if !rules.contains_key(reference) {
                undefined.push(format!("{rule} -> {reference}"));
            }
        }
    }
    assert!(
        undefined.is_empty(),
        "undefined rule references: {undefined:?}"
    );

    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<String> = roots.iter().cloned().collect();
    while let Some(rule) = queue.pop_front() {
        if !reached.insert(rule.clone()) {
            continue;
        }
        for reference in &rules[&rule] {
            queue.push_back(reference.clone());
        }
    }

    let unreachable: Vec<&String> = rules.keys().filter(|k| !reached.contains(*k)).collect();
    assert!(
        unreachable.is_empty(),
        "rules unreachable from @roots {roots:?}: {unreachable:?}"
    );
}

#[test]
fn the_reference_encoding_defines_the_same_rule_names() {
    let ebnf = common::read("grammar/nost.ebnf");
    let (rules, _) = rule_graph(&ebnf);
    let pest = common::read("grammar/nost.pest");

    // Rule names the pest encoding defines, taken from `name = ` at line start.
    let mut pest_rules: BTreeSet<String> = BTreeSet::new();
    for line in pest.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_uppercase()) {
            if let Some((head, _)) = line.split_once('=') {
                let head = head.trim();
                if !head.is_empty() && head.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    pest_rules.insert(head.to_string());
                }
            }
        }
    }

    // The two encodings differ only where pest has a built-in for a lexical
    // primitive the generator-neutral EBNF has to spell out.
    //
    // pest-only: pest drives trivia through the special WHITESPACE and COMMENT
    // rules, which the EBNF states as the `trivia` lexical root instead.
    //
    // EBNF-only: pest uses ASCII_DIGIT and ASCII_HEX_DIGIT for `digit` and
    // `hex_digit`, and WHITESPACE for `whitespace`, so it defines no rule of its
    // own for them.
    let pest_only_allowed: BTreeSet<&str> = ["WHITESPACE", "COMMENT"].into_iter().collect();
    let ebnf_only_allowed: BTreeSet<&str> = ["trivia", "whitespace", "digit", "hex_digit"]
        .into_iter()
        .collect();

    let missing_from_pest: Vec<&String> = rules
        .keys()
        .filter(|name| !pest_rules.contains(*name) && !ebnf_only_allowed.contains(name.as_str()))
        .collect();
    assert!(
        missing_from_pest.is_empty(),
        "the reference encoding is missing rules the normative EBNF defines: {missing_from_pest:?}"
    );

    let extra_in_pest: Vec<&String> = pest_rules
        .iter()
        .filter(|name| !rules.contains_key(*name) && !pest_only_allowed.contains(name.as_str()))
        .collect();
    assert!(
        extra_in_pest.is_empty(),
        "the reference encoding defines rules the normative EBNF does not: {extra_in_pest:?}"
    );
}
