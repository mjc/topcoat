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
                    for branch in 0..8 {
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
                            let branch_semicolon = if branch >= 5 { ";" } else { semicolon };
                            let branch = match branch {
                                0 => format!("if {condition} {body}"),
                                1 => format!("if !{condition} {{}} else {body}"),
                                2 => format!("if false {{}} else if {condition} {body} else {{}}"),
                                3 => format!("if {condition} {{ if true {body} }}"),
                                4 => format!("{{ if {condition} {body} }}"),
                                5..=7 => {
                                    let depth = [1, 2, 4][branch - 5];
                                    format!(
                                        "{}if {condition} {body}{}",
                                        "(".repeat(depth),
                                        ")".repeat(depth)
                                    )
                                }
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

#[test]
fn nested_parenthesized_statements_preserve_jump_targets() {
    let mut engine = Engine::new(Duration::from_secs(5));
    for asynchronous in [false, true] {
        let prefix = if asynchronous { "async " } else { "" };
        for depth in [1, 2, 4] {
            for (statement, expected) in [
                ("raw!(\"function () {}\", ())", 5.0),
                ("raw!(\"{ first: 7, second: 9 }\", ())", 5.0),
                ("if true { break; }", 9.0),
                ("{ break; }", 9.0),
                ("while false {}", 5.0),
                ("loop { break; }", 5.0),
                ("return 7.0", 7.0),
            ] {
                let statement = format!("{}{statement}{}", "(".repeat(depth), ")".repeat(depth));
                check(
                    &mut engine,
                    &format!("{prefix}|| {{ loop {{ {statement}; return 5.0; }} 9.0 }}"),
                    expected,
                    asynchronous,
                );
            }
        }
    }
}

#[test]
fn value_position_jumps_preserve_iteration_traces() {
    let mut engine = Engine::new(Duration::from_secs(5));
    for asynchronous in [false, true] {
        for loop_kind in ["while", "loop"] {
            for jump in ["break", "continue", "return"] {
                for at in [1, 2, 4, 5] {
                    for form in 0..4 {
                        let condition = if asynchronous {
                            format!(
                                "raw!(\"Promise.resolve(cx.hydrate((trace.push('check:' + step), step === {at})))\", true).await"
                            )
                        } else {
                            format!(
                                "raw!(\"cx.hydrate((trace.push('check:' + step), step === {at}))\", true)"
                            )
                        };
                        let action = if jump == "return" {
                            "return raw!(\"cx.hydrate(trace.concat('return:' + step).join(','))\", String::new())".to_owned()
                        } else {
                            format!("raw!(\"trace.push('{jump}:' + step);\", ()); {jump}")
                        };
                        let expression = match form {
                            0 => format!("if {condition} {{ {action}; }} else {{ 9.0 }}"),
                            1 => format!("{{ if {condition} {{ {action}; }} 9.0 }}"),
                            2 => format!("(({{ {{ if {condition} {{ {action}; }} 9.0 }} }}))"),
                            _ => format!(
                                "(if false {{ 11.0 }} else if {condition} {{ {action}; }} else {{ 9.0 }})"
                            ),
                        };
                        let contents = format!(
                            "raw!(\"trace.push('visit:' + step);\", ()); let _value = {expression}; raw!(\"trace.push('after:' + step);\", ());"
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
                            "{prefix}|| {{ raw!(\"let step = 0; let trace = [];\", ()); {loop_source}; raw!(\"trace.push('done:' + step);\", ()); raw!(\"cx.hydrate(trace.join(','))\", String::new()) }}"
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
                            trace.push(format!("after:{step}"));
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

#[test]
fn nested_closure_jumps_remain_local_to_each_invocation() {
    let mut engine = Engine::new(Duration::from_secs(5));
    for source in [
        "|| { let callback = || { let value = if true { return 7.0; } else { 9.0 }; value + 1.0 }; raw!(\"${callback}().add(${callback}())\", 14.0) }",
        "|| { let callback = || loop { let _value = if true { break 7.0; } else { 9.0 }; }; raw!(\"${callback}().add(${callback}())\", 14.0) }",
        "|| { let outer = || loop { let callback = || { let _value = if true { return 5.0; } else { 9.0 }; 11.0 }; break raw!(\"${callback}().add(cx.hydrate(2.0))\", 7.0); }; raw!(\"${outer}().add(${outer}())\", 14.0) }",
    ] {
        check(&mut engine, source, 14.0, false);
    }
    check(
        &mut engine,
        "async || { let callback = async || { let _value = if raw!(\"Promise.resolve(cx.hydrate(true))\", true).await { return 7.0; } else { 9.0 }; 11.0 }; raw!(\"(await ${callback}()).add(await ${callback}())\", 14.0) }",
        14.0,
        true,
    );
}

#[test]
fn jump_markers_are_private_to_async_invocations() {
    let mut engine = Engine::new(Duration::from_secs(5));
    check(
        &mut engine,
        "async || { let callback = async |value| { let _value = if raw!(\"Promise.resolve(cx.hydrate(true))\", true).await { return value; } else { 9.0 }; 11.0 }; raw!(\"(await Promise.all([${callback}(cx.hydrate(5.0)), ${callback}(cx.hydrate(7.0))])).reduce((sum, value) => sum.add(value), cx.hydrate(0.0))\", 12.0) }",
        12.0,
        true,
    );
}

#[test]
fn jump_catchers_rethrow_foreign_objects_unchanged() {
    let mut engine = Engine::new(Duration::from_secs(5));
    for asynchronous in [false, true] {
        let prefix = if asynchronous { "async " } else { "" };
        let call = if asynchronous {
            "await ${callback}()"
        } else {
            "${callback}()"
        };
        let error_source = if asynchronous {
            "Promise.reject(sentinel)"
        } else {
            "(() => { throw sentinel; })()"
        };
        let error = format!(
            "raw!({error_source:?}, 9.0){}",
            if asynchronous { ".await" } else { "" }
        );
        let caller = format!(
            "{}({prefix}() => {{ try {{ {call}; return cx.hydrate(11.0); }} catch (error) {{ return cx.hydrate(error === sentinel && !inspected ? 5.0 : 9.0); }} }})()",
            if asynchronous { "await " } else { "" }
        );
        let source = format!(
            "{prefix}|| {{ raw!(\"let inspected = false; let sentinel = {{ get value() {{ inspected = true; throw new Error('inspected'); }} }};\", ()); let callback = {prefix}|| {{ let value = if false {{ return 7.0; }} else {{ {error} }}; value }}; raw!({caller:?}, 5.0) }}"
        );
        check(&mut engine, &source, 5.0, asynchronous);
    }
}
