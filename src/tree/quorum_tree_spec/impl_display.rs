use std::fmt;

use super::QuorumTreeSpec;
use crate::Node;

impl<ID> fmt::Display for QuorumTreeSpec<ID>
where ID: Ord + fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            self.fmt_multiline(f, 0)
        } else {
            self.fmt_single_line(f)
        }
    }
}

impl<ID> QuorumTreeSpec<ID>
where ID: Ord + fmt::Display
{
    fn fmt_single_line(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/(", self.quorum_size)?;
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            Self::fmt_node_single_line(node, f)?;
        }
        write!(f, ")")
    }

    fn fmt_multiline(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        write!(f, "{}/(", self.quorum_size)?;

        for node in &self.nodes {
            writeln!(f)?;
            Self::write_indent(f, indent + 2)?;
            Self::fmt_node_multiline(node, f, indent + 2)?;
            write!(f, ",")?;
        }

        writeln!(f)?;
        Self::write_indent(f, indent)?;
        write!(f, ")")
    }

    fn fmt_node_single_line(node: &Node<ID>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match node {
            Node::Id(id) => write!(f, "{id}"),
            Node::Subtree(m) => m.spec.fmt_single_line(f),
        }
    }

    fn fmt_node_multiline(
        node: &Node<ID>,
        f: &mut fmt::Formatter<'_>,
        indent: usize,
    ) -> fmt::Result {
        match node {
            Node::Id(id) => write!(f, "{id}"),
            Node::Subtree(m) => m.spec.fmt_multiline(f, indent),
        }
    }

    fn write_indent(f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        for _ in 0..indent {
            write!(f, " ")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::QuorumTreeSpec;
    use crate::Node;
    use crate::QuorumTree;

    fn id(i: u64) -> Node<u64> {
        Node::Id(i)
    }

    fn set(quorum_size: u64, nodes: &[u64]) -> Node<u64> {
        Node::Subtree(QuorumTree::new(quorum_size, nodes.iter().copied().map(id)).unwrap())
    }

    #[test]
    fn test_display_single_line() {
        let qset = QuorumTreeSpec::new(2, [set(1, &[3, 4]), id(5), set(1, &[1, 2])]).unwrap();

        assert_eq!("2/(5,1/(1,2),1/(3,4))", qset.to_string());
    }

    #[test]
    fn test_display_multiline() {
        let qset = QuorumTreeSpec::new(2, [set(1, &[3, 4]), id(5), set(1, &[1, 2])]).unwrap();
        let expected = r#"2/(
  5,
  1/(
    1,
    2,
  ),
  1/(
    3,
    4,
  ),
)"#;

        assert_eq!(expected, format!("{:#}", qset));
    }
}
