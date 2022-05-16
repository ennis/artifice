use crate::{
    model,
    model::{attribute::AttributeAny, file::DocumentDatabase, metadata, EditAction, Node, Path, ShareGroup},
};
use core::fmt;
use imbl::{HashMap, Vector};
use kyute_common::{Atom, Data};
use std::{
    fmt::{Formatter, Write},
    sync::Arc,
};

/// The root object of artifice documents.
#[derive(Clone)]
pub struct Document {
    /// Document revision index
    revision: usize,
    /// Root node
    pub(crate) root: Node,
    //nodes: HashMap<Path, Node>,
    // Share groups
    //pub share_groups: Vector<ShareGroup>,
}

impl Data for Document {
    fn same(&self, other: &Self) -> bool {
        self.revision.same(&other.revision) && self.root.same(&other.root)
    }
}

impl Document {
    /// Returns a new document.
    pub fn new() -> Document {
        Document {
            revision: 0,
            //nodes: Default::default(),
            root: Node::new(0, Path::root()),
        }
    }

    /// Returns the node with the given path.
    pub fn node(&self, path: &Path) -> Option<&Node> {
        if path.is_root() {
            Some(&self.root)
        } else {
            self.node(&path.parent().unwrap())?.children.get(&path.name())
        }
    }

    /// Returns a mutable reference to the node with the given path.
    pub fn node_mut(&mut self, path: &Path) -> Option<&mut Node> {
        if path.is_root() {
            Some(&mut self.root)
        } else {
            self.node_mut(&path.parent().unwrap())?.children.get_mut(&path.name())
        }
    }

    /// Returns a reference to the root node.
    pub fn root(&self) -> &Node {
        &self.root
    }

    /// Returns the attribute at the given path.
    pub fn attribute(&self, path: &Path) -> Option<&AttributeAny> {
        self.node(&path.parent()?)?.attribute(&path.name())
    }

    /// Prints a textual representation of this document.
    pub fn dump(&self, out: &mut dyn std::fmt::Write) {
        let mut printer = DocumentPrettyPrinter::new(out);
        printer.print_node(&self.root, true);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Dump
////////////////////////////////////////////////////////////////////////////////////////////////////

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        self.dump(f);
        Ok(())
    }
}

struct DocumentPrettyPrinter<'a> {
    output: &'a mut dyn std::fmt::Write,
    indent: usize,
    lines: Vec<usize>,
}

impl<'a> DocumentPrettyPrinter<'a> {
    fn new(output: &'a mut dyn std::fmt::Write) -> DocumentPrettyPrinter<'a> {
        DocumentPrettyPrinter {
            output,
            indent: 0,
            lines: vec![],
        }
    }

    fn print_line_prefix(&mut self) {
        let mut pad = vec![' '; self.indent];
        for &p in self.lines.iter() {
            pad[p] = '│';
        }
        for c in pad {
            self.output.write_char(c);
        }
    }

    fn print_node(&mut self, node: &Node, is_last: bool) {
        self.print_line_prefix();
        self.output.write_char(if is_last { '└' } else { '├' });
        write!(self.output, "{}", node.path.name().as_ref());
        if let Some(op) = node.metadata(metadata::OPERATOR) {
            write!(self.output, " <{}>", op.as_ref());
        }
        writeln!(self.output, " ({:?})", node.path);

        let child_item_count = node.children.len() + node.attributes.len();
        let mut child_index = 0;

        if !is_last {
            self.lines.push(self.indent);
        }
        self.indent += 2;

        for attr in node.attributes.values() {
            child_index += 1;
            if child_index == child_item_count {
                self.print_attribute(attr, true);
            } else {
                self.print_attribute(attr, false);
            }
        }

        for n in node.children.values() {
            child_index += 1;
            if child_index == child_item_count {
                self.print_node(n, true);
            } else {
                self.print_node(n, false);
            }
        }

        self.indent -= 2;
        if !is_last {
            self.lines.pop();
        }
    }

    fn print_attribute(&mut self, attr: &AttributeAny, is_last: bool) {
        self.print_line_prefix();
        self.output.write_char(if is_last { '└' } else { '├' });
        write!(self.output, "{} [{}]", attr.path.name().as_ref(), attr.ty.as_ref());
        if let Some(ref val) = attr.value {
            write!(self.output, " = {:?}", val);
        }
        if let Some(ref cx) = attr.connection {
            write!(self.output, " ⇒ {:?}", cx);
        }
        writeln!(self.output);
    }
}
