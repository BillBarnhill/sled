use std::fmt::Debug;
use std::mem::size_of;

use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum Data {
    Index(Vec<(IVec, PageId)>),
    Leaf(Vec<(IVec, PageId)>),
}

impl Data {
    #[inline]
    pub(crate) fn size_in_bytes(&self) -> u64 {
        let self_sz = size_of::<Self>() as u64;

        let inner_sz = match self {
            Data::Index(ref v) => {
                let mut sz = 0_u64;
                for (k, _p) in v.iter() {
                    sz = sz
                        .saturating_add(k.len() as u64)
                        .saturating_add(size_of::<IVec>() as u64)
                        .saturating_add(size_of::<PageId>() as u64);
                }
                sz.saturating_add(
                    size_of::<Vec<(IVec, PageId)>>() as u64
                )
            }
            Data::Leaf(ref v) => {
                let mut sz = 0_u64;
                for (k, value) in v.iter() {
                    sz = sz
                        .saturating_add(k.len() as u64)
                        .saturating_add(size_of::<IVec>() as u64)
                        .saturating_add(size_of::<PageId>() as u64);
                }
                sz.saturating_add(
                    size_of::<Vec<(IVec, IVec)>>() as u64
                )
            }
        };

        self_sz.saturating_add(inner_sz)
    }

    pub(crate) fn len(&self) -> usize {
        match *self {
            Data::Index(ref ptrs) => ptrs.len(),
            Data::Leaf(ref items) => items.len(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn split(&self, lhs_prefix: &[u8]) -> (IVec, Data) {
        fn split_inner<T>(
            xs: &[(IVec, T)],
            lhs_prefix: &[u8],
        ) -> (IVec, Vec<(IVec, T)>)
        where
            T: Clone + Debug + Ord,
        {
            let mut decoded_xs: Vec<_> = xs
                .iter()
                .map(|&(ref k, ref v)| {
                    let decoded_k = prefix_decode(lhs_prefix, &*k);
                    (decoded_k, v.clone())
                })
                .collect();
            decoded_xs.sort();

            let (_lhs, rhs) =
                decoded_xs.split_at(decoded_xs.len() / 2 + 1);
            let split = rhs
                .first()
                .expect("rhs should contain at least one element")
                .0
                .clone();
            let rhs_data: Vec<_> = rhs
                .iter()
                .map(|&(ref k, ref v)| {
                    let new_k = prefix_encode(&*split, k);
                    (new_k, v.clone())
                })
                .collect();

            (split.into(), rhs_data)
        }

        match *self {
            Data::Index(ref ptrs) => {
                let (split, rhs) = split_inner(ptrs, lhs_prefix);
                (split, Data::Index(rhs))
            }
            Data::Leaf(ref items) => {
                let (split, rhs) = split_inner(items, lhs_prefix);
                (split, Data::Leaf(rhs))
            }
        }
    }

    pub(crate) fn drop_gte(&mut self, bound: &[u8], prefix: &[u8]) {
        match *self {
            Data::Index(ref mut ptrs) => {
                ptrs.retain(|&(ref k, _)| {
                    prefix_cmp_encoded(k, bound, prefix)
                        == std::cmp::Ordering::Less
                })
            }
            Data::Leaf(ref mut items) => {
                items.retain(|&(ref k, _)| {
                    prefix_cmp_encoded(k, bound, prefix)
                        == std::cmp::Ordering::Less
                })
            }
        }
    }

    pub(crate) fn leaf_ref(&self) -> Option<&Vec<(IVec, PageId)>> {
        match *self {
            Data::Index(_) => None,
            Data::Leaf(ref items) => Some(items),
        }
    }
}
