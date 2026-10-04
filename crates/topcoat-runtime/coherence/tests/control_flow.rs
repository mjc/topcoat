use topcoat_runtime_coherence::coherent;

#[test]
fn branching() {
    for condition in [false, true] {
        coherent!(if condition { 1.0 } else { 2.0 });
        coherent!(if condition { "yes" } else { "no" });
        coherent!(if condition {
            1.0
        } else if !condition {
            2.0
        } else {
            3.0
        });
        coherent!(direct => if condition { "yes" } else { "no" });
    }
    let missing: Option<f64> = None;
    coherent!(if true { 1.0 } else { missing.unwrap() });
    coherent!(if false { missing.unwrap() } else { 1.0 });
}

#[test]
fn blocks_and_shadowing() {
    let value = 3.0;
    coherent!({
        let value = value + 1.0;
        value * 2.0
    });
    coherent!({
        let value = value + 1.0;
        let value = value * 2.0;
        value
    });
    coherent!({
        let inner = {
            let value = 9.0;
            value + 1.0
        };
        inner + value
    });
    coherent!(direct => { let value = value + 1.0; value * 2.0 });
}

#[test]
fn bounded_loops() {
    coherent!({
        while false {}
        3.0
    });
    coherent!({
        loop {
            break;
        }
        3.0
    });
}

#[test]
fn conditional_break_preserves_the_loop() {
    coherent!({
        while true {
            if true {
                break;
            };
        }
        3.0
    });
    for condition in [false, true] {
        coherent!({
            loop {
                if condition {
                    if true {
                        break;
                    }
                } else if !condition {
                    break;
                } else {
                    break;
                }
            }
            3.0
        });
    }
}

#[test]
fn conditional_continue_skips_the_rest_of_the_iteration() {
    // raw! supplies the same bounded counter in each language without adding
    // assignment syntax to the expression grammar.
    let step = std::cell::Cell::new(0.0);
    coherent!({
        raw!("let step = 0;", step.set(0.0));
        while raw!("cx.hydrate(step++ < 4)", {
            let current = step.get();
            step.set(current + 1.0);
            current < 4.0
        }) {
            if raw!("cx.hydrate(step < 3)", step.get() < 3.0) {
                continue;
            }
            return raw!("cx.hydrate(step)", step.get());
        }
        9.0
    });
}

#[test]
fn loop_body_discards_tail_values() {
    let step = std::cell::Cell::new(0.0);
    coherent!({
        raw!("let step = 0;", step.set(0.0));
        while raw!("cx.hydrate(step++ < 3)", {
            let current = step.get();
            step.set(current + 1.0);
            current < 3.0
        }) {
            if true {
            } else {
            }
        }
        raw!("cx.hydrate(step)", step.get())
    });
}

#[test]
fn statement_blocks_preserve_jump_targets() {
    coherent!({
        loop {
            {
                if true {
                    break;
                }
            }
        }
        3.0
    });
    coherent!({
        {
            if true {
                return 1.0;
            }
        }
        2.0
    });
    coherent!({
        loop {
            loop {
                if true {
                    break;
                }
            }
            break;
        }
        3.0
    });
}

#[test]
fn return_inside_if() {
    coherent!({
        if true {
            return 1.0;
        }
        2.0
    });
}

#[test]
fn break_inside_if_inside_loop() {
    coherent!({
        loop {
            if true {
                break;
            }
            break;
        }
        3.0
    });
}

#[test]
fn break_inside_if_inside_while() {
    coherent!({
        while true {
            if true {
                break;
            }
            break;
        }
        3.0
    });
}

#[test]
fn continue_inside_if_inside_loop() {
    // An untaken continue must still produce valid JavaScript.
    coherent!({
        loop {
            if false {
                continue;
            }
            break;
        }
        3.0
    });
}

#[test]
fn continue_inside_if_inside_while() {
    coherent!({
        while true {
            if false {
                continue;
            }
            break;
        }
        3.0
    });
}
