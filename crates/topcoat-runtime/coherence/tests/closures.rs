use topcoat::runtime::expr;
use topcoat_runtime_coherence::{Awaitable, Case, coherent};

#[test]
fn local_closure_calls_preserve_arguments() {
    let zero = expr!({
        let ready = || true;
        ready()
    });
    assert!(zero.clone().into_evaluated_and_js().0);
    Case::evaluated("zero arguments", zero).assert();

    let one = expr!({
        let identity = |value| value;
        identity("sentinel".to_owned())
    });
    assert_eq!(one.clone().into_evaluated_and_js().0, "sentinel");
    Case::evaluated("one argument", one).assert();

    let two = expr!({
        let second = |_first, second| second;
        second("first".to_owned(), "sentinel".to_owned())
    });
    assert_eq!(two.clone().into_evaluated_and_js().0, "sentinel");
    Case::evaluated("two arguments", two).assert();
}

#[test]
fn typed_local_closures_use_vocabulary_types() {
    let compiled = expr!({
        let identity = |value: String| -> String { value };
        identity("sentinel".to_owned())
    });
    assert_eq!(compiled.clone().into_evaluated_and_js().0, "sentinel");
    Case::evaluated("typed identity", compiled).assert();
    coherent!({
        let length = |text: &str| -> usize { text.len() };
        length("sentinel")
    });
    coherent!({
        let identity = |value: Option<String>| -> Option<String> { value };
        identity(Some("sentinel".to_owned()))
    });
    coherent!({
        let identity = |value: ()| -> () { value };
        identity(())
    });
    coherent!({
        let length = |text: String| -> usize { raw!("${text}.len()", text.len()) };
        length("sentinel".to_owned())
    });
}

#[test]
fn mutable_local_closures_can_be_called_repeatedly() {
    let runs = &mut 0.0;
    coherent!({
        raw!("globalThis.closureRuns = 0;", ());
        let mut next = || {
            raw!("cx.hydrate(++globalThis.closureRuns)", {
                *runs += 1.0;
                *runs
            })
        };
        let first = next();
        (first, next())
    });
}

#[test]
fn local_closures_preserve_scope_and_call_semantics() {
    let suffix = "!".to_owned();
    coherent!({
        let append = || suffix.clone();
        (append(), append())
    });
    coherent!({
        let identity = |value| value;
        let alias = identity;
        (alias)(42)
    });
    coherent!({
        let identity = {
            let run = |value| value;
            run
        };
        identity(42)
    });
    for condition in [false, true] {
        coherent!({
            let identity = |value| value;
            let chosen = if condition { identity } else { identity };
            chosen(42)
        });
    }
    coherent!({
        let value = "moved".to_owned();
        let consume = || value;
        consume()
    });
    coherent!({
        let outer = || true;
        let inner = {
            let outer = || false;
            outer()
        };
        (inner, outer())
    });
    coherent!((|first, second| (first, second))(1, 2));
}

#[test]
fn local_closure_returns_and_loop_jumps_stay_in_their_scope() {
    coherent!({
        let echo = |value: String| -> String {
            while true {
                break;
            }
            return value;
        };
        let first = echo("first".to_owned());
        let second = echo("second".to_owned());
        (first, second)
    });
}

#[test]
fn nested_async_closures_preserve_arguments_after_suspension() {
    let ready = Awaitable::ready("sentinel".to_owned()).after_yield();
    coherent!(async => {
        let run = async |first, second| {
            let identity = |value| value;
            (identity(first), second.await)
        };
        run("first".to_owned(), ready).await
    });
}

#[test]
fn local_async_closures_return_after_suspension() {
    let ready = Awaitable::ready("sentinel".to_owned()).after_yield();
    coherent!(async => {
        let run = async |value| {
            let result = value.await;
            return result;
        };
        run(ready).await
    });
}

#[test]
fn unawaited_local_async_closures_do_not_run() {
    let runs = &std::cell::Cell::new(0.0);
    coherent!(async => {
        raw!("globalThis.closureRuns = 0;", ());
        let work = async || {
            raw!("globalThis.closureRuns += 1;", runs.set(runs.get() + 1.0));
        };
        let _pending = work();
        raw!("cx.hydrate(globalThis.closureRuns)", runs.get())
    });
}
