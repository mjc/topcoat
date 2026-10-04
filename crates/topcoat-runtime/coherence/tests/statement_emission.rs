use std::time::Duration;

use syn::{Expr as SynExpr, visit::Visit};
use topcoat_runtime_coherence::{Engine, Observe, Outcome};
use topcoat_runtime_grammar::expr::Expr;

#[derive(Default)]
struct Source {
    javascript: String,
}

impl<'ast> Visit<'ast> for Source {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        // Visit the builder receiver first to preserve source-part order.
        syn::visit::visit_expr_method_call(self, call);
        if call.method == "source" {
            let SynExpr::Lit(literal) = &call.args[0] else {
                panic!("expected a source literal");
            };
            let syn::Lit::Str(source) = &literal.lit else {
                panic!("expected a source string");
            };
            self.javascript.push_str(&source.value());
        }
        assert_ne!(call.method, "expression", "fixtures must not have captures");
    }
}

fn check(engine: &mut Engine, source: &str, expected: impl Observe, asynchronous: bool) {
    let expression: Expr =
        syn::parse_str(source).unwrap_or_else(|error| panic!("{source}\n{error}"));
    let expansion = syn::parse2(expression.expr_to_tokens().unwrap()).unwrap();
    let mut emitted = Source::default();
    emitted.visit_expr(&expansion);
    assert!(!emitted.javascript.is_empty());
    let javascript = serde_json::to_string(&emitted.javascript).unwrap();
    let call = if asynchronous {
        format!("TopcoatCoherence.executeAsync({javascript})")
    } else {
        format!("TopcoatCoherence.execute({javascript}, true)")
    };
    let script = format!(
        "{}\n{call}",
        include_str!("../../browser/dist/coherence.js"),
    );
    let result = if asynchronous {
        engine.evaluate_async(&script)
    } else {
        engine.evaluate(&script)
    }
    .unwrap_or_else(|error| panic!("{source}\n{}\n{error:#}", emitted.javascript));
    let actual: Outcome = serde_json::from_str(&result).unwrap();
    assert_eq!(
        actual,
        Outcome::Return(expected.observe()),
        "{source}\n{}",
        emitted.javascript
    );
}

#[test]
fn jumps_preserve_iteration_traces() {
    let mut engine = Engine::new(Duration::from_secs(5));
    for asynchronous in [false, true] {
        for loop_kind in ["while", "loop"] {
            for jump in ["break", "continue", "return"] {
                for at in [1, 2, 4, 5] {
                    for branch in 0..6 {
                        for semicolon in ["", ";"] {
                            let condition = format!(
                                "raw!(\"cx.hydrate((trace.push('check:' + step), step === {at}))\", true)"
                            );
                            let action = if jump == "return" {
                                "return raw!(\"cx.hydrate(trace.concat('return:' + step).join(','))\", String::new())".to_owned()
                            } else {
                                format!("raw!(\"trace.push('{jump}:' + step);\", ()); {jump}")
                            };
                            let body = format!("{{ {action}{semicolon} }}");
                            let branch_semicolon = if branch == 5 { ";" } else { semicolon };
                            let branch = match branch {
                                0 => format!("if {condition} {body}"),
                                1 => format!("if !{condition} {{}} else {body}"),
                                2 => format!("if false {{}} else if {condition} {body} else {{}}"),
                                3 => format!("if {condition} {{ if true {body} }}"),
                                4 => format!("{{ if {condition} {body} }}"),
                                5 => format!("(if {condition} {body})"),
                                _ => unreachable!(),
                            };
                            for following in [false, true] {
                                let after = if following {
                                    "raw!(\"trace.push('after:' + step);\", ());"
                                } else {
                                    ""
                                };
                                let contents = format!(
                                    "raw!(\"trace.push('visit:' + step);\", ()); {branch}{branch_semicolon} {after}"
                                );
                                let loop_source = if loop_kind == "while" {
                                    format!(
                                        "while raw!(\"cx.hydrate(step++ < 4)\", false) {{ {contents} }}"
                                    )
                                } else {
                                    format!(
                                        "loop {{ if raw!(\"cx.hydrate(step++ >= 4)\", false) {{ break; }} {contents} }}"
                                    )
                                };
                                let prefix = if asynchronous { "async " } else { "" };
                                let source = format!(
                                    "{prefix}|| {{ raw!(\"let step = 0; let trace = [];\", ());
                                 {loop_source}{semicolon}
                                 raw!(\"trace.push('done:' + step);\", ());
                                 raw!(\"cx.hydrate(trace.join(','))\", String::new()) }}"
                                );
                                let mut trace = Vec::new();
                                let mut step = 0;
                                let mut returned = false;
                                while step < 4 {
                                    step += 1;
                                    trace.push(format!("visit:{step}"));
                                    trace.push(format!("check:{step}"));
                                    if step == at {
                                        trace.push(format!("{jump}:{step}"));
                                        match jump {
                                            "break" => break,
                                            "continue" => continue,
                                            "return" => {
                                                returned = true;
                                                break;
                                            }
                                            _ => unreachable!(),
                                        }
                                    }
                                    if following {
                                        trace.push(format!("after:{step}"));
                                    }
                                }
                                if !returned {
                                    if step == 4 && !(jump == "break" && at == 4) {
                                        step += 1;
                                    }
                                    trace.push(format!("done:{step}"));
                                }
                                check(&mut engine, &source, trace.join(","), asynchronous);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn parenthesized_tail_jumps_and_loops_execute() {
    let mut engine = Engine::new(Duration::from_secs(5));
    for asynchronous in [false, true] {
        let prefix = if asynchronous { "async " } else { "" };
        for tail in ["return 7.0", "(return 7.0)", "(((return 7.0)))"] {
            check(
                &mut engine,
                &format!("{prefix}|| {{ {tail} }}"),
                7.0,
                asynchronous,
            );
        }
        for tail in ["return 7.0", "(return 7.0)"] {
            check(
                &mut engine,
                &format!("{prefix}|| {tail}"),
                7.0,
                asynchronous,
            );
        }
        for tail in ["while false {}", "(while false {})", "(loop { break })"] {
            check(
                &mut engine,
                &format!("{prefix}|| {{ {tail} }}"),
                (),
                asynchronous,
            );
            check(&mut engine, &format!("{prefix}|| {tail}"), (), asynchronous);
        }
    }
}
