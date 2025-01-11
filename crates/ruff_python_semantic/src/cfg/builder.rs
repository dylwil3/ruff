use ruff_python_ast::{ExceptHandler, Expr, MatchCase, Stmt};

/// Represents a condition to be tested in a multi-way branch
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
    ExceptHandler(&'stmt ExceptHandler),
    /// A fallback case (else/default/finally)
    Always,
}

pub trait ControlEdge<'stmt> {
    type Block: Copy;

    /// Creates an unconditional edge to the target block
    fn always(target: Self::Block) -> Self;

    /// Creates a multi-way branch based on conditions
    fn switch(conditions: Vec<(Condition<'stmt>, Self::Block)>) -> Self;
}

/// A trait for building Control Flow Graphs (CFG).
/// Implementations of this trait can construct CFGs by adding basic blocks,
/// statements, and edges while maintaining loop context.
pub trait CFGBuilder<'stmt> {
    type BasicBlock: Copy;
    type Edge: ControlEdge<'stmt, Block = Self::BasicBlock>;

    /// Creates a new CFG builder, creating initial and terminal blocks internally.
    fn new() -> Self;

    /// Creates a new CFG builder with initial capacity hint for internal collections.
    fn with_capacity(capacity: usize) -> Self;

    /// Returns the current basic block being constructed.
    fn current(&mut self) -> Self::BasicBlock;

    /// Returns the current exit block for the scope being processed.
    fn current_exit(&mut self) -> Self::BasicBlock;

    /// Returns the terminal block of the CFG.
    /// This is the block that return statements will target.
    fn terminal(&mut self) -> Self::BasicBlock;

    /// Updates the current exit block.
    fn update_exit(&mut self, new_exit: Self::BasicBlock);

    /// Adds a statement to the current basic block.
    fn push_stmt(&mut self, stmt: &'stmt Stmt);

    /// Changes the current working block to the specified block.
    fn move_to(&mut self, block: Self::BasicBlock);

    /// Creates a new basic block.
    fn new_block(&mut self) -> Self::BasicBlock;

    /// Creates a new block to handle entering and exiting a loop body.
    fn new_loop_guard(&mut self, stmt: &'stmt Stmt) -> Self::BasicBlock;

    /// Adds an outgoing edge from the current block to the target specified in the edge.
    fn add_edge(&mut self, edge: Self::Edge);

    /// Creates basic blocks and edges from a sequence of statements.
    fn process_stmts(&mut self, stmts: impl IntoIterator<Item = &'stmt Stmt>) {
        let mut stmts = stmts.into_iter().peekable();

        // If we have any statements, create a new block for them
        // since we're likely starting at the initial block
        if stmts.peek().is_some() {
            let new_block = self.new_block();
            let edge = Self::Edge::always(new_block);
            self.add_edge(edge);
            self.move_to(new_block);
        }

        while let Some(stmt) = stmts.next() {
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
                    let next_block = self.new_block();

                    // Create the loop guard block with the test
                    let guard = self.new_loop_guard(stmt);

                    // Create a block for the loop body
                    let body = self.new_block();

                    // Set up break/continue targets
                    self.push_loop_exit(next_block);

                    // Add the conditional edge from guard
                    let conditions = vec![
                        (Condition::Test(&stmt_while.test), body),
                        (Condition::Always, next_block),
                    ];
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

                    // Add edge back to guard from wherever we ended up
                    let edge = Self::Edge::always(guard);
                    self.add_edge(edge);

                    // Clean up loop context and continue from next block
                    self.pop_loop_exit();
                    self.move_to(next_block);
                }
                Stmt::For(stmt_for) => {
                    // Create a new block for any following statements
                    let next_block = self.new_block();

                    // Create the loop guard block with the iterator
                    let guard = self.new_loop_guard(stmt);

                    // Create a block for the loop body
                    let body = self.new_block();

                    // Set up break/continue targets
                    self.push_loop_exit(next_block);

                    // Add the conditional edge from guard
                    let conditions = vec![
                        (
                            Condition::Iterator {
                                target: &stmt_for.target,
                                iter: &stmt_for.iter,
                                is_async: stmt_for.is_async,
                            },
                            body,
                        ),
                        (Condition::Always, next_block),
                    ];
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

                    // Add edge back to guard from wherever we ended up
                    let edge = Self::Edge::always(guard);
                    self.add_edge(edge);

                    // Clean up loop context and continue from next block
                    self.pop_loop_exit();
                    self.move_to(next_block);
                }

                // Switch statements
                Stmt::If(stmt_if) => {
                    // Create a new block for any following statements
                    let next_block = self.new_block();

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
                            conditions.push((Condition::Always, *block));
                        }
                    }

                    // If no else clause was present, add fallthrough to next block
                    if clause_blocks.is_empty()
                        || stmt_if.elif_else_clauses.last().unwrap().test.is_some()
                    {
                        conditions.push((Condition::Always, next_block));
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
                    let next_block = self.new_block();

                    // Create a vec of conditions and their target blocks
                    let mut conditions = Vec::new();

                    // Create blocks for each case
                    let case_blocks: Vec<_> = stmt_match
                        .cases
                        .iter()
                        .map(|case| (case, self.new_block()))
                        .collect();

                    // Add conditions for each case
                    for (case, block) in &case_blocks {
                        conditions.push((
                            Condition::Match {
                                subject: &stmt_match.subject,
                                case,
                            },
                            *block,
                        ));
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
                Stmt::Try(stmt_try) => todo!(),
                Stmt::With(stmt_with) => todo!(),

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
                Stmt::Raise(_) => todo!(),
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
                Stmt::Assert(stmt_assert) => todo!(),
            }
        }

        // End by connecting the current block to the terminal block
        let edge = Self::Edge::always(self.terminal());
        self.add_edge(edge);
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
}
