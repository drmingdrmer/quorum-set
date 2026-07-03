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
    use crate::Node;

    fn id(i: u64) -> Node<u64> {
        Node::Id(i)
    }

    #[test]
    fn test_display_single_line() {
        let qset = QuorumTree::new(2, [id(3), id(1), id(2)]).unwrap();

        assert_eq!("2/(1,2,3)", qset.to_string());
    }

    #[test]
    fn test_display_multiline() {
        let qset = QuorumTree::new(2, [id(3), id(1), id(2)]).unwrap();
        let expected = r#"2/(
  1,
  2,
  3,
)"#;

        assert_eq!(expected, format!("{:#}", qset));
    }
}
