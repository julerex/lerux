use lerux_ladybird_web::{
    Combinator, CompoundSelector, Property, SimpleSelector, StyleRule, StyleSheet, Value,
};

fn sheet(input: &str) -> StyleSheet {
    StyleSheet::parse(input).expect("style sheet")
}

fn only_rule(input: &str) -> StyleRule {
    let parsed = sheet(input);
    let mut rules = parsed.rules().iter().cloned();
    let rule = rules.next().expect("rule");
    assert!(rules.next().is_none());
    rule
}

fn simple(compound: &CompoundSelector, index: usize) -> &SimpleSelector {
    compound.simples().get(index).expect("simple selector")
}

#[test]
fn documented_selector_is_id_then_classes_then_child_type() {
    let rule = only_rule("#foo .bar.baz > img { color: black }");
    let selector = &rule.selectors()[0];
    assert!(rule.selectors().get(1).is_none());
    let compounds = selector.compounds();
    assert_eq!(compounds.len(), 3);

    assert_eq!(compounds[0].combinator(), Combinator::None);
    assert!(matches!(simple(&compounds[0], 0), SimpleSelector::Id(id) if id.as_ref() == "foo"));
    assert!(compounds[0].simples().get(1).is_none());

    assert_eq!(compounds[1].combinator(), Combinator::Descendant);
    assert!(
        matches!(simple(&compounds[1], 0), SimpleSelector::Class(name) if name.as_ref() == "bar")
    );
    assert!(
        matches!(simple(&compounds[1], 1), SimpleSelector::Class(name) if name.as_ref() == "baz")
    );
    assert!(compounds[1].simples().get(2).is_none());

    assert_eq!(compounds[2].combinator(), Combinator::Child);
    assert!(
        matches!(simple(&compounds[2], 0), SimpleSelector::Type(name) if name.as_ref() == "img")
    );
    assert!(compounds[2].simples().get(1).is_none());

    assert_eq!(rule.declarations().len(), 1);
    assert_eq!(rule.declarations()[0].property(), &Property::Color);
    assert_eq!(
        rule.declarations()[0].value(),
        &Value::Specified("black".into())
    );
}

#[test]
fn compound_keeps_type_class_and_id_in_source_order() {
    let rule = only_rule("P.Note#Main { display: block }");
    let compound = &rule.selectors()[0].compounds()[0];
    assert_eq!(compound.combinator(), Combinator::None);
    assert!(rule.selectors()[0].compounds().get(1).is_none());
    assert!(matches!(simple(compound, 0), SimpleSelector::Type(name) if name.as_ref() == "p"));
    assert!(matches!(simple(compound, 1), SimpleSelector::Class(name) if name.as_ref() == "Note"));
    assert!(matches!(simple(compound, 2), SimpleSelector::Id(name) if name.as_ref() == "Main"));
    assert_eq!(rule.declarations()[0].property(), &Property::Display);
    assert_eq!(
        rule.declarations()[0].value(),
        &Value::Specified("block".into())
    );
}

#[test]
fn accepted_properties_stay_in_source_order() {
    let rule = only_rule(
        "p { color: #000; background-color: #fff; font-size: 16px; display: block; margin: 0 8px; padding: 8px 0; width: 100%; }",
    );
    let expected = [
        (Property::Color, "#000"),
        (Property::BackgroundColor, "#fff"),
        (Property::FontSize, "16px"),
        (Property::Display, "block"),
        (Property::Margin, "0 8px"),
        (Property::Padding, "8px 0"),
        (Property::Width, "100%"),
    ];
    assert_eq!(rule.declarations().len(), expected.len());
    for (declaration, (property, value)) in rule.declarations().iter().zip(expected) {
        assert_eq!(declaration.property(), &property);
        assert_eq!(declaration.value(), &Value::Specified(value.into()));
    }
}

#[test]
fn unknown_property_is_kept_and_custom_property_is_unresolved() {
    let rule =
        only_rule("p { Float: Left; --Accent: #654ff0; color: var(--Accent); width: variable; }");
    let declarations = rule.declarations();
    assert_eq!(declarations.len(), 4);
    assert_eq!(
        declarations[0].property(),
        &Property::Unknown("float".into())
    );
    assert_eq!(declarations[0].value(), &Value::Specified("Left".into()));
    assert_eq!(
        declarations[1].property(),
        &Property::Custom("--Accent".into())
    );
    assert_eq!(
        declarations[1].value(),
        &Value::Unresolved("#654ff0".into())
    );
    assert_eq!(declarations[2].property(), &Property::Color);
    assert_eq!(
        declarations[2].value(),
        &Value::Unresolved("var(--Accent)".into())
    );
    assert_eq!(declarations[3].property(), &Property::Width);
    assert_eq!(
        declarations[3].value(),
        &Value::Specified("variable".into())
    );
}

#[test]
fn var_function_with_whitespace_stays_unresolved() {
    let rule = only_rule("p { margin: VAR (--gap); }");
    assert_eq!(rule.declarations()[0].property(), &Property::Margin);
    assert_eq!(
        rule.declarations()[0].value(),
        &Value::Unresolved("VAR (--gap)".into())
    );
}

#[test]
fn comma_separates_selectors_and_rules_stay_in_order() {
    let parsed = sheet("h1, h2.x { margin: 0 } p > span { padding: 1px }");
    assert_eq!(parsed.rules().len(), 2);

    let first = &parsed.rules()[0];
    assert_eq!(first.selectors().len(), 2);
    assert!(matches!(
        simple(&first.selectors()[0].compounds()[0], 0),
        SimpleSelector::Type(name) if name.as_ref() == "h1"
    ));
    let second_selector = &first.selectors()[1].compounds()[0];
    assert!(
        matches!(simple(second_selector, 0), SimpleSelector::Type(name) if name.as_ref() == "h2")
    );
    assert!(
        matches!(simple(second_selector, 1), SimpleSelector::Class(name) if name.as_ref() == "x")
    );
    assert_eq!(first.declarations()[0].property(), &Property::Margin);

    let second = &parsed.rules()[1];
    let compounds = second.selectors()[0].compounds();
    assert_eq!(compounds[0].combinator(), Combinator::None);
    assert_eq!(compounds[1].combinator(), Combinator::Child);
    assert!(
        matches!(simple(&compounds[1], 0), SimpleSelector::Type(name) if name.as_ref() == "span")
    );
    assert_eq!(second.declarations()[0].property(), &Property::Padding);
    assert_eq!(
        second.declarations()[0].value(),
        &Value::Specified("1px".into())
    );
}

#[test]
fn comment_does_not_split_a_compound_or_end_a_value() {
    let rule = only_rule("div/*c*/.note { color: /* x */ red; width: \"a;b\"; }");
    let compound = &rule.selectors()[0].compounds()[0];
    assert!(rule.selectors()[0].compounds().get(1).is_none());
    assert!(matches!(simple(compound, 0), SimpleSelector::Type(name) if name.as_ref() == "div"));
    assert!(matches!(simple(compound, 1), SimpleSelector::Class(name) if name.as_ref() == "note"));
    assert_eq!(
        rule.declarations()[0].value(),
        &Value::Specified("red".into())
    );
    assert_eq!(
        rule.declarations()[1].value(),
        &Value::Specified("\"a;b\"".into())
    );
}

#[test]
fn empty_sheet_has_no_rules() {
    assert!(sheet("").rules().is_empty());
    assert!(sheet("  /* only */  ").rules().is_empty());
}

#[test]
fn at_rules_and_other_combinators_are_rejected() {
    assert!(StyleSheet::parse("@import \"x\";").is_err());
    assert!(StyleSheet::parse("@media screen { p { color: red } }").is_err());
    assert!(StyleSheet::parse("div + p { color: red }").is_err());
    assert!(StyleSheet::parse("div ~ p { color: red }").is_err());
    assert!(StyleSheet::parse("p { color: red").is_err());
}
