use core::iter;

use lerux_ladybird_html::{Element, Parent};

use crate::css::{Combinator, CompoundSelector, Selector, SimpleSelector, StyleRule};

/// Field order is the comparison order: ids, then classes, then types.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Specificity {
    ids: u32,
    classes: u32,
    types: u32,
}

/// The specificity of the most specific selector in `rule` that matches `element`.
pub(super) fn match_rule(rule: &StyleRule, element: &Element<'_>) -> Option<Specificity> {
    rule.selectors()
        .iter()
        .filter(|selector| matches(selector, element))
        .map(specificity)
        .max()
}

fn specificity(selector: &Selector) -> Specificity {
    let mut specificity = Specificity::default();
    for simple in selector
        .compounds()
        .iter()
        .flat_map(CompoundSelector::simples)
    {
        match simple {
            SimpleSelector::Id(_) => specificity.ids += 1,
            SimpleSelector::Class(_) => specificity.classes += 1,
            SimpleSelector::Type(_) => specificity.types += 1,
        }
    }
    specificity
}

fn matches(selector: &Selector, element: &Element<'_>) -> bool {
    let Some((subject, rest)) = selector.compounds().split_last() else {
        return false;
    };
    compound_matches(subject, element) && rest_matches(rest, subject.combinator(), element)
}

/// `combinator` joins the last compound of `rest` to `element`.
fn rest_matches(rest: &[CompoundSelector], combinator: Combinator, element: &Element<'_>) -> bool {
    let Some((compound, before)) = rest.split_last() else {
        return true;
    };
    let joins = |candidate: &Element<'_>| {
        compound_matches(compound, candidate)
            && rest_matches(before, compound.combinator(), candidate)
    };
    match combinator {
        Combinator::Child => parent(element).is_some_and(|parent| joins(&parent)),
        Combinator::Descendant => ancestors(element).any(|ancestor| joins(&ancestor)),
        Combinator::None => false,
    }
}

fn compound_matches(compound: &CompoundSelector, element: &Element<'_>) -> bool {
    compound.simples().iter().all(|simple| match simple {
        SimpleSelector::Type(name) => element.name() == name.as_ref(),
        SimpleSelector::Class(name) => element.classes().any(|class| class == name.as_ref()),
        SimpleSelector::Id(id) => element.id() == Some(id.as_ref()),
    })
}

fn parent<'a>(element: &Element<'a>) -> Option<Element<'a>> {
    match element.parent() {
        Parent::Element(parent) => Some(parent),
        Parent::Document => None,
    }
}

fn ancestors<'a>(element: &Element<'a>) -> impl Iterator<Item = Element<'a>> {
    iter::successors(parent(element), parent)
}
