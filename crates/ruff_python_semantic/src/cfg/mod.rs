// # Overview
//! A control flow graph is a grouping of statements into _basic blocks_
//! and edges indicating possible paths that a program may take through
//! those statements. In general, this is an _over_ approximation to the
//! actual paths that could be taken in a program. The restrictions placed
//! on which edges we draw are essentially purely syntactical. Finer analyses,
//! such as pruning based on statically known branches, are deferred.

//! To build a control-flow graph, we step through each statement in order.
//! By default, statements are added to the current basic block until we
//! reach a statement that invokes control flow. These are exactly the following:
//! | Switch  | Loops   | Jumps      | Exception Handling |
//! |---------|---------|------------|--------------------|
//! | `if`    | `for`   | `break`    | `try`              |
//! | `match` | `while` | `continue` |                    |
//! |         |         | `raise`    |                    |
//! |         |         | `return`   |                    |
//!
//!
//! There are also `assert` and `with` statements, which are syntactic sugar
//! for a combination of the above.
//!
//! We now discuss how each kind of control flow is handled.
//!
//! ## Switch Statements
//! Upon reaching a switch statement, the statement
//! itself terminates the basic block, and several outgoing
//! edges are added, labeled by the condition that needs to
//! be satisfied in order to traverse that edge. For example:
//!
//! ```python
//! if cond:
//!   f()
//! else:
//!   g()
//! ```
//! Produces:
//! ```text
//!          +--------+
//!          |  Start |
//!          +--------+
//!              |
//!              v
//!           +--------+
//!   +-------| SWITCH |
//!   |       +--------+
//!   |          |
//!   | cond     | not cond
//!   |          |
//!   v          v
//! +-----+     +-----+
//! | f() |     | g() |
//! +-----+     +-----+
//!   |          |
//!   +-----+----+
//!         |
//!       +--------+
//!       |  End   |
//!       +--------+
//! ```
//! ## Loops
//!
//! A loop consists of a _loop guard_, a _body_, and an optional
//! _else_ clause. The _loop guard_ is a condition we check to
//! determine whether to re-enter the loop body, the _body_ is again
//! a list of statements, and the _else_ clause is a possible
//! exit from the loop body or guard (depending). For simplicity,
//! let's ignore the else clause for a moment and specialize to the
//! case of `while` loops.
//!
//! When we reach a `while` loop, we begin by creating a new
//! basic block that is comprised entirely of the `loop` guard,
//! and which unconditionally follows the preceding basic block.
//! The loop guard consists of the test clause for the `while` statement.
//! This loop guard will have an outgoing node to the _loop exit_
//! and another to the loop body; the edge followed corresponds to the
//! veracity of the test clause.
//!
//! The loop body will almost always have two outgoing edges: one
//! which points back to the loop guard, and another that goes to the
//! loop exit. An exception would be the case of a jump statement, like
//! `continue` or `return`.
//!
//! For example,
//! ```python
//! while cond:
//!   continue
//! ```
//!
//! would create
//!
//! ```text
//!          +--------+
//!          |  Start |
//!          +--------+
//!              |
//!              v
//!           +-----------+
//!   +-------| LOOP GUARD|<+
//!   |       +-----------+ |
//!   |          |          |
//!   | else     | cond     |
//!   |          |          |
//!   |          v          |
//!   |      +----------+   |
//!   |      | continue |---+
//!   |      +----------+
//!   |         
//!   |         
//!   |   +--------+
//!   +-> |  End   |
//!       +--------+
//! ```
//!
//! ## Exception Handling
//!
//! Before discussing jumps, which ought to have been
//! relatively straightforward, we must introduce a wrinkle:
//! exception handling. Control flow would be much simpler
//! if it weren't for `try` statements!
//!
//! First, we have to alter our interpretation of what
//! the control flow graph actually represents when we
//! are inside of a `try` statement. Basic blocks within
//! a `try` statement no longer indicate sequences of statements
//! that are unconditionally executed one after the next.
//! Instead, we are to interpret these blocks as statements
//! that may or may not be aborted early due to an exception.
//!
//! We then introduce two "virtual" blocks which are tasked
//! with cleaning up the program state and resuming ordinary
//! control flow. They are `EXCEPTION DISPATCH` and `RECOVERY`
//! blocks, which we use in the presence of `except` and `finally`
//! clauses, respectively.
//!
//! For example:
//!
//! ```python
//! try:
//!   maybe_raise()
//! except ValueError:
//!   this()
//! except TypeError:
//!   that()
//! finally:
//!   cleanup()
//! ```
//!
//! results in:
//!
//! ```text
//!         +-------+
//!         | START |
//!         +-------+
//!             |
//!             v
//!         +---------------+
//!         | maybe_raise() |
//!         +---------------+
//!             |
//!             v
//!         +-----------+
//!         | DISPATCH  |-------------+
//!         +-----------+             |
//! ValueError|        | TypeError    | else
//!   +---------+  +--------+         |
//!   | this()  |  | that() |         |
//!   +---------+  +--------+         |
//!           |        |              |
//!           v        v              |
//!         +-----------+             |
//!         | cleanup() |<------------+
//!         +-----------+
//!               |
//!               v
//!         +-----------+
//!         | RECOVERY  |
//!         +-----------+       
//! ```
//!
//! The recovery block is especially useful in the handling
//! of deferred jumps, which we now discuss.
//!
//! ## Jumps
//!
//! Upon reaching a jump statement, we must first examine
//! the stack of _try context_s. That is, we need to know whether we
//! are in certain clauses of a (nested) `try` statement,
//! in which case we must account for exceptions or defer
//! the jump until after executing a `finally` clause.
//!
//! More specifically, a `TryContext` consists of:
//! - one of the five possible subsets of `try`/`except`/`else`/`finally` that can occur
//! - the current state: in `try`, `except`, `else`, or `finally`
//! - a list of deferred jump statements
//!
//! To determine whether we should continue to defer jump statements,
//! we ask whether any of the `TryContext` members are in a `try` state,
//! or in an `except`/`else` state with an upcoming `finally` clause.
//! If so, we pop the last `TryContext`, append its deferred jumps to the
//! new last `TryContext`, and proceed to the next block.

//! Outside of these conditions, in the ordinary course of things,
//! we terminate the basic block
//! and add an outgoing edge. The target of the edge is:
//!
//! - the innermost loop guard, in the case of `continue`
//! - the innermost loop exit, in the case of `break`
//! - the terminal node, in the case of `return` or `raise`
//!
//! Finally, when we are in the `RECOVERY` block after a `finally` clause,
//! and we have checked that we do not need to further defer jumps,
//! we then resolve all deferred jumps and pop a `TryContext`.
//! Resolving the jumps just means that we add edges to the corresponding
//! targets out of the `RECOVERY` block.
