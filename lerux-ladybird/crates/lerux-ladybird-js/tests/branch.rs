//! The host check for the script engine.
//!
//! This test crate links `lerux-ladybird-js` only.

use lerux_ladybird_js::Value;

const PICK: &str = "\
let bonus = 2;
function pick(n) {
  if (n) {
    return n + bonus;
  } else {
    return 0;
  }
}
";

#[test]
fn function_branch_returns_an_integer() {
    assert_number(&format!("{PICK}pick(5);"), 7.0);
    assert_number(&format!("{PICK}pick(0);"), 0.0);
}

#[test]
fn syntax_error_is_returned() {
    let error = lerux_ladybird_js::run("function (").expect_err("syntax");
    assert!(error.is_syntax());
}

#[test]
fn runtime_error_does_not_panic() {
    let error = lerux_ladybird_js::run("1 + true;").expect_err("runtime");
    assert!(error.is_runtime());
}

#[test]
fn arithmetic_uses_precedence() {
    let source = "\
function pick(n) {
  if (n) { return -2 + 3 * 3; }
  return 0;
}
pick(1);
";
    assert_number(source, 7.0);
}

#[test]
fn recursion_branches_and_collects() {
    let source = "\
function down(n) {
  if (n) { return n + down(n - 1); }
  return 0;
}
down(20);
";
    assert_number(source, 210.0);
}

#[test]
fn sibling_call_reads_an_outer_let() {
    let source = "\
function inc(n) { return n + 1; }
function pick(n) {
  if (n) { return inc(n); }
  return 0;
}
pick(4);
";
    assert_number(source, 5.0);
}

#[test]
fn block_let_is_local_to_the_arm() {
    let source = "\
function pick(n) {
  if (n) {
    let bonus = 3;
    return n + bonus;
  }
  return 0;
}
pick(4);
";
    assert_number(source, 7.0);
}

#[test]
fn string_boolean_and_function_results() {
    assert_eq!(
        run_ok("function pick(n) { if (n) { return \"yes\"; } return \"no\"; } pick(1);"),
        Value::String("yes".into())
    );
    assert_eq!(
        run_ok("function pick(n) { if (n) { return true; } return false; } pick(0);"),
        Value::Boolean(false)
    );
    assert_eq!(
        run_ok("function pick(n) { if (n) { return pick; } return 0; } pick(1);"),
        Value::Object
    );
    assert_eq!(
        run_ok("function pick(n) { if (n) { return undefined; } return 1; } pick(1);"),
        Value::Undefined
    );
}

#[test]
fn unicode_scalar_name_branches() {
    let source = "\
function λ(n) {
  if (n) { return n + 1; }
  return 0;
}
λ(2);
";
    assert_number(source, 3.0);
}

#[test]
fn comment_and_escape_do_not_panic() {
    let source = "\
// true arm
function pick(n) {
  /* branch */
  if (n) { return \"a\\n\\u0042\"; }
  return \"\";
}
pick(1);
";
    assert_eq!(run_ok(source), Value::String("a\nB".into()));
}

#[test]
fn unknown_name_is_a_syntax_error() {
    let source = "\
function pick(n) {
  if (n) { return bonus; }
  return 0;
}
";
    assert!(lerux_ladybird_js::run(source)
        .expect_err("name")
        .is_syntax());
}

fn assert_number(source: &str, expected: f64) {
    let Value::Number(number) = run_ok(source) else {
        panic!("number");
    };
    assert_eq!(number.to_bits(), expected.to_bits());
}

fn run_ok(source: &str) -> Value {
    lerux_ladybird_js::run(source).expect("run")
}
