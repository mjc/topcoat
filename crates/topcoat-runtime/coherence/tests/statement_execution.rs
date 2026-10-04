use std::{
    cell::{Cell, RefCell},
    ops::AsyncFnOnce,
};

use topcoat::runtime::{Expr, Surrogate, Surrogated, expr};
use topcoat_runtime_coherence::{Awaitable, Case, coherent};

fn check_trace<F, S>(compiled: Expr<F>, expected: &str)
where
    F: FnOnce() -> S,
    S: Surrogate<Real = String>,
{
    let (run, js) = compiled.into_evaluated_and_js();
    let checked = Expr::evaluate(
        || {
            move || {
                let actual = run().into_real();
                assert_eq!(actual, expected);
                actual.into_surrogate()
            }
        },
        js,
    );
    Case::deferred("statement trace", checked).assert();
}

fn check_async_trace<F, S>(compiled: Expr<F>, expected: &str)
where
    F: AsyncFnOnce() -> S,
    S: Surrogate<Real = String>,
{
    let (run, js) = compiled.into_evaluated_and_js();
    let checked = Expr::evaluate(
        || {
            async move || {
                let actual = run().await.into_real();
                assert_eq!(actual, expected);
                actual.into_surrogate()
            }
        },
        js,
    );
    Case::asynchronous("async statement trace", checked)
        .unwrap()
        .assert();
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
