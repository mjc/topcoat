use super::js::Js;

/// Tracks Rust jump targets across generated JavaScript functions.
#[derive(Default)]
pub(super) struct ControlFlow {
    depth: usize,
    next_target: usize,
    functions: Vec<Target>,
    loops: Vec<Target>,
}

impl ControlFlow {
    pub(super) fn enter_function(&mut self, root: bool) {
        self.enter_value();
        let target = self.target(false, root);
        self.functions.push(target);
    }

    pub(super) fn leave_function(&mut self) -> Target {
        self.leave_value();
        self.functions.pop().expect("active function")
    }

    pub(super) fn enter_value(&mut self) {
        self.depth += 1;
    }

    pub(super) fn leave_value(&mut self) {
        self.depth -= 1;
    }

    pub(super) fn enter_loop(&mut self, returns_value: bool) {
        let target = self.target(returns_value, false);
        self.loops.push(target);
    }

    pub(super) fn leave_loop(&mut self) -> Target {
        self.loops.pop().expect("active loop")
    }

    pub(super) fn loop_jump(&mut self) -> Option<Jump> {
        let start = self.functions.last().map_or(0, |target| target.loop_start);
        if self.loops.len() <= start {
            return None;
        }
        Some(self.loops.last_mut()?.jump(self.depth))
    }

    pub(super) fn return_jump(&mut self) -> Jump {
        self.functions
            .last_mut()
            .expect("active function")
            .jump(self.depth)
    }

    fn target(&mut self, returns_value: bool, root: bool) -> Target {
        let marker = format!("__control{}", self.next_target);
        self.next_target += 1;
        Target {
            marker,
            depth: self.depth,
            loop_start: self.loops.len(),
            returns_value,
            root,
            escapes: false,
        }
    }
}

pub(super) struct Target {
    marker: String,
    depth: usize,
    loop_start: usize,
    returns_value: bool,
    root: bool,
    pub(super) escapes: bool,
}

impl Target {
    fn jump(&mut self, depth: usize) -> Jump {
        let marker = if depth == self.depth {
            None
        } else {
            self.escapes = true;
            Some(self.marker.clone())
        };
        Jump {
            marker,
            returns_value: self.returns_value,
            root: self.root,
        }
    }

    pub(super) fn declaration(&self, js: &mut Js) {
        if self.escapes {
            // Catch only this invocation's marker; runtime failures pass through.
            js.push_str(&format!("const {} = {{}}; ", self.marker));
        }
    }

    pub(super) fn function_body(&self, body: Js, js: &mut Js, is_block: bool) {
        if !self.escapes {
            js.append(body);
            return;
        }
        js.push_str("{ ");
        self.declaration(js);
        js.push_str("try ");
        if is_block {
            js.append(body);
        } else {
            js.push_str("{ return (");
            js.append(body);
            js.push_str("); }");
        }
        js.push_str(&format!(
            " catch (__jump) {{ if (__jump === {}) return {}.value; throw __jump; }} }}",
            self.marker, self.marker,
        ));
    }

    pub(super) fn loop_body(&self, body: Js, js: &mut Js) {
        if !self.escapes {
            js.append(body);
            return;
        }
        js.push_str("{ try ");
        js.append(body);
        js.push_str(&format!(
            " catch (__jump) {{ if (__jump === {}) {{ if ({}.continuing) continue; ",
            self.marker, self.marker,
        ));
        if self.returns_value {
            js.push_str(&format!("return {}.value; ", self.marker));
        } else {
            js.push_str("break; ");
        }
        js.push_str("} throw __jump; } }");
    }
}

pub(super) struct Jump {
    pub(super) marker: Option<String>,
    pub(super) returns_value: bool,
    pub(super) root: bool,
}
