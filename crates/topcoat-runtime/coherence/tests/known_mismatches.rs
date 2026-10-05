use topcoat::runtime::{Expr, Surrogate, Surrogated, expr};
use topcoat_runtime_coherence::{Case, Observe, coherent};

#[test]
fn captured_nan() {
    let value = f64::NAN;
    coherent!(known "captured_nan" => value);
}

#[test]
fn captured_infinity() {
    let value = f64::INFINITY;
    coherent!(known "captured_infinity" => value);
}

#[test]
fn some_unit() {
    coherent!(known "some_unit" => true.then_some({}).is_some());
}

#[test]
fn loop_break_value() {
    let (run, js) = expr!(|| {
        let value = "loop value";
        loop {
            break value;
        }
    })
    .into_evaluated_and_js();
    let value = run().into_real();
    assert_eq!(value.observe(), "loop value".observe());
    Case::deferred(
        "value-producing loop",
        Expr::evaluate(|| move || value.into_surrogate(), js),
    )
    .assert_known("loop_break_value");
}

#[test]
fn return_in_value_conditional() {
    coherent!(known "return_in_value_conditional" => {
        let value = if true { return 7.0; } else { 9.0 };
        value + 1.0
    });
}

#[test]
fn break_in_value_conditional() {
    coherent!(known "break_in_value_conditional" => {
        loop {
            let _value = if true { break; } else { 9.0 };
        }
        7.0
    });
}
