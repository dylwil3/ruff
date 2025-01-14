//! # Overview
//! To build a control-flow graph, we step through each statement in order.
//! By default, statements are added to the current basic block until we
//! reach a statement that invokes control flow. These are exactly the following:
//!
//! | Branching                  | Loops   | Jumps      |
//! |----------------------------|---------|------------|
//! | `if`                       | `for`   | `break`    |
//! | `match`                    | `while` | `continue` |
//! | `try`                      |         | `raise`    |
//! | `with`                     |         | `return`   |
//!
//! There are also `assert` statements, which are equivalent
//! to a branch followed by a jump. That is,
//! ```
//! assert cond
//! ```
//!
//! is equivalent to
//!
//! ```
//! if not cond:
//!   raise AssertionError
//! ```
//!
//! (Technically there is an additional `if __debug__` wrapped around, but
//! we will ignore that.)
//!
//! The control flow graph is then constructed using this equivalent
//! form.
//!
//!
//! We now discuss how each kind of control flow is handled.
//!
//! ## Branching
//!
//! Upon reaching a branching statement, the statement
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
//!           +-------+
//!   +------>|if stmt|
//!   |       +-------+
//!   |          |
//!   | cond     | not cond
//!   |          |
//!   v          v
//! +----+     +----+
//! | f()|     | g()|
//! +----+     +----+
//!   |          |
//!   +-----+----+
//!         |
//!       +--------+
//!       |  End   |
//!       +--------+
//!```
//!
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
//! `continue` or `raise`.
//!
//! For example,
//!
//! ```python
//! while cond:
//!   continue
//! ```
//! would create
//!
//!```text
//!          ┌─────────┐  
//!          │         │  
//!          │  Start  │  
//!          │         │  
//!          └────┬────┘  
//!               │       
//!          ┌────▼────┐  
//!          │         │  
//!     ┌───►│  cond?  │  
//!     │    │         │  
//!     │    └─────┬──┬┘  
//!     │          │  │   
//!     │          │  │   
//! ┌───┴───────┐  │  │   
//! │           │  │  │   
//! │ continue  │◄─┘  │   
//! │           │     │   
//! └───────────┘     │   
//!                   │   
//!                   │   
//!           ┌───────▼──┐
//!           │          │
//!           │Loop Exit │
//!           │          │
//!           └──────────┘
//! ```
//!
//! ## Jumps
//!
//! Upon reaching a jump statement, we terminate the basic block
//! and add an outgoing edge. The target of the outgoing edge is
//! determined by the _loop context_ and the _try context_. That is,
//! we need to know whether and where we are within a try-statement,
//! and what (if any) is the innermost loop surrounding the current
//! statement.
//!
//! ### Break and Continue
//!
//! When we encounter a `break` or `continue`, we first check the
//! _try context_ to see if we are in a try context that has a
//! `finally` clause. In this case, we resolve the `finally` clause
//! first. Assuming no further jumps have occurred, we then check the
//! current loop context and direct flow to the loop exit (resp. guard)
//! in the case of a `break` (resp. `continue`).
//!
//! ### Raise and Return
//!
//! When we encounter a `raise` or `return`, we first check the
//! _try context_ stack to see if we are in a try context, and
//! whether it has an `except` or `finally` clause. In the case of
//! a `return`, we first resolve the `finally` block before returning.
//! In the case of a `raise`, we visit `except` blocks and then the
//! `finally` block. Without type inference, we are limited in our
//! ability to determine which `except` block is visited. As a first
//! pass, we pretend that we visit each of them.
//!
//! # Implementation Details
//!
//! Our implementation is non-recursive, guaranteed to terminate,
//! and takes a single, forward pass through a list of statements.
//!
//! To achieve this, our builder maintains two pieces of information:
//! - A stack of loop contexts
//! - A stack of `try` contexts
//!
//! # Misc other things
//! - When we encounter raise/return we should make remaining
//! things in local block an orphan node of some kind and mark
//! unreachable? Or add to worklist for later or something?
//! I guess there could be some context about being unreachable
//! or something...? Or maybe no special casing is required? Unclear.
pub mod builder;
pub mod implementations;
pub mod visualize;
