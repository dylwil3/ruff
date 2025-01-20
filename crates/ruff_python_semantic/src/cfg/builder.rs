use std::fmt;

use ruff_python_ast::{ExceptHandler, ExceptHandlerExceptHandler, Expr, MatchCase, Stmt};

pub trait ControlFlowGraph<'stmt> {
    type Block: Copy;
    type Edge: ControlEdge<'stmt, Block = Self::Block>;

    fn initial(&self) -> Self::Block;

    fn terminal(&self) -> Self::Block;

    fn num_blocks(&self) -> usize;

    /// Get all statements in a block
    fn stmts(&self, block: Self::Block) -> impl IntoIterator<Item = &'stmt Stmt>;

    /// Get outgoing edge from block
    /// (Note that an `Edge` actually represents multiple edges... confusingly
    /// we should probably change the name.)
    fn outgoing(&self, block: Self::Block) -> &Self::Edge;

    fn predecessors(
        &self,
        block: Self::Block,
    ) -> impl IntoIterator<Item = Self::Block> + ExactSizeIterator;
}

/// Represents a condition to be tested in a multi-way branch
#[derive(Debug, Clone)]
pub enum Condition<'stmt> {
    /// A boolean test expression
    Test(&'stmt Expr),
    /// A match case with its subject expression
    Match {
        subject: &'stmt Expr,
        case: &'stmt MatchCase,
    },
    /// A test for iterator exhaustion
    Iterator {
        target: &'stmt Expr,
        iter: &'stmt Expr,
        is_async: bool,
    },
    /// An except handler for try/except blocks
    ExceptHandler(&'stmt ExceptHandlerExceptHandler),
    /// An uncaught exception
    UncaughtException,
    /// A fallback case (else/wildcard case/etc.)
    Else,
    /// Unconditional edge
    Always,
}

pub trait ControlEdge<'stmt> {
    type Block: Copy;

    /// Creates an unconditional edge to the target block
    fn always(target: Self::Block) -> Self;

    /// Creates a multi-way branch based on conditions
    fn switch(conditions: Vec<(Condition<'stmt>, Self::Block)>) -> Self;

    fn targets(&self) -> impl Iterator<Item = Self::Block> + ExactSizeIterator;

    fn conditions(&self) -> impl Iterator<Item = Condition<'stmt>>;
}

/// A trait for building Control Flow Graphs (CFG).
/// Implementations of this trait can construct CFGs by adding basic blocks,
/// statements, and edges while maintaining loop context.
pub trait CFGBuilder<'stmt> {
    type BasicBlock: fmt::Debug + Copy;
    type Edge: ControlEdge<'stmt, Block = Self::BasicBlock>;
    type Graph: ControlFlowGraph<'stmt, Block = Self::BasicBlock, Edge = Self::Edge>;

    /// Creates a new CFG builder, creating initial and terminal blocks internally.
    fn new() -> Self;

    /// Creates a new CFG builder with initial capacity hint for internal collections.
    fn with_capacity(capacity: usize) -> Self;

    /// Returns the current basic block being constructed.
    fn current(&self) -> Self::BasicBlock;

    /// Returns the current exit block for the scope being processed.
    fn current_exit(&self) -> Self::BasicBlock;

    /// Returns the terminal block of the CFG.
    /// This is the block that return statements will target.
    fn terminal(&self) -> Self::BasicBlock;

    fn at_terminal(&self) -> bool;
    fn at_exit(&self) -> bool;

    /// Updates the current exit block.
    fn update_exit(&mut self, new_exit: Self::BasicBlock);

    /// Adds a statement to the current basic block.
    fn push_stmt(&mut self, stmt: &'stmt Stmt);

    /// Changes the current working block to the specified block.
    fn move_to(&mut self, block: Self::BasicBlock);

    /// Creates a new basic block.
    fn new_block(&mut self) -> Self::BasicBlock;

    /// Creates a new block if there are more statements to process,
    /// otherwise returns the current exit block
    fn next_or_exit<I>(&mut self, stmts: &mut std::iter::Peekable<I>) -> Self::BasicBlock
    where
        I: Iterator<Item = &'stmt Stmt>,
    {
        if stmts.peek().is_some() {
            self.new_block()
        } else {
            self.current_exit()
        }
    }

    /// Creates a new block to handle entering and exiting a loop body.
    fn new_loop_guard(&mut self, stmt: &'stmt Stmt) -> Self::BasicBlock;

    /// Creates a new block to handle dispatching control flow at the end
    /// of a `try` block.
    fn new_exception_dispatch(&mut self) -> Self::BasicBlock;

    /// Adds an outgoing edge from the current block to the target specified in the edge.
    fn add_edge(&mut self, edge: Self::Edge);

    /// Get outgoing edge from block
    /// (Note that an `Edge` actually represents multiple edges... confusingly
    /// we should probably change the name.)
    fn out(&self, block: Self::BasicBlock) -> &Self::Edge;

    /// Creates basic blocks and edges from a sequence of statements.
    fn process_stmts(&mut self, stmts: impl IntoIterator<Item = &'stmt Stmt>) {
        let mut stmts = stmts.into_iter().peekable();

        while let Some(stmt) = stmts.next() {
            // Save current exit
            let cache_exit = self.current_exit();
            match stmt {
                Stmt::FunctionDef(_)
                | Stmt::ClassDef(_)
                | Stmt::Assign(_)
                | Stmt::AugAssign(_)
                | Stmt::AnnAssign(_)
                | Stmt::TypeAlias(_)
                | Stmt::Import(_)
                | Stmt::ImportFrom(_)
                | Stmt::Global(_)
                | Stmt::Nonlocal(_)
                | Stmt::Expr(_)
                | Stmt::Pass(_)
                | Stmt::Delete(_)
                | Stmt::IpyEscapeCommand(_) => {
                    self.push_stmt(stmt);
                }
                // Loops
                Stmt::While(stmt_while) => {
                    // Create a new block for any following statements
                    let next_block = self.next_or_exit(&mut stmts);

                    // Create the loop guard block with the test,
                    // and traverse unconditional edge to it.
                    let guard = self.new_loop_guard(stmt);
                    self.add_edge(Self::Edge::always(guard));
                    self.move_to(guard);

                    // Create a block for the loop body
                    let body = self.new_block();

                    // Set up break/continue targets
                    self.push_loop_exit(next_block);

                    // Add the conditional edge from guard
                    let (conditions, else_block) = if stmt_while.orelse.is_empty() {
                        // No else clause - fail straight to next block
                        (
                            vec![
                                (Condition::Test(&stmt_while.test), body),
                                (Condition::Else, next_block),
                            ],
                            None,
                        )
                    } else {
                        // Create else block and route normal exit through it
                        let else_block = self.new_block();
                        (
                            vec![
                                (Condition::Test(&stmt_while.test), body),
                                (Condition::Else, else_block),
                            ],
                            Some(else_block),
                        )
                    };
                    let edge = Self::Edge::switch(conditions);
                    self.add_edge(edge);

                    // Save the current exit for later
                    let old_exit = self.current_exit();

                    // Process loop body
                    self.move_to(body);
                    self.update_exit(guard); // continue and natural loop end go to guard
                    self.process_stmts(&stmt_while.body);

                    // Restore the old exit
                    self.update_exit(old_exit);

                    // Process else clause if it exists
                    if let Some(else_block) = else_block {
                        self.move_to(else_block);
                        self.update_exit(next_block);
                        self.process_stmts(&stmt_while.orelse);
                    }

                    // Clean up loop context and continue from next block
                    self.pop_loop_exit();
                    self.move_to(next_block);
                }
                Stmt::For(stmt_for) => {
                    // Create a new block for any following statements
                    let next_block = self.next_or_exit(&mut stmts);

                    // Create the loop guard block with the iterator
                    let guard = self.new_loop_guard(stmt);
                    self.add_edge(Self::Edge::always(guard));
                    self.move_to(guard);

                    // Create blocks for the loop body and else clause
                    let body = self.new_block();

                    // Set up break/continue targets
                    // break jumps directly to next_block, skipping else clause
                    self.push_loop_exit(next_block);

                    // Add the conditional edge from guard
                    let (conditions, else_block) = if stmt_for.orelse.is_empty() {
                        (
                            vec![
                                (
                                    Condition::Iterator {
                                        target: &stmt_for.target,
                                        iter: &stmt_for.iter,
                                        is_async: stmt_for.is_async,
                                    },
                                    body,
                                ),
                                (Condition::Else, next_block),
                            ],
                            None,
                        )
                    } else {
                        let else_block = self.new_block();
                        (
                            vec![
                                (
                                    Condition::Iterator {
                                        target: &stmt_for.target,
                                        iter: &stmt_for.iter,
                                        is_async: stmt_for.is_async,
                                    },
                                    body,
                                ),
                                // Normal loop exit goes to else clause
                                (Condition::Else, else_block),
                            ],
                            Some(else_block),
                        )
                    };

                    let edge = Self::Edge::switch(conditions);
                    self.add_edge(edge);

                    // Save the current exit for later
                    let old_exit = self.current_exit();

                    // Process loop body
                    self.move_to(body);
                    self.update_exit(guard); // continue and natural loop end go to guard
                    self.process_stmts(&stmt_for.body);

                    // Restore the old exit
                    self.update_exit(old_exit);

                    // Process else clause with next_block as its exit
                    if let Some(else_block) = else_block {
                        self.move_to(else_block);
                        self.update_exit(next_block);
                        self.process_stmts(&stmt_for.orelse);
                    }

                    // Clean up loop context and continue from next block
                    self.pop_loop_exit();
                    self.move_to(next_block);
                }

                // Switch statements
                Stmt::If(stmt_if) => {
                    // Create a new block for any following statements
                    let next_block = self.next_or_exit(&mut stmts);

                    // Create a vec of conditions and their target blocks
                    let mut conditions = Vec::new();

                    // Add the initial if branch
                    let if_block = self.new_block();
                    conditions.push((Condition::Test(&stmt_if.test), if_block));

                    // Create blocks for each elif/else clause
                    let clause_blocks: Vec<_> = stmt_if
                        .elif_else_clauses
                        .iter()
                        .map(|clause| (clause, self.new_block()))
                        .collect();

                    // Add conditions for each elif/else clause
                    for (clause, block) in &clause_blocks {
                        if let Some(test) = &clause.test {
                            // elif clause
                            conditions.push((Condition::Test(test), *block));
                        } else {
                            // else clause (must be last)
                            conditions.push((Condition::Else, *block));
                        }
                    }

                    // If no else clause was present, add fallthrough to next block
                    if clause_blocks.is_empty()
                        || stmt_if.elif_else_clauses.last().unwrap().test.is_some()
                    {
                        conditions.push((Condition::Else, next_block));
                    }

                    // Save the current exit for later
                    let old_exit = self.current_exit();

                    // Add the switch edge from current to all branches
                    let edge = Self::Edge::switch(conditions);
                    self.add_edge(edge);

                    // Process if body
                    self.move_to(if_block);
                    self.update_exit(next_block);
                    self.process_stmts(&stmt_if.body);

                    // Process each elif/else body
                    for (clause, block) in clause_blocks {
                        self.move_to(block);
                        self.process_stmts(&clause.body);
                    }

                    // Restore the old exit
                    self.update_exit(old_exit);

                    // Continue from next_block
                    self.move_to(next_block);
                }
                Stmt::Match(stmt_match) => {
                    // Create a new block for any following statements
                    let next_block = self.next_or_exit(&mut stmts);

                    // Create a vec of conditions and their target blocks
                    let mut conditions = Vec::new();

                    // Create blocks for each case
                    let case_blocks: Vec<_> = stmt_match
                        .cases
                        .iter()
                        .map(|case| (case, self.new_block()))
                        .collect();

                    // Add conditions for each case
                    let mut has_wildcard = false;
                    for (case, block) in &case_blocks {
                        if case.pattern.is_wildcard() {
                            has_wildcard = true
                        }
                        conditions.push((
                            Condition::Match {
                                subject: &stmt_match.subject,
                                case,
                            },
                            *block,
                        ));
                    }

                    // If the last condition was not a wildcard
                    // add an "else" edge to the next block
                    if !has_wildcard {
                        conditions.push((Condition::Else, next_block))
                    }

                    // Save the current exit for later
                    let old_exit = self.current_exit();

                    // Add the switch edge from current to all cases
                    let edge = Self::Edge::switch(conditions);
                    self.add_edge(edge);

                    // Process each case's body
                    for (case, block) in case_blocks {
                        self.move_to(block);
                        self.update_exit(next_block);
                        self.process_stmts(&case.body);
                    }

                    // Restore the old exit
                    self.update_exit(old_exit);

                    // Continue from next_block
                    self.move_to(next_block);
                }
                Stmt::Try(stmt_try) => {
                    // This is almost identical to a match statement,
                    // but with some subtleties regarding the possible
                    // `finally` block, and the need to update
                    // the context stack.

                    // Save the current exit for later
                    let old_exit = self.current_exit();

                    // First we move to a new block for the try
                    // statement: even though execution unconditionally
                    // flows into this block, we will often need
                    // to run dataflow analyses differently within
                    // a try block.
                    let try_block = self.new_block();
                    self.add_edge(Self::Edge::always(try_block));
                    self.move_to(try_block);

                    // Next, we add a block, similar to the loop guard,
                    // for handling control flow as we exit the `try`-block
                    let dispatch_block = self.new_exception_dispatch();
                    self.update_exit(dispatch_block);
                    self.process_stmts(&stmt_try.body);
                    self.move_to(dispatch_block);
                    self.update_exit(old_exit);

                    // Create a new block for any following statements
                    let next_block = self.next_or_exit(&mut stmts);

                    // If there is a `finally` block, it will sometimes
                    // replace the role of the terminal block and,
                    // at other times, replace the role of the `next_block`
                    let maybe_finally_block = if stmt_try.finalbody.is_empty() {
                        None
                    } else {
                        Some(self.new_block())
                    };

                    // Create a vec of conditions and their target blocks
                    let mut conditions = Vec::new();

                    // Create blocks for each case
                    let except_blocks: Vec<_> = stmt_try
                        .handlers
                        .iter()
                        .map(|ExceptHandler::ExceptHandler(handler)| (handler, self.new_block()))
                        .collect();

                    // Add conditions for each case
                    let mut has_bare_except = false;
                    for (handler, block) in &except_blocks {
                        if handler.type_.is_none() {
                            has_bare_except = true
                        }
                        conditions.push((Condition::ExceptHandler(handler), *block));
                    }

                    // Add `else` if present
                    let maybe_else_block = if stmt_try.orelse.is_empty() {
                        None
                    } else {
                        Some(self.new_block())
                    };

                    // If no exception is raised, flow continues
                    // to `else` (if present), otherwise to `finally`
                    // (if present), otherwise to the next block.
                    match (maybe_else_block, maybe_finally_block) {
                        (Some(else_block), _) => conditions.push((Condition::Else, else_block)),
                        (_, Some(finally_block)) => {
                            conditions.push((Condition::Else, finally_block))
                        }
                        (None, None) => conditions.push((Condition::Else, next_block)),
                    }

                    // If there are no catchall except handlers,
                    // add edge to indicate the other exceptions
                    // will direct flow to `finally` (where they
                    // will then be re-raised unless masked by an
                    // early jump).
                    if !has_bare_except && maybe_finally_block.is_some() {
                        conditions
                            .push((Condition::UncaughtException, maybe_finally_block.unwrap()))
                    }

                    // Add the switch edge from current to all cases
                    let edge = Self::Edge::switch(conditions);
                    self.add_edge(edge);

                    // Process each case's body
                    for (handler, block) in except_blocks {
                        self.move_to(block);
                        self.update_exit(maybe_finally_block.unwrap_or(next_block));
                        self.process_stmts(&handler.body);
                    }

                    // Process the `else` clause if present
                    if let Some(else_block) = maybe_else_block {
                        self.move_to(else_block);
                        self.update_exit(maybe_finally_block.unwrap_or(next_block));
                        self.process_stmts(&stmt_try.orelse);
                    }

                    // Process the `finally` clause if present
                    if let Some(finally_block) = maybe_finally_block {
                        self.move_to(finally_block);
                        self.update_exit(next_block);
                        self.process_stmts(&stmt_try.finalbody);
                    }

                    // Restore the old exit
                    self.update_exit(old_exit);

                    // Continue from next_block
                    self.move_to(next_block);
                }
                Stmt::With(_) => {
                    self.push_stmt(stmt);
                }

                // Jumps
                Stmt::Return(_) => {
                    self.push_stmt(stmt);
                    let edge = Self::Edge::always(self.terminal());
                    self.add_edge(edge);

                    if stmts.peek().is_some() {
                        let next_block = self.new_block();
                        self.move_to(next_block);
                    }
                }
                Stmt::Break(_) => {
                    self.push_stmt(stmt);
                    let edge = Self::Edge::always(self.loop_exit());
                    self.add_edge(edge);

                    if stmts.peek().is_some() {
                        let next_block = self.new_block();
                        self.move_to(next_block);
                    }
                }

                // TODO
                Stmt::Raise(_) => {
                    self.push_stmt(stmt);
                }

                Stmt::Continue(_) => {
                    self.push_stmt(stmt);
                    // We should only be processing a `continue` while
                    // inside a loop body. We will already have updated the
                    // `current_exit` to the loop guard before descending into
                    // the loop body.
                    let edge = Self::Edge::always(self.current_exit());
                    self.add_edge(edge);

                    if stmts.peek().is_some() {
                        let next_block = self.new_block();
                        self.move_to(next_block);
                    }
                }
                // Assert is sort of a mixture of a switch and a jump,
                // so handled as such
                // TODO
                Stmt::Assert(_) => {
                    self.push_stmt(stmt);
                }
            }
            // Restore exit
            self.update_exit(cache_exit);
            // If we have an outgoing edge, move to exit
            if self.out(self.current()).conditions().next().is_some() {
                self.move_to(self.current_exit());
            }
        }

        // End by connecting the current block to the exit if necessary.
        if !self.at_exit() {
            let edge = Self::Edge::always(self.current_exit());
            self.add_edge(edge);
        }
    }

    /// Returns the current loop exit block without removing it.
    fn loop_exit(&self) -> Self::BasicBlock;

    /// Pushes a block onto the loop exit stack.
    /// This block represents where control should flow when encountering a
    /// 'break' statement within a loop.
    fn push_loop_exit(&mut self, exit: Self::BasicBlock);

    /// Pops and returns the most recently pushed loop exit block.
    /// This is called when finishing the processing of a loop construct.
    fn pop_loop_exit(&mut self) -> Self::BasicBlock;

    fn build(self) -> Self::Graph;
}
