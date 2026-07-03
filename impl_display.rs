use std::fmt;

use super::QuorumTree;

impl<ID> fmt::Display for QuorumTree<ID>
where ID: Ord + fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.spec, f)
    }
}

#[cfg(test)]
mod tests {
    use super::QuorumTree;
    use crate::QuorumNode;

    fn id(i: u64) -> QuorumNode<u64> {
        QuorumNode::Id(i)
    }

    #[test]
    fn test_display_single_line() {
        let qset = QuorumTree::new(2, [id(3), id(1), id(2)]);

        assert_eq!("2/(1,2,3)", qset.to_string());
    }

    #[test]
    fn test_display_multiline() {
        let qset = QuorumTree::new(2, [id(3), id(1), id(2)]);
        let expected = r#"2/(
  1,
  2,
  3,
)"#;

        assert_eq!(expected, format!("{:#}", qset));
    }
}
