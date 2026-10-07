//! The in-memory note tree, its stable ids, and its Merkle hashes.
//!
//! # The hash definition is a wire format
//!
//! These digests are the whole point of the tree: a server can compare one root
//! hash instead of diffing documents. That only works if both sides compute the
//! same bytes, so the definition below is a contract, not an implementation
//! detail. Changing it invalidates every stored `.listmeta` and every
//! comparison a peer has already made. It is `sicompass_sync::merkle`'s, which
//! the board plugin and the sync server share; this module only applies it.
//!
//! ```text
//! hash(leaf)   = sha256( b"s\0" || text )
//! hash(branch) = sha256( b"o\0" || key || b"\0" || d0 || d1 || ... )
//! root         = sha256( b"r\0" || d0 || d1 || ... )
//! ```
//!
//! where `d0..dn` are the children's **raw 32-byte digests** in list order, not
//! their hex forms.
//!
//! The `s` / `o` / `r` prefixes are domain separation. Without them a leaf
//! whose text is `x` and a childless branch whose key is `x` would hash
//! identically, and a tree could be restructured without changing its root.
//!
//! Two things are deliberately **not** hashed:
//!
//! * **Ids**, because they are local bookkeeping. Two machines that built the
//!   same tree independently must agree on its root hash.
//! * **Visibility**, because marking a note public is a change of audience, not
//!   of content. Hashing it would make a peer see a flipped switch as a rewrite
//!   and re-send the whole note.

use sicompass_sync::merkle;

/// A node's local identity. Minted once, never reused, never hashed.
///
/// Position cannot serve as identity: inserting a row above a note renumbers
/// it, and anything holding a position (a navigation path, a pending rename)
/// would silently start pointing at a different note.
pub type NodeId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}

impl Visibility {
    /// The stored, language-neutral form. Never localized: this string is
    /// written to disk and read by a server.
    pub fn as_str(self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Public => "public",
        }
    }

    /// Unknown values read back as `Private`. A note whose visibility we cannot
    /// understand must not be treated as published.
    pub fn parse(s: &str) -> Self {
        match s {
            "public" => Visibility::Public,
            _ => Visibility::Private,
        }
    }
}

/// One node: a line of text, plus children if it has any.
///
/// A leaf and a childless branch are different things — a branch is somewhere
/// the user can descend into and add to, so `is_branch` is stored rather than
/// derived from `children.is_empty()`.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub text: String,
    pub is_branch: bool,
    pub children: Vec<Node>,
    /// Set only on a top-level note, which is the unit of sharing. Deeper nodes
    /// inherit their note's visibility and store `None`.
    pub visibility: Option<Visibility>,
}

impl Node {
    pub fn leaf(id: NodeId, text: impl Into<String>) -> Self {
        Node {
            id,
            text: text.into(),
            is_branch: false,
            children: Vec::new(),
            visibility: None,
        }
    }

    pub fn branch(id: NodeId, text: impl Into<String>) -> Self {
        Node {
            id,
            text: text.into(),
            is_branch: true,
            children: Vec::new(),
            visibility: None,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        if self.is_branch {
            let children: Vec<[u8; 32]> = self.children.iter().map(Node::hash).collect();
            merkle::branch_hash(&self.text, &children)
        } else {
            merkle::leaf_hash(&self.text)
        }
    }

    pub fn hash_hex(&self) -> String {
        hex(&self.hash())
    }

    /// Depth-first search for a node by id, returning a mutable borrow.
    pub fn find_mut(nodes: &mut [Node], id: NodeId) -> Option<&mut Node> {
        for n in nodes.iter_mut() {
            if n.id == id {
                return Some(n);
            }
            if let Some(found) = Node::find_mut(&mut n.children, id) {
                return Some(found);
            }
        }
        None
    }

    pub fn find(nodes: &[Node], id: NodeId) -> Option<&Node> {
        for n in nodes {
            if n.id == id {
                return Some(n);
            }
            if let Some(found) = Node::find(&n.children, id) {
                return Some(found);
            }
        }
        None
    }

    /// The chain of ids from the root down to the list that *contains* `id`.
    ///
    /// `Some(vec![])` means the node is a top-level note. `None` means no node
    /// with that id exists.
    pub fn path_to_parent_of(nodes: &[Node], id: NodeId) -> Option<Vec<NodeId>> {
        for n in nodes {
            if n.id == id {
                return Some(Vec::new());
            }
            if let Some(mut rest) = Node::path_to_parent_of(&n.children, id) {
                let mut path = vec![n.id];
                path.append(&mut rest);
                return Some(path);
            }
        }
        None
    }

    pub fn max_id(nodes: &[Node]) -> NodeId {
        nodes
            .iter()
            .map(|n| n.id.max(Node::max_id(&n.children)))
            .max()
            .unwrap_or(0)
    }
}

/// The whole tree: the top-level notes, plus the id counter.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tree {
    pub notes: Vec<Node>,
    next_id: NodeId,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            notes: Vec::new(),
            next_id: 1,
        }
    }

    /// Rebuild the counter from what is actually in the tree. Called after a
    /// load, so an id read off disk can never be handed out a second time.
    pub fn reseat_counter(&mut self) {
        self.next_id = Node::max_id(&self.notes) + 1;
    }

    /// The next id [`Tree::mint_id`] would hand out.
    pub fn next_id(&self) -> NodeId {
        self.next_id
    }

    /// Never hand out an id below `floor`. A reload (after a sync merged
    /// another machine's notes in) rebuilds the counter from what is on disk,
    /// which can be lower than ids the undo timeline still holds for deleted
    /// notes; reusing one would let an undo bring a note back over another.
    pub fn raise_counter(&mut self, floor: NodeId) {
        self.next_id = self.next_id.max(floor);
    }

    pub fn mint_id(&mut self) -> NodeId {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        id
    }

    /// The root hash: the tree's identity, and what a peer compares first.
    pub fn root_hash(&self) -> [u8; 32] {
        let notes: Vec<[u8; 32]> = self.notes.iter().map(Node::hash).collect();
        merkle::root_hash(&notes)
    }

    pub fn root_hash_hex(&self) -> String {
        hex(&self.root_hash())
    }

    /// The list at `path` (a chain of node ids from the root), or the top-level
    /// notes for an empty path. `None` when any id along the way is gone.
    pub fn list_at(&self, path: &[NodeId]) -> Option<&Vec<Node>> {
        let mut cur = &self.notes;
        for id in path {
            let node = cur.iter().find(|n| n.id == *id)?;
            cur = &node.children;
        }
        Some(cur)
    }

    pub fn list_at_mut(&mut self, path: &[NodeId]) -> Option<&mut Vec<Node>> {
        let mut cur = &mut self.notes;
        for id in path {
            let idx = cur.iter().position(|n| n.id == *id)?;
            cur = &mut cur[idx].children;
        }
        Some(cur)
    }
}

pub use merkle::hex;

#[cfg(test)]
mod tests {
    use super::*;

    /// The hash is a wire format, so it is pinned to bytes computed
    /// independently (Python's hashlib over the formula in the module doc).
    /// The same vector is asserted in `sicompass-sync`'s `merkle` tests.
    #[test]
    fn the_root_hash_matches_the_published_vector() {
        let mut groceries = Node::branch(1, "Groceries");
        let mut weekend = Node::branch(3, "Weekend");
        weekend.children.push(Node::leaf(4, "bread"));
        groceries.children.push(Node::leaf(2, "milk"));
        groceries.children.push(weekend);
        let tree = Tree {
            notes: vec![groceries, Node::leaf(5, "Ideas")],
            next_id: 6,
        };
        assert_eq!(
            tree.notes[0].hash_hex(),
            "7f659c2de7c765c2d0ef2d8f16aa1ae315c7f071e74bcfa6419fdcbb1a99a25d"
        );
        assert_eq!(
            tree.root_hash_hex(),
            "02b439fedb3ec6dd9b403bfa49e40bf5eb3fdcacd8bf3f501eca3278d98f79ea"
        );
        assert_eq!(
            Tree::new().root_hash_hex(),
            "96229c0a1dcb79d7d50913f882e3144961b5616140ded9ab844bd685e08e3a30"
        );
    }

    #[test]
    fn the_counter_can_be_raised_but_never_lowered() {
        let mut t = Tree::new();
        t.raise_counter(40);
        assert_eq!(t.mint_id(), 40);
        t.raise_counter(3);
        assert_eq!(t.mint_id(), 41);
    }
}
