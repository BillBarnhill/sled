use super::*;

// TODO
// Merged
// LeftMerge(head: Raw, rhs: PageId, hi: Bound)
// ParentMerge(lhs: PageId, rhs: PageId)
// TxBegin(TxID), // in-mem
// TxCommit(TxID), // in-mem
// TxAbort(TxID), // in-mem

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum Frag {
    Commit(u64),
    Stage(Key, Version),
    Update(Key, Version),
    Base(Node),
    ChildSplit(ChildSplit),
    ParentSplit(ParentSplit),
    Counter(usize),
    Meta(Meta),
}

impl Frag {
    pub(super) fn unwrap_base(&self) -> &Node {
        if let Frag::Base(base, ..) = self {
            base
        } else {
            panic!("called unwrap_base_ptr on non-Base Frag!")
        }
    }

    pub(super) fn unwrap_meta(&self) -> &Meta {
        if let Frag::Meta(meta) = self {
            meta
        } else {
            panic!("called unwrap_base_ptr on non-Base Frag!")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ParentSplit {
    pub(crate) at: Vec<u8>,
    pub(crate) to: PageId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ChildSplit {
    pub(crate) at: Vec<u8>,
    pub(crate) to: PageId,
}
