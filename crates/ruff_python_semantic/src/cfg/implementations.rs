use crate::cfg::builder::{CFGBuilder, Condition, ControlEdge, ControlFlowGraph, TryContext};
use ruff_index::{newtype_index, IndexVec};
use ruff_python_ast::Stmt;

use super::builder::TryKind;

pub fn build_cfg(stmts: &[Stmt]) -> CFG<'_> {
    let mut builder = CFGConstructor::with_capacity(stmts.len());
    builder.process_stmts(stmts);
    builder.build()
}

#[newtype_index]
pub struct BlockId;

#[derive(Debug, Default, Clone)]
pub struct NextBlock<'stmt> {
    conditions: Vec<Condition<'stmt>>,
    targets: Vec<BlockId>,
}

impl<'stmt> ControlEdge<'stmt> for NextBlock<'stmt> {
    type Block = BlockId;

    fn always(target: Self::Block) -> Self {
        Self {
            conditions: vec![Condition::Always],
            targets: vec![target],
        }
    }

    fn switch(conditions: Vec<(Condition<'stmt>, Self::Block)>) -> Self {
        let (conditions, targets): (Vec<_>, Vec<_>) = conditions.into_iter().unzip();
        Self {
            conditions,
            targets,
        }
    }

    fn targets(&self) -> impl Iterator<Item = Self::Block> + ExactSizeIterator {
        self.targets.iter().copied()
    }

    fn conditions(&self) -> impl Iterator<Item = Condition<'stmt>> {
        self.conditions.iter().cloned()
    }
}

#[derive(Debug, Default)]
pub enum BlockKind {
    #[default]
    Generic,
    LoopGuard,
    ExceptionDispatch,
    Recovery,
    Terminal,
}

#[derive(Debug, Default)]
struct BlockData<'stmt> {
    kind: BlockKind,
    stmts: Vec<&'stmt Stmt>,
    out: NextBlock<'stmt>,
    parents: Vec<BlockId>,
}

#[derive(Debug)]
pub struct CFG<'stmt> {
    blocks: IndexVec<BlockId, BlockData<'stmt>>,
    initial: BlockId,
    terminal: BlockId,
}

impl<'stmt> CFG<'stmt> {
    pub fn kind(&self, block: BlockId) -> &BlockKind {
        &self.blocks[block].kind
    }
}

impl<'stmt> ControlFlowGraph<'stmt> for CFG<'stmt> {
    type Block = BlockId;
    type Edge = NextBlock<'stmt>;

    fn initial(&self) -> Self::Block {
        self.initial
    }

    fn terminal(&self) -> Self::Block {
        self.terminal
    }

    fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    fn stmts(&self, block: Self::Block) -> impl IntoIterator<Item = &'stmt Stmt> {
        self.blocks[block].stmts.clone()
    }

    fn outgoing(&self, block: Self::Block) -> &Self::Edge {
        &self.blocks[block].out
    }

    fn predecessors(
        &self,
        block: Self::Block,
    ) -> impl IntoIterator<Item = Self::Block> + ExactSizeIterator {
        self.blocks[block].parents.iter().copied()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LoopContext {
    guard: BlockId,
    exit: BlockId,
}

impl LoopContext {
    pub fn new(guard: BlockId, exit: BlockId) -> Self {
        Self { guard, exit }
    }
}

#[derive(Debug)]
pub struct CFGConstructor<'stmt> {
    cfg: CFG<'stmt>,
    current: BlockId,
    current_exit: BlockId,
    loop_contexts: Vec<LoopContext>,
    try_contexts: Vec<TryContext<'stmt>>,
}

impl<'stmt> CFGBuilder<'stmt> for CFGConstructor<'stmt> {
    type BasicBlock = BlockId;
    type Edge = NextBlock<'stmt>;
    type Graph = CFG<'stmt>;

    fn new() -> Self {
        Self::with_capacity(16) // reasonable default capacity
    }

    fn with_capacity(capacity: usize) -> Self {
        let mut blocks = IndexVec::with_capacity(capacity);
        let initial = blocks.push(BlockData::default());
        let terminal = blocks.push(BlockData {
            kind: BlockKind::Terminal,
            ..BlockData::default()
        });

        Self {
            cfg: CFG {
                blocks,
                initial,
                terminal,
            },
            current: initial,
            current_exit: terminal,
            loop_contexts: Vec::new(),
            try_contexts: Vec::new(),
        }
    }

    fn current(&self) -> Self::BasicBlock {
        self.current
    }

    fn current_exit(&self) -> Self::BasicBlock {
        self.current_exit
    }

    fn terminal(&self) -> Self::BasicBlock {
        self.cfg.terminal
    }

    fn update_exit(&mut self, new_exit: Self::BasicBlock) {
        self.current_exit = new_exit;
    }

    fn push_stmt(&mut self, stmt: &'stmt Stmt) {
        self.cfg.blocks[self.current].stmts.push(stmt);
    }

    fn move_to(&mut self, block: Self::BasicBlock) {
        self.current = block;
    }

    fn new_block(&mut self) -> Self::BasicBlock {
        self.cfg.blocks.push(BlockData::default())
    }

    fn new_loop_guard(&mut self, _stmt: &'stmt Stmt) -> Self::BasicBlock {
        // For now, just create a new block - we might want to store the
        // stmt association later for analysis
        self.cfg.blocks.push(BlockData {
            kind: BlockKind::LoopGuard,
            ..BlockData::default()
        })
    }

    fn add_edge(&mut self, edge: Self::Edge) {
        // I don't think we should ever be overwriting an existing edge...
        // debug_assert!(self.cfg.blocks[self.current].out.targets.is_empty());
        // debug_assert!(self.cfg.blocks[self.current].out.conditions.is_empty());
        for &target in &edge.targets {
            self.cfg.blocks[target].parents.push(self.current)
        }
        self.cfg.blocks[self.current].out = edge;
    }

    fn loop_exit(&self) -> Self::BasicBlock {
        self.loop_contexts
            .last()
            .expect("Syntax error to have `break` or `continue` outside of a loop")
            .exit
    }

    fn build(self) -> Self::Graph {
        self.cfg
    }

    fn at_terminal(&self) -> bool {
        self.current() == self.terminal()
    }
    fn at_exit(&self) -> bool {
        self.current() == self.current_exit()
    }

    fn out(&self, block: Self::BasicBlock) -> &Self::Edge {
        self.cfg.outgoing(block)
    }

    fn new_exception_dispatch(&mut self) -> Self::BasicBlock {
        self.cfg.blocks.push(BlockData {
            kind: BlockKind::ExceptionDispatch, // New kind
            ..BlockData::default()
        })
    }

    fn push_try_context(&mut self, kind: TryKind) {
        self.try_contexts.push(TryContext::new(kind));
    }

    fn last_try_context(&self) -> Option<&TryContext<'stmt>> {
        self.try_contexts.last()
    }

    fn last_mut_try_context(&mut self) -> Option<&mut TryContext<'stmt>> {
        self.try_contexts.last_mut()
    }

    fn pop_try_context(&mut self) -> Option<TryContext<'stmt>> {
        self.try_contexts.pop()
    }

    fn new_recovery(&mut self) -> Self::BasicBlock {
        self.cfg.blocks.push(BlockData {
            kind: BlockKind::Recovery, // New kind
            ..BlockData::default()
        })
    }

    fn loop_guard(&self) -> Self::BasicBlock {
        self.loop_contexts
            .last()
            .expect("Must be inside loop for `continue`.")
            .guard
    }

    fn push_loop(&mut self, guard: Self::BasicBlock, exit: Self::BasicBlock) {
        self.loop_contexts.push(LoopContext::new(guard, exit));
    }

    fn pop_loop(&mut self) -> Option<(Self::BasicBlock, Self::BasicBlock)> {
        let Some(ctxt) = self.loop_contexts.pop() else {
            return None;
        };
        Some((ctxt.guard, ctxt.exit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::implementations::build_cfg;
    use ruff_python_parser::parse_module;

    #[test]
    fn test_empty_function() {
        let source = "def empty(): pass";
        let module = parse_module(source).unwrap();
        if let Stmt::FunctionDef(func) = &module.into_syntax().body[0] {
            let cfg = build_cfg(&func.body);

            // Should have initial (with pass) and terminal blocks
            assert_eq!(cfg.num_blocks(), 2);

            // Initial block should have pass statement and edge to terminal
            let initial = cfg.initial();
            let stmts: Vec<_> = cfg.stmts(initial).into_iter().collect();
            assert_eq!(stmts.len(), 1);
            assert!(matches!(stmts[0], Stmt::Pass(_)));

            let out = cfg.outgoing(initial);
            assert_eq!(out.targets.len(), 1);
            assert_eq!(out.targets[0], cfg.terminal());
        } else {
            panic!("Expected function definition");
        }
    }

    #[test]
    fn test_simple_return() {
        let source = "def foo(): return 42";
        let module = parse_module(source).unwrap();
        if let Stmt::FunctionDef(func) = &module.into_syntax().body[0] {
            let cfg = build_cfg(&func.body);

            // Should have initial (with return) and terminal blocks
            assert_eq!(cfg.num_blocks(), 2);

            // Initial block should have return statement
            let initial = cfg.initial();
            let stmts: Vec<_> = cfg.stmts(initial).into_iter().collect();
            assert_eq!(stmts.len(), 1);
            assert!(matches!(stmts[0], Stmt::Return(_)));

            // Return should go straight to terminal
            let out = cfg.outgoing(initial);
            assert_eq!(out.targets.len(), 1);
            assert_eq!(out.targets[0], cfg.terminal());
        } else {
            panic!("Expected function definition");
        }
    }

    #[test]
    fn test_if_statement() {
        let source = r#"
def foo():
    if x > 0:
        return 1
    else:
        return 2
"#;
        let module = parse_module(source).unwrap();
        if let Stmt::FunctionDef(func) = &module.into_syntax().body[0] {
            let cfg = build_cfg(&func.body);

            // Should have: initial (with condition), if block, else block, terminal
            assert_eq!(cfg.num_blocks(), 4);

            let initial = cfg.initial();
            let initial_out = cfg.outgoing(initial);

            // Initial block should branch to two blocks
            assert_eq!(initial_out.conditions.len(), 2);
            assert_eq!(initial_out.targets.len(), 2);

            // Both targets should contain return statements
            for &target in &initial_out.targets {
                let stmts: Vec<_> = cfg.stmts(target).into_iter().collect();
                assert_eq!(stmts.len(), 1);
                assert!(matches!(stmts[0], Stmt::Return(_)));

                // Each should go to terminal
                let out = cfg.outgoing(target);
                assert_eq!(out.targets.len(), 1);
                assert_eq!(out.targets[0], cfg.terminal());
            }
        } else {
            panic!("Expected function definition");
        }
    }
}
