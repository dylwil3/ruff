//! Copied or heavily inspired by rustc data structures
use ruff_index::Idx;
use ruff_text_size::Ranged;
use std::fmt::{self, Display};

use super::{
    builder::{Condition, ControlEdge, ControlFlowGraph},
    implementations::{BlockId, CFG},
};

pub trait DirectedGraph<'a> {
    type Node: Idx;

    fn num_nodes(&self) -> usize;
}

pub trait StartNode<'a>: DirectedGraph<'a> {
    fn start_node(&self) -> Self::Node;
}

pub trait Successors<'a>: DirectedGraph<'a> {
    fn successors(&self, node: Self::Node) -> Vec<Self::Node>;
}

#[derive(Debug, Default)]
pub enum MermaidNodeShape {
    #[default]
    Rectangle,
    DoubleRectangle,
    RoundedRectangle,
    Stadium,
    Circle,
    DoubleCircle,
    Asymmetric,
    Rhombus,
    Hexagon,
    Parallelogram,
    Trapezoid,
}

impl MermaidNodeShape {
    fn open_close(&self) -> (&'static str, &'static str) {
        match self {
            Self::Rectangle => ("[", "]"),
            Self::DoubleRectangle => ("[[", "]]"),
            Self::RoundedRectangle => ("(", ")"),
            Self::Stadium => ("([", "])"),
            Self::Circle => ("((", "))"),
            Self::DoubleCircle => ("(((", ")))"),
            Self::Asymmetric => (">", "]"),
            Self::Rhombus => ("{", "}"),
            Self::Hexagon => ("{{", "}}"),
            Self::Parallelogram => ("[/", "/]"),
            Self::Trapezoid => ("[/", "\\]"),
        }
    }
}

pub struct MermaidNode {
    shape: MermaidNodeShape,
    content: String,
}

impl MermaidNode {
    pub fn with_content(content: String) -> Self {
        Self {
            shape: MermaidNodeShape::default(),
            content,
        }
    }

    fn mermaid_write_quoted_str(f: &mut fmt::Formatter<'_>, value: &str) -> fmt::Result {
        let mut parts = value.split('"');
        if let Some(v) = parts.next() {
            write!(f, "{v}")?;
        }
        for v in parts {
            write!(f, "#quot;{v}")?;
        }
        Ok(())
    }
}

impl Display for MermaidNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (open, close) = self.shape.open_close();
        write!(f, "{open}\"")?;
        if self.content.is_empty() {
            write!(f, "empty")?;
        } else {
            MermaidNode::mermaid_write_quoted_str(f, &self.content)?;
        }
        write!(f, "\"{close}")
    }
}

#[derive(Debug, Default)]
pub enum MermaidEdgeKind {
    #[default]
    Arrow,
    DottedArrow,
    ThickArrow,
    BidirectionalArrow,
}

impl Display for MermaidEdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MermaidEdgeKind::Arrow => write!(f, "-->"),
            MermaidEdgeKind::DottedArrow => write!(f, "-..->"),
            MermaidEdgeKind::ThickArrow => write!(f, "==>"),
            MermaidEdgeKind::BidirectionalArrow => write!(f, "<-->"),
        }
    }
}

#[derive(Debug, Default)]
pub struct MermaidEdge {
    kind: MermaidEdgeKind,
    content: String,
}

impl Display for MermaidEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.content.is_empty() {
            write!(f, "{}", self.kind)
        } else {
            write!(f, "{}|\"{}\"|", self.kind, self.content)
        }
    }
}

pub trait MermaidGraph<'a>: DirectedGraph<'a> + Successors<'a> {
    fn draw_node(&self, node: Self::Node) -> MermaidNode;
    fn draw_edges(&self, node: Self::Node) -> impl Iterator<Item = (Self::Node, MermaidEdge)> {
        self.successors(node)
            .into_iter()
            .map(|idx| (idx, MermaidEdge::default()))
    }
    fn draw_graph(&self) -> String {
        let mut graph = Vec::new();

        // Begin mermaid graph.
        graph.push("flowchart TD".to_string());

        // Draw nodes
        let num_nodes = self.num_nodes();
        for idx in 0..num_nodes {
            let node = Self::Node::new(idx);
            graph.push(format!("\tnode{}{}", idx, &self.draw_node(node)));
        }

        // Draw edges
        for idx in 0..num_nodes {
            graph.extend(
                self.draw_edges(Self::Node::new(idx))
                    .map(|(end_idx, edge)| format!("\tnode{}{}node{}", idx, edge, end_idx.index())),
            )
        }
        graph.join("\n")
    }
}

impl<'stmt, T: ControlFlowGraph<'stmt>> DirectedGraph<'stmt> for T
where
    T::Block: Idx,
{
    type Node = T::Block;

    fn num_nodes(&self) -> usize {
        self.num_blocks()
    }
}

impl<'stmt, T: ControlFlowGraph<'stmt>> StartNode<'stmt> for T
where
    T::Block: Idx,
{
    fn start_node(&self) -> Self::Node {
        self.initial()
    }
}

impl<'stmt, T: ControlFlowGraph<'stmt>> Successors<'stmt> for T
where
    T::Block: Idx,
{
    fn successors(&self, node: Self::Node) -> Vec<Self::Node> {
        self.out(node).targets().collect()
    }
}

pub(crate) struct CFGWithSource<'stmt> {
    cfg: CFG<'stmt>,
    source: &'stmt str,
}

impl<'stmt> CFGWithSource<'stmt> {
    pub(crate) fn new(cfg: CFG<'stmt>, source: &'stmt str) -> Self {
        Self { cfg, source }
    }
}

impl<'stmt> DirectedGraph<'stmt> for CFGWithSource<'stmt> {
    type Node = BlockId;

    fn num_nodes(&self) -> usize {
        self.cfg.num_nodes()
    }
}

impl<'stmt> StartNode<'stmt> for CFGWithSource<'stmt> {
    fn start_node(&self) -> Self::Node {
        self.cfg.start_node()
    }
}

impl<'stmt> Successors<'stmt> for CFGWithSource<'stmt> {
    fn successors(&self, node: Self::Node) -> Vec<Self::Node> {
        self.cfg.successors(node)
    }
}

impl<'stmt> MermaidGraph<'stmt> for CFGWithSource<'stmt> {
    fn draw_node(&self, node: Self::Node) -> MermaidNode {
        let statements: Vec<String> = self
            .cfg
            .stmts(node)
            .into_iter()
            .map(|stmt| self.source[stmt.range()].to_string())
            .collect();

        // Special case for terminal block
        if node == self.cfg.terminal() {
            if statements.is_empty() {
                return MermaidNode {
                    shape: MermaidNodeShape::DoubleCircle,
                    content: "EXIT".to_string(),
                };
            }
        }

        let content = if statements.is_empty() {
            "EMPTY".to_string()
        } else {
            statements.join("\n")
        };

        MermaidNode::with_content(content)
    }

    fn draw_edges(&self, node: Self::Node) -> impl Iterator<Item = (Self::Node, MermaidEdge)> {
        let edge_data = self.cfg.out(node);
        edge_data
            .targets()
            .zip(edge_data.conditions())
            .map(|(target, condition)| {
                let edge = match condition {
                    Condition::Test(expr) => MermaidEdge {
                        kind: MermaidEdgeKind::Arrow,
                        content: self.source[expr.range()].to_string(),
                    },
                    Condition::Always => {
                        if target == self.cfg.terminal() {
                            MermaidEdge {
                                kind: MermaidEdgeKind::ThickArrow,
                                content: String::new(),
                            }
                        } else {
                            MermaidEdge {
                                kind: MermaidEdgeKind::Arrow,
                                content: String::new(),
                            }
                        }
                    }
                    Condition::Match { subject, case } => {
                        let pattern = &self.source[case.pattern.range()];
                        let subject = &self.source[subject.range()];
                        MermaidEdge {
                            kind: MermaidEdgeKind::Arrow,
                            content: format!("{} matches {}", subject, pattern),
                        }
                    }
                    Condition::Iterator {
                        target,
                        iter,
                        is_async,
                    } => {
                        let target = &self.source[target.range()];
                        let iter = &self.source[iter.range()];
                        let prefix = if is_async { "async " } else { "" };
                        MermaidEdge {
                            kind: MermaidEdgeKind::Arrow,
                            content: format!("{}for {} in {}", prefix, target, iter),
                        }
                    }
                    Condition::ExceptHandler(handler) => {
                        let exc_types = match &handler.as_except_handler().unwrap().type_ {
                            Some(t) => self.source[t.range()].to_string(),
                            None => "any exception".to_string(),
                        };
                        MermaidEdge {
                            kind: MermaidEdgeKind::Arrow,
                            content: format!("except {}", exc_types),
                        }
                    }
                    Condition::Else => {
                        if target == self.cfg.terminal() {
                            MermaidEdge {
                                kind: MermaidEdgeKind::ThickArrow,
                                content: "Else".to_string(),
                            }
                        } else {
                            MermaidEdge {
                                kind: MermaidEdgeKind::Arrow,
                                content: "Else".to_string(),
                            }
                        }
                    }
                };
                (target, edge)
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl<'stmt> CFGWithSource<'stmt> {
    // Add debug method to print all edges
    pub fn debug_edges(&self) {
        println!("Debug: Listing all edges in CFG");
        for block_idx in 0..self.cfg.num_blocks() {
            let block = BlockId::new(block_idx);
            let edge = self.cfg.out(block);
            println!(
                "Block {}: targets={:?}, conditions={:?}",
                block_idx,
                edge.targets().collect::<Vec<_>>(),
                edge.conditions().collect::<Vec<_>>()
            );
        }
    }
}
