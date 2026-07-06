use std::fmt;
use std::fmt::Write as _;

use sha2::Digest;
use sha2::Sha256;

use super::QuorumTreeSpec;
use crate::CanonicalId;
use crate::tree::canonical_id::MAX_CANONICAL_ID_LEN;

impl<ID> CanonicalId for QuorumTreeSpec<ID>
where ID: Ord + CanonicalId
{
    fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
    where W: fmt::Write + ?Sized {
        write!(f, "{}/", self.quorum_size)?;

        let mut s = String::new();
        write!(&mut s, "(")?;

        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 {
                write!(&mut s, ",")?;
            }
            node.fmt_canonical_id(&mut s)?;
        }
        write!(&mut s, ")")?;

        if s.len() > MAX_CANONICAL_ID_LEN {
            write!(
                f,
                "Hash#{}:{:x}",
                self.nodes.len(),
                Sha256::digest(s.as_bytes())
            )?;
        } else {
            write!(f, "{}", s)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::QuorumTreeSpec;
    use crate::CanonicalId;
    use crate::Node;
    use crate::QuorumTree;
    use crate::tree::canonical_id::MAX_CANONICAL_ID_LEN;

    fn id(i: u64) -> Node<u64> {
        Node::Id(i)
    }

    fn str_id(s: &str) -> Node<String> {
        Node::Id(s.to_string())
    }

    fn set(quorum_size: u64, nodes: &[u64]) -> Node<u64> {
        Node::Subtree(QuorumTree::new(quorum_size, nodes.iter().copied().map(id)).unwrap())
    }

    #[test]
    fn test_canonical_id_generation() {
        let qset = QuorumTreeSpec::new(2, [id(3), id(1), id(2)]).unwrap();

        assert_eq!("2/(Id=1,Id=2,Id=3)", qset.canonical_id());
    }

    #[test]
    fn test_canonical_id_generation_for_tree() {
        let qset = QuorumTreeSpec::new(2, [set(1, &[3, 4]), id(5), set(1, &[1, 2])]).unwrap();

        assert_eq!(
            "2/(Id=5,Subtree=1/(Id=1,Id=2),Subtree=1/(Id=3,Id=4))",
            qset.canonical_id()
        );
    }

    #[test]
    fn test_canonical_id_is_deterministic_for_node_order() {
        let qset1 = QuorumTreeSpec::new(2, [set(1, &[3, 4]), id(5), set(1, &[1, 2])]).unwrap();
        let qset2 = QuorumTreeSpec::new(2, [id(5), set(1, &[1, 2]), set(1, &[3, 4])]).unwrap();
        let qset3 = QuorumTreeSpec::new(2, [set(1, &[2, 1]), set(1, &[4, 3]), id(5)]).unwrap();

        assert_eq!(
            "2/(Id=5,Subtree=1/(Id=1,Id=2),Subtree=1/(Id=3,Id=4))",
            qset1.canonical_id()
        );
        assert_eq!(qset1.canonical_id(), qset2.canonical_id());
        assert_eq!(qset1.canonical_id(), qset3.canonical_id());
    }

    #[test]
    fn test_canonical_id_hashes_large_tree() {
        let nested = QuorumTree::new(2, [set(1, &[9, 10]), id(11), set(1, &[7, 8])]).unwrap();

        assert_eq!(
            "2/(Id=11,Subtree=1/(Id=7,Id=8),Subtree=1/(Id=9,Id=10))",
            nested.canonical_id()
        );

        let qset = QuorumTreeSpec::new(3, [
            set(2, &[4, 5, 6]),
            Node::Subtree(nested),
            id(12),
            set(2, &[1, 2, 3]),
        ])
        .unwrap();
        let canonical_body = "(Id=12,Subtree=2/(Id=1,Id=2,Id=3),Subtree=2/(Id=11,Subtree=1/(Id=7,Id=8),Subtree=1/(Id=9,Id=10)),Subtree=2/(Id=4,Id=5,Id=6))";

        assert!(canonical_body.len() > MAX_CANONICAL_ID_LEN);
        assert_eq!(
            "3/Hash#4:f7bbda30597f2548c831f13d8c9b295ef538373ed7f437414836e41b05be1c0a",
            qset.canonical_id()
        );
    }

    #[test]
    fn test_canonical_id_escapes_node_id() {
        let qset = QuorumTreeSpec::new(1, [str_id("a(b"), str_id("c,d"), str_id("e]f")]).unwrap();

        assert_eq!("1/(Id=a%28b,Id=c%2Cd,Id=e%5Df)", qset.canonical_id());
    }

    #[test]
    fn test_canonical_id_separates_nodes() {
        let qset1 = QuorumTreeSpec::new(1, [str_id("ab"), str_id("c")]).unwrap();
        let qset2 = QuorumTreeSpec::new(1, [str_id("a"), str_id("bc")]).unwrap();

        assert_eq!("1/(Id=ab,Id=c)", qset1.canonical_id());
        assert_eq!("1/(Id=a,Id=bc)", qset2.canonical_id());
        assert_ne!(qset1.canonical_id(), qset2.canonical_id());
    }

    #[test]
    fn test_canonical_id_escapes_nested_set() {
        let sub = QuorumTree::new(1, [str_id("a,b")]).unwrap();
        let qset = QuorumTreeSpec::new(1, [Node::Subtree(sub)]).unwrap();

        assert_eq!("1/(Subtree=1/(Id=a%2Cb))", qset.canonical_id());
    }

    #[test]
    fn test_canonical_id_keeps_64_char_user_id() {
        let raw_id = "a".repeat(64);
        let node = str_id(&raw_id);

        assert_eq!(format!("Id={}", raw_id), node.canonical_id());

        let qset = QuorumTreeSpec::new(1, [node]).unwrap();

        assert_eq!(
            "1/Hash#1:5b73ff9467e04134c5408ff58a9151f89d59b9e0d42d2b274901e38dac22b8cf",
            qset.canonical_id()
        );
    }

    #[test]
    fn test_canonical_id_hashes_long_user_id() {
        let raw_id = "a".repeat(65);
        let node = str_id(&raw_id);

        assert_eq!(
            "Id=Hash#1:635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0",
            node.canonical_id()
        );

        let qset = QuorumTreeSpec::new(1, [node, str_id("b")]).unwrap();

        assert_eq!(
            "1/Hash#2:d3cd62fe20b51a819da9b88e8bdae8e52de56922371181a53a34d80927c85509",
            qset.canonical_id()
        );
    }
}
