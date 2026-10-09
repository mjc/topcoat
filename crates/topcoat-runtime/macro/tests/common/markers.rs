use serde_json::Value;

/// Parses the final signal declaration directly from its HTML comment.
pub(super) fn last_signal(html: &str) -> Value {
    let marker = "<!--::topcoat::signal(";
    let start = html.rfind(marker).expect(html) + marker.len();
    let end = html[start..].find(")-->").expect(html) + start;
    serde_json::from_str(&html[start..end]).expect(html)
}

/// Returns the id of the final signal declaration.
pub(super) fn last_signal_id(html: &str) -> String {
    last_signal(html)["id"]
        .as_str()
        .expect("signal id is a string")
        .to_owned()
}
