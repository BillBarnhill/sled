use std::borrow::{BorrowMut, Cow};

use super::*;

pub(crate) struct View<'a> {
    frags: &'a [&'a Frag],
    cached_base: Option<usize>,
    pub(crate) cached_data: Option<Cow<'a, Data>>,
}

impl<'a> View<'a> {
    pub(crate) fn new(frags: &'a [&'a Frag]) -> View<'a> {
        View {
            frags: frags,
            cached_base: None,
            cached_data: None,
        }
    }

    pub(crate) fn is_free(&self) -> bool {
        self.frags.is_empty()
    }

    pub(crate) fn lo(&mut self) -> &'a [u8] {
        self.base().lo.as_ref()
    }

    pub(crate) fn hi(&mut self) -> &'a [u8] {
        for frag in &self.frags[..self.base_offset()] {
            if let Frag::ChildSplit(cs) = frag {
                return cs.at.as_ref();
            }
        }

        self.base().hi.as_ref()
    }

    pub(crate) fn next(&mut self) -> Option<PageId> {
        for frag in &self.frags[..self.base_offset()] {
            if let Frag::ChildSplit(cs) = frag {
                return Some(cs.to);
            }
        }

        self.base().next
    }

    fn base(&mut self) -> &'a Node {
        let frag = &self.frags[self.base_offset()];
        if let Frag::Base(node) = frag {
            node
        } else {
            unimplemented!()
        }
    }

    pub(crate) fn data(&mut self) -> &'a Data {
        if let Some(data) = self.cached_data {
            return &data;
        }

        let mut datacow = Cow::Borrowed(&self.base().data);

        for offset in (0..self.base_offset()).rev() {
            // iterate backwards from the base, applying data changes
            let frag = &self.frags[offset];

            match frag {
                Frag::Set(k, v) => {
                    if let Data::Leaf(ref mut records) = **datacow.borrow_mut()
                    {
                        let search = records
                            .binary_search_by(|(k, _)| prefix_cmp(k, &k));
                        match search {
                            Ok(idx) => records[idx] = (*k, *v),
                            Err(idx) => records.insert(idx, (*k, *v)),
                        }
                    } else {
                        panic!("tried to Set a value to an index");
                    }
                }
                Frag::Del(k) => {
                    if let Data::Leaf(ref mut records) = **datacow.borrow_mut()
                    {
                        let search =
                            records.binary_search_by(|&(ref k, ref _v)| {
                                prefix_cmp(k, &*k)
                            });
                        if let Ok(idx) = search {
                            records.remove(idx);
                        }
                    } else {
                        unimplemented!()
                    }
                }
                Frag::Merge(k, v) => unimplemented!(),
                Frag::ChildSplit(cs) => {
                    datacow.borrow_mut().drop_gte(&cs.at, self.lo());
                }
                Frag::ParentSplit(ps) => {
                    if let Data::Index(ref mut ptrs) = **datacow.borrow_mut() {
                        let encoded_sep = prefix_encode(self.lo(), &ps.at);
                        match ptrs.binary_search_by(|a| {
                            prefix_cmp(&a.0, &encoded_sep)
                        }) {
                            Ok(_) => panic!("must not have found ptr"),
                            Err(idx) => ptrs.insert(idx, (encoded_sep, ps.to)),
                        }
                    } else {
                        panic!("tried to attach a ParentSplit to a Leaf chain");
                    }
                }
                Frag::Base(Node) => unimplemented!(),
            }
        }

        self.cached_data = Some(datacow);

        &self.cached_data.unwrap()
    }

    fn base_offset(&mut self) -> usize {
        if let Some(cached) = self.cached_base {
            cached
        } else {
            let offset = self
                .frags
                .iter()
                .position(|f| if let Frag::Base(_) = f { true } else { false })
                .unwrap();

            self.cached_base = Some(offset);

            offset
        }
    }

    pub(crate) fn should_split(&self, max_sz: u64) -> bool {
        self.data().len() > 2 && self.size_in_bytes() > max_sz
    }

    pub(crate) fn split(&self) -> Node {
        let (split, right_data) = self.data().split(self.lo());
        Node {
            data: right_data,
            next: self.next(),
            lo: split,
            hi: self.hi().into(),
        }
    }

    #[inline]
    pub(crate) fn size_in_bytes(&self) -> u64 {
        let self_sz = std::mem::size_of::<Self>() as u64;
        let lo_sz = self.lo().len();
        let hi_sz = self.hi().len();
        let data_sz = self.data().size_in_bytes();

        self_sz
            .saturating_add(lo_sz as u64)
            .saturating_add(hi_sz as u64)
            .saturating_add(data_sz)
    }
}
