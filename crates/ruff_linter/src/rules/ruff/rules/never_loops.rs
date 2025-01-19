use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_python_ast::Stmt;
use ruff_python_semantic::cfg::{
    builder::{ControlEdge, ControlFlowGraph},
    implementations::build_cfg,
};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for loops whose body executes at most once.
///
/// ## Why is this bad?
/// If the intent was to only use the first member of the
/// iterable, if it exists, then this would be clearer if
/// handled directly using `next`. Otherwise, this may
/// be a sign that there is a bug.
///
/// ## Example
/// ```python
/// def f(iter):
///    for i in iter:
///       if i>0:
///          return 1
///       else:
///          return 2
/// ```
///
/// Use instead:
/// ```python
/// def f(iter):
///    first = next(iter,None)
///    if first is None:
///       return None
///    elif first>0:
///       return 1
///    else:
///       return 2
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct NeverLoops;

impl Violation for NeverLoops {
    #[derive_message_formats]
    fn message(&self) -> String {
        "This loop executes at most once".to_string()
    }
}

/// RUF300
pub(crate) fn never_loops(checker: &mut Checker, body: &[Stmt]) {
    for (i, stmt) in body.iter().enumerate() {
        match stmt {
            Stmt::For(_) | Stmt::While(_) => {
                let cfg = build_cfg(&body[i..i + 1]);
                let loop_guard = cfg.out(cfg.initial()).targets().next().unwrap();
                let loop_body = cfg.out(loop_guard).targets().next().unwrap();
                if cfg
                    .out(loop_body)
                    .targets()
                    .find(|tgt| tgt == &loop_guard)
                    .is_none()
                {
                    dbg!(&cfg);
                    checker
                        .diagnostics
                        .push(Diagnostic::new(NeverLoops, stmt.range()));
                }
            }
            _ => {
                continue;
            }
        }
    }
}
