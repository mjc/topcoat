use std::{
    cell::{Cell, RefCell},
    ops::AsyncFnOnce,
    panic::catch_unwind,
};

use topcoat::runtime::{Expr, Surrogate, Surrogated, expr};
use topcoat_runtime_coherence::{Awaitable, Case, coherent};

fn check_trace<F, S>(compiled: Expr<F>, expected: &str)
where
    F: FnOnce() -> S,
    S: Surrogate<Real = String>,
{
    let (run, js) = compiled.into_evaluated_and_js();
    let completed = Cell::new(false);
    let checked = Expr::evaluate(
        || {
            let completed = &completed;
            move || {
                let actual = run().into_real();
                assert_eq!(actual, expected);
                completed.set(true);
                actual.into_surrogate()
            }
        },
        js,
    );
    Case::deferred("statement trace", checked).assert();
    // Matching panics must not hide a failed Rust trace assertion.
    assert!(completed.get(), "Rust trace must complete successfully");
}

fn check_async_trace<F, S>(compiled: Expr<F>, expected: &str)
where
    F: AsyncFnOnce() -> S,
    S: Surrogate<Real = String>,
{
    let (run, js) = compiled.into_evaluated_and_js();
    let completed = Cell::new(false);
    let checked = Expr::evaluate(
        || {
            let completed = &completed;
            async move || {
                let actual = run().await.into_real();
                assert_eq!(actual, expected);
                completed.set(true);
                actual.into_surrogate()
            }
        },
        js,
    );
    Case::asynchronous("async statement trace", checked)
        .unwrap()
        .assert();
    // Matching panics must not hide a failed Rust trace assertion.
    assert!(completed.get(), "Rust trace must complete successfully");
}

#[test]
fn captured_thresholds_preserve_continue_side_effects() {
    for at in [1, 2, 4, 5] {
        let step = Cell::new(0.0);
        let trace = RefCell::new(Vec::<String>::new());
        let limit = 4.0;
        let expected = (1..=4)
            .filter(|&step| step != at)
            .map(|step| step.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let at = f64::from(at);
        check_trace(
            expr!(|| {
                raw!("let step = 0; let trace = [];", ());
                while raw!("cx.hydrate(step++ < 4)", {
                    let current = step.get();
                    step.set(current + 1.0);
                    current < limit
                }) {
                    if raw!("cx.hydrate(step)", step.get()) == at {
                        continue;
                    }
                    raw!(
                        "trace.push(String(step));",
                        trace.borrow_mut().push(step.get().to_string())
                    );
                }
                raw!("cx.hydrate(trace.join(','))", trace.borrow().join(","))
            }),
            &expected,
        );
    }
}

#[test]
fn nested_loops_target_the_innermost_loop() {
    let outer = Cell::new(0usize);
    let inner = Cell::new(0usize);
    let trace = RefCell::new(Vec::<String>::new());
    check_trace(
        expr!(|| {
            raw!("let outer = 0; let inner = 0; let trace = [];", ());
            while raw!("cx.hydrate(outer++ < 2)", {
                let current = outer.get();
                outer.set(current + 1);
                current < 2
            }) {
                raw!("inner = 0;", inner.set(0));
                loop {
                    if raw!("cx.hydrate(inner++ >= 3)", {
                        let current = inner.get();
                        inner.set(current + 1);
                        current >= 3
                    }) {
                        break;
                    }
                    if raw!("cx.hydrate(inner === 2)", inner.get() == 2) {
                        {
                            continue;
                        }
                    } else if raw!(
                        "cx.hydrate(outer === 2 && inner === 3)",
                        outer.get() == 2 && inner.get() == 3
                    ) {
                        {
                            break;
                        }
                    }
                    raw!(
                        "trace.push(outer + ':' + inner);",
                        trace
                            .borrow_mut()
                            .push(format!("{}:{}", outer.get(), inner.get()))
                    );
                }
                raw!(
                    "trace.push('end:' + outer);",
                    trace.borrow_mut().push(format!("end:{}", outer.get()))
                );
            }
            raw!("cx.hydrate(trace.join(','))", trace.borrow().join(","))
        }),
        "1:1,1:3,end:1,2:1,end:2",
    );
}

#[test]
fn async_taken_jumps_follow_awaited_conditions() {
    let step = Cell::new(0usize);
    let trace = RefCell::new(Vec::<String>::new());
    check_async_trace(
        expr!(async || {
            raw!("let step = 0; let trace = [];", ());
            loop {
                raw!("step++;", step.set(step.get() + 1));
                if raw!("cx.hydrate(step > 4)", step.get() > 4) {
                    return "iteration limit".to_owned();
                }
                raw!(
                    "trace.push('visit:' + step);",
                    trace.borrow_mut().push(format!("visit:{}", step.get()))
                );
                if raw!("cx.hydrate(step === 1)", step.get() == 1) {
                    if raw!(
                        "Promise.resolve(cx.hydrate(true))",
                        Awaitable::ready(true).after_yield()
                    )
                    .await
                    {
                        continue;
                    }
                } else if raw!("cx.hydrate(step === 3)", step.get() == 3) {
                    if raw!(
                        "Promise.resolve(cx.hydrate(true))",
                        Awaitable::ready(true).after_yield()
                    )
                    .await
                    {
                        break;
                    }
                }
                raw!(
                    "trace.push('after:' + step);",
                    trace.borrow_mut().push(format!("after:{}", step.get()))
                );
            }
            raw!(
                "trace.push('done');",
                trace.borrow_mut().push("done".into())
            );
            raw!("cx.hydrate(trace.join(','))", trace.borrow().join(","))
        }),
        "visit:1,visit:2,after:2,visit:3,done",
    );

    let trace = RefCell::new(Vec::<String>::new());
    let failure = Awaitable::<bool>::panicking("untaken branch").after_yield();
    check_async_trace(
        expr!(async || {
            raw!("let trace = [];", ());
            if false {
                if failure.await {
                    return "unexpected".to_owned();
                }
            }
            {
                if raw!(
                    "Promise.resolve(cx.hydrate(true))",
                    Awaitable::ready(true).after_yield()
                )
                .await
                {
                    raw!(
                        "trace.push('return');",
                        trace.borrow_mut().push("return".into())
                    );
                    return raw!("cx.hydrate(trace.join(','))", trace.borrow().join(","));
                }
            }
            "fell through".to_owned()
        }),
        "return",
    );
}

#[test]
#[expect(
    path_statements,
    reason = "The test exercises discarded scoped bindings."
)]
fn statement_and_value_scopes_remain_distinct() {
    for condition in [false, true] {
        coherent!({
            let value = 3.0;
            if condition {
                let value = 5.0;
                value;
            } else {
                let value = 7.0;
                value;
            }
            {
                let value = 11.0;
                value;
            }
            let selected = if condition {
                let value = 13.0;
                value
            } else {
                let value = 17.0;
                value
            };
            value + selected
        });
        coherent!({
            if condition {
                "discarded";
            }
            let value = { if condition { 1.0 } else { 2.0 } };
            value + 3.0
        });
    }
    coherent!({ (return 7.0) });
    coherent!({ (while false {}) });
    coherent!({
        (loop {
            break;
        })
    });
    Case::deferred("concise return", expr!(|| return 7.0)).assert();
    Case::deferred(
        "concise loop",
        expr!(|| loop {
            break;
        }),
    )
    .assert();
}

#[test]
#[expect(
    unused_must_use,
    reason = "The test exercises a discarded method call."
)]
fn discarded_parenthesized_raw_expressions_keep_expression_syntax() {
    coherent!({
        (raw!("function () {}", ()));
        7.0
    });
    coherent!({
        (raw!("{ answer: 7, extra: 9 }", ()));
        7.0
    });
    coherent!({
        (raw!("{ clone: () => cx.hydrate(false) }", false).clone());
        7.0
    });
    coherent!(async => {
        (raw!("function () {}", ()));
        7.0
    });
}

#[test]
fn discarded_parenthesized_expressions_preserve_side_effects() {
    let trace = RefCell::new(Vec::<String>::new());
    check_trace(
        expr!(|| {
            raw!("let trace = [];", ());
            (raw!(
                "{ first: trace.push('first'), second: trace.push('second') }",
                {
                    trace.borrow_mut().push("first".into());
                    trace.borrow_mut().push("second".into());
                }
            ));
            (raw!("function () { trace.push('unexpected'); }", ()));
            raw!("cx.hydrate(trace.join(','))", trace.borrow().join(","))
        }),
        &["first", "second"].join(","),
    );
}

#[test]
fn discarded_block_tail_raw_expressions_keep_expression_syntax() {
    coherent!({
        {
            raw!("", ())
        };
        7.0
    });
    coherent!({
        {
            raw!("; /* empty */ // still empty\n;", ())
        };
        7.0
    });
    coherent!({
        {
            raw!("/* empty */\u{feff}// still empty\n", ())
        };
        7.0
    });
    coherent!({
        {
            raw!("/* prefix */ // still a prefix\nfunction () {}", ())
        };
        7.0
    });
    coherent!({
        {
            raw!("function () {}", ())
        };
        7.0
    });
    coherent!({
        {
            raw!("{ answer: 7, extra: 9 }", ())
        };
        7.0
    });
    coherent!(async => {
        {
            raw!("function () {}", ())
        };
        7.0
    });
}

#[test]
fn discarded_raw_tails_recognize_html_comments() {
    coherent!({
        {
            raw!("<!-- opening\n", ())
        };
        7.0
    });
    coherent!({
        {
            raw!("\n--> closing\n", ())
        };
        7.0
    });
    coherent!(async => {
        {
            raw!("<!-- opening\r\n/* middle */\n--> closing\u{2028}", ())
        };
        7.0
    });
    coherent!({
        {
            raw!("<!-- opening\nfunction () {}", ())
        };
        7.0
    });
    coherent!({
        {
            raw!("\n--> closing\n{ answer: 7, extra: 9 }", ())
        };
        7.0
    });
}

#[test]
fn discarded_raw_tails_preserve_precedence_and_side_effects() {
    let trace = RefCell::new(Vec::<String>::new());
    check_trace(
        expr!(|| {
            raw!("let trace = [];", ());
            {
                raw!(
                    "true && trace.push('and')",
                    trace.borrow_mut().push("and".into())
                )
            };
            {
                raw!(
                    "true ? trace.push('then') : trace.push('else')",
                    trace.borrow_mut().push("then".into())
                )
            };
            {
                raw!(
                    "trace.push('semicolon');",
                    trace.borrow_mut().push("semicolon".into())
                )
            };
            raw!("cx.hydrate(trace.join(','))", trace.borrow().join(","))
        }),
        &["and", "then", "semicolon"].join(","),
    );
}

#[test]
fn parenthesized_statement_forms_preserve_the_loop_target() {
    coherent!({
        while true {
            (if true {
                break;
            });
        }
        7.0
    });
    coherent!({
        loop {
            ({
                break;
            });
        }
        7.0
    });
}

#[test]
fn parenthesized_returns_preserve_sync_and_async_closure_targets() {
    coherent!({ (return 7.0) });
    coherent!(async => { (((return 7.0))) });
}

#[test]
fn trace_helpers_reject_matching_panics() {
    let missing: Option<String> = None;
    assert!(catch_unwind(|| check_trace(expr!(|| missing.unwrap()), "completed")).is_err());
}

#[test]
fn trace_helpers_reject_an_assertion_matching_a_javascript_panic() {
    let missing: Option<String> = None;
    assert!(
        catch_unwind(|| {
            check_trace(
                expr!(|| raw!("${missing}.unwrap()", "unexpected".to_owned())),
                "completed",
            );
        })
        .is_err()
    );
}

#[test]
fn trace_helpers_reject_matching_async_panics() {
    let missing: Option<String> = None;
    assert!(
        catch_unwind(|| check_async_trace(expr!(async || missing.unwrap()), "completed")).is_err()
    );
}

#[test]
fn trace_helpers_reject_an_assertion_matching_a_javascript_rejection() {
    let missing: Option<String> = None;
    assert!(
        catch_unwind(|| {
            check_async_trace(
                expr!(async || raw!("${missing}.unwrap()", "unexpected".to_owned())),
                "completed",
            );
        })
        .is_err()
    );
}

#[test]
fn value_position_continue_preserves_iteration_traces() {
    for at in [1, 2, 4, 5] {
        let step = Cell::new(0.0);
        let trace = RefCell::new(Vec::<String>::new());
        let expected = (1..=4)
            .filter(|&step| step != at)
            .map(|step| step.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let at = f64::from(at);
        check_trace(
            expr!(|| {
                raw!("let step = 0; let trace = [];", ());
                while raw!("cx.hydrate(step++ < 4)", {
                    let current = step.get();
                    step.set(current + 1.0);
                    current < 4.0
                }) {
                    let _value = {
                        if raw!("cx.hydrate(step)", step.get()) == at {
                            continue;
                        }
                        9.0
                    };
                    raw!(
                        "trace.push(String(step));",
                        trace.borrow_mut().push(step.get().to_string())
                    );
                }
                raw!("cx.hydrate(trace.join(','))", trace.borrow().join(","))
            }),
            &expected,
        );
    }
}

#[test]
fn break_payloads_evaluate_once_before_leaving_the_loop() {
    let trace = RefCell::new(Vec::<String>::new());
    check_trace(
        expr!(|| {
            raw!("let trace = [];", ());
            let value = loop {
                let _value = if true {
                    break raw!("trace.push('payload'), cx.hydrate('value')", {
                        trace.borrow_mut().push("payload".into());
                        "value".to_owned()
                    });
                } else {
                    9.0
                };
            };
            raw!(
                "cx.hydrate(trace.join(',') + ',' + ${value}.dehydrate())",
                format!("{},{}", trace.borrow().join(","), value)
            )
        }),
        "payload,value",
    );
    let trace = RefCell::new(Vec::<String>::new());
    check_trace(
        expr!(|| {
            raw!("let trace = [];", ());
            let _value = if true {
                return raw!("trace.push('return'), cx.hydrate(trace.join(','))", {
                    trace.borrow_mut().push("return".into());
                    trace.borrow().join(",")
                });
            } else {
                9.0
            };
            "unexpected".to_owned()
        }),
        "return",
    );
}

#[test]
fn async_value_position_jumps_keep_their_targets() {
    let value = Awaitable::ready("payload".to_owned()).after_yield();
    check_async_trace(
        expr!(async || {
            let result = loop {
                let _value = if true {
                    break value.await;
                } else {
                    9.0
                };
            };
            result
        }),
        "payload",
    );
    let value = Awaitable::ready("returned".to_owned()).after_yield();
    check_async_trace(
        expr!(async || {
            let _result = loop {
                let _value = if true {
                    return value.await;
                } else {
                    9.0
                };
                break 9.0;
            };
            "unexpected".to_owned()
        }),
        "returned",
    );
    let step = Cell::new(0.0);
    let trace = RefCell::new(Vec::<String>::new());
    check_async_trace(
        expr!(async || {
            raw!("let step = 0; let trace = [];", ());
            let result = loop {
                raw!("step++;", step.set(step.get() + 1.0));
                let _value = if raw!("cx.hydrate(step)", step.get()) < 3.0 {
                    if raw!(
                        "Promise.resolve(cx.hydrate(true))",
                        Awaitable::ready(true).after_yield()
                    )
                    .await
                    {
                        continue;
                    }
                    9.0
                } else {
                    11.0
                };
                raw!(
                    "trace.push(String(step));",
                    trace.borrow_mut().push(step.get().to_string())
                );
                break raw!("cx.hydrate(trace.join(','))", trace.borrow().join(","));
            };
            result
        }),
        "3",
    );
}

#[test]
#[expect(
    clippy::diverging_sub_expression,
    reason = "The test exercises jumps as initializer expressions."
)]
fn awaited_diverging_initializers_preserve_payloads() {
    let value = Awaitable::ready("break".to_owned()).after_yield();
    check_async_trace(
        expr!(async || loop {
            let _value = break value.await;
        }),
        "break",
    );
    let value = Awaitable::ready("return".to_owned()).after_yield();
    check_async_trace(
        expr!(async || {
            let _value = return value.await;
        }),
        "return",
    );
}

#[test]
fn raw_fallback_awaits_keep_value_wrappers_async() {
    check_async_trace(
        expr!(async || {
            let value = loop {
                break raw!("await Promise.resolve(cx.hydrate('payload'))", {
                    tokio::task::yield_now().await;
                    "payload".to_owned()
                });
            };
            value
        }),
        "payload",
    );
    check_async_trace(
        expr!(async || {
            let value = {
                raw!("await Promise.resolve(cx.hydrate('block'))", {
                    tokio::task::yield_now().await;
                    "block".to_owned()
                })
            };
            value
        }),
        "block",
    );
}

#[test]
fn returns_leave_awaited_while_value_expressions() {
    let value = Awaitable::ready("returned".to_owned()).after_yield();
    check_async_trace(
        expr!(async || {
            let _value = while raw!(
                "Promise.resolve(cx.hydrate(true))",
                Awaitable::ready(true).after_yield()
            )
            .await
            {
                let _selected = if true {
                    return value.await;
                } else {
                    9.0
                };
            };
            "unexpected".to_owned()
        }),
        "returned",
    );
}

#[test]
fn async_value_wrappers_leave_returned_futures_unpolled() {
    let failure = Awaitable::<f64>::panicking("unused future").after_yield();
    check_async_trace(
        expr!(async || {
            let _future = {
                raw!("${failure}", {
                    tokio::task::yield_now().await;
                    failure.clone()
                })
            };
            "completed".to_owned()
        }),
        "completed",
    );
    let failure = Awaitable::<f64>::panicking("unused future").after_yield();
    check_async_trace(
        expr!(async || {
            let _future = if true {
                raw!("${failure}", {
                    tokio::task::yield_now().await;
                    failure.clone()
                })
            } else {
                raw!("${failure}", failure.clone())
            };
            "completed".to_owned()
        }),
        "completed",
    );
    let failure = Awaitable::<f64>::panicking("unused future").after_yield();
    check_async_trace(
        expr!(async || {
            let _future = loop {
                break raw!("${failure}", {
                    tokio::task::yield_now().await;
                    failure.clone()
                });
            };
            "completed".to_owned()
        }),
        "completed",
    );
    let failure = Awaitable::<f64>::panicking("unused future").after_yield();
    check_async_trace(
        expr!(async || {
            let _future = loop {
                let _value = if true {
                    break raw!("${failure}", {
                        tokio::task::yield_now().await;
                        failure.clone()
                    });
                } else {
                    9.0
                };
            };
            "completed".to_owned()
        }),
        "completed",
    );
}

#[test]
fn boxed_future_payloads_can_be_awaited_by_the_caller() {
    let ready = Awaitable::ready("payload".to_owned()).after_yield();
    check_async_trace(
        expr!(async || {
            let future = loop {
                let _value = if true {
                    break raw!("${ready}", {
                        tokio::task::yield_now().await;
                        ready.clone()
                    });
                } else {
                    9.0
                };
            };
            future.await
        }),
        "payload",
    );
}

#[test]
fn async_value_wrappers_preserve_unit_fallthrough() {
    coherent!(async => { let value = { raw!("await Promise.resolve()", tokio::task::yield_now().await); }; value });
    coherent!(async => { let value = if false { raw!("await Promise.resolve()", tokio::task::yield_now().await); }; value });
    coherent!(async => { let value = while false { raw!("await Promise.resolve()", tokio::task::yield_now().await); }; value });
    coherent!(async => { let value = { raw!("await Promise.resolve();", tokio::task::yield_now().await); raw!("/* unit */;", ()) }; value });
}

#[test]
fn async_raw_value_tails_preserve_javascript_terminators() {
    check_async_trace(
        expr!(async || {
            let value = {
                raw!("await Promise.resolve(cx.hydrate('payload'));", {
                    tokio::task::yield_now().await;
                    "payload".to_owned()
                })
            };
            value
        }),
        "payload",
    );
    check_async_trace(
        expr!(async || {
            let value = if true {
                raw!("await Promise.resolve(cx.hydrate('payload')); // value", {
                    tokio::task::yield_now().await;
                    "payload".to_owned()
                })
            } else {
                "unexpected".to_owned()
            };
            value
        }),
        "payload",
    );
    check_async_trace(
        expr!(async || {
            let value = loop {
                break raw!(
                    "await Promise.resolve(cx.hydrate('payload')); /* value */",
                    {
                        tokio::task::yield_now().await;
                        "payload".to_owned()
                    }
                );
            };
            value
        }),
        "payload",
    );
}

#[test]
fn async_raw_value_tails_preserve_comma_expressions() {
    let ready = Awaitable::ready(0.0);
    check_async_trace(
        expr!(async || {
            let value = {
                ready.await;
                raw!("cx.hydrate(0), cx.hydrate('payload')", "payload".to_owned())
            };
            value
        }),
        "payload",
    );
}

#[test]
fn async_raw_returns_preserve_automatic_semicolon_insertion() {
    coherent!(async => { let value = { raw!("Promise.resolve()", Awaitable::ready(())).await; raw!("// value\ncx.hydrate(7)", ()) }; value });
    coherent!(async => { let value = { raw!("Promise.resolve()", Awaitable::ready(())).await; raw!("/* value\n */ cx.hydrate(7)", ()) }; value });
    coherent!(async => { let value = { raw!("Promise.resolve()", Awaitable::ready(())).await; raw!("\ncx.hydrate(7)", ()) }; value });
}
