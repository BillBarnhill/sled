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
    Set(IVec, IVec),
    Del(IVec),
    Merge(IVec, IVec),
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

    pub(super) fn set_key(&mut self, key: IVec) {
        match self {
            Frag::Set(_, v) => *self = Frag::Set(key, *v),
            Frag::Del(_) => *self = Frag::Del(key),
            Frag::Merge(_, v) => *self = Frag::Merge(key, *v),
            other => panic!("set_key called on {:?}", other),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ParentSplit {
    pub(crate) at: IVec,
    pub(crate) to: PageId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ChildSplit {
    pub(crate) at: IVec,
    pub(crate) to: PageId,
}
