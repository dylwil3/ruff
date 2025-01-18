use crate::cfg::builder::{CFGBuilder, Condition, ControlEdge, ControlFlowGraph};
use ruff_index::{newtype_index, IndexVec};
use ruff_python_ast::Stmt;

pub fn build_cfg(stmts: &[Stmt]) -> CFG<'_> {
    let mut builder = CFGConstructor::with_capacity(stmts.len());
    builder.process_stmts(stmts);
    builder.build()
}

#[newtype_index]
pub struct BlockId;

#[derive(Debug, Clone)]
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

#[derive(Debug)]
struct BlockData<'stmt> {
    stmts: Vec<&'stmt Stmt>,
    out: NextBlock<'stmt>,
}

impl<'stmt> BlockData<'stmt> {
    fn new() -> Self {
        Self {
            stmts: Vec::new(),
            out: NextBlock {
                conditions: Vec::new(),
                targets: Vec::new(),
            },
        }
    }
}

#[derive(Debug)]
pub struct CFG<'stmt> {
    blocks: IndexVec<BlockId, BlockData<'stmt>>,
    initial: BlockId,
    terminal: BlockId,
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

    fn out(&self, block: Self::Block) -> &Self::Edge {
        &self.blocks[block].out
    }
}

#[derive(Debug)]
pub struct CFGConstructor<'stmt> {
    cfg: CFG<'stmt>,
    current: BlockId,
    current_exit: BlockId,
    loop_exits: Vec<BlockId>,
}

impl<'stmt> CFGConstructor<'stmt> {
    fn with_capacity(capacity: usize) -> Self {
        let mut blocks = IndexVec::with_capacity(capacity);
        let initial = blocks.push(BlockData::new());
        let terminal = blocks.push(BlockData::new());

        Self {
            cfg: CFG {
                blocks,
                initial,
                terminal,
            },
            current: initial,
            current_exit: terminal,
            loop_exits: Vec::new(),
        }
    }
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
        let initial = blocks.push(BlockData::new());
        let terminal = blocks.push(BlockData::new());

        Self {
            cfg: CFG {
                blocks,
                initial,
                terminal,
            },
            current: initial,
            current_exit: terminal,
            loop_exits: Vec::new(),
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
        self.cfg.blocks.push(BlockData::new())
    }

    fn new_loop_guard(&mut self, _stmt: &'stmt Stmt) -> Self::BasicBlock {
        // For now, just create a new block - we might want to store the
        // stmt association later for analysis
        self.new_block()
    }

    fn add_edge(&mut self, edge: Self::Edge) {
        // I don't think we should ever be overwriting an existing edge...
        debug_assert!(self.cfg.blocks[self.current].out.targets.is_empty());
        debug_assert!(self.cfg.blocks[self.current].out.conditions.is_empty());
        self.cfg.blocks[self.current].out = edge;
    }

    fn loop_exit(&self) -> Self::BasicBlock {
        *self
            .loop_exits
            .last()
            .expect("Syntax error to have `break` or `continue` outside of a loop")
    }

    fn push_loop_exit(&mut self, exit: Self::BasicBlock) {
        self.loop_exits.push(exit);
    }

    fn pop_loop_exit(&mut self) -> Self::BasicBlock {
        self.loop_exits.pop().expect("loop exit stack empty")
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
        self.cfg.out(block)
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

            let out = cfg.out(initial);
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
            let out = cfg.out(initial);
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
            let initial_out = cfg.out(initial);

            // Initial block should branch to two blocks
            assert_eq!(initial_out.conditions.len(), 2);
            assert_eq!(initial_out.targets.len(), 2);

            // Both targets should contain return statements
            for &target in &initial_out.targets {
                let stmts: Vec<_> = cfg.stmts(target).into_iter().collect();
                assert_eq!(stmts.len(), 1);
                assert!(matches!(stmts[0], Stmt::Return(_)));

                // Each should go to terminal
                let out = cfg.out(target);
                assert_eq!(out.targets.len(), 1);
                assert_eq!(out.targets[0], cfg.terminal());
            }
        } else {
            panic!("Expected function definition");
        }
    }
}
