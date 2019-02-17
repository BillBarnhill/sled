use std::sync::Mutex;

use super::*;

#[derive(Debug)]
pub(crate) struct BLinkMaterializer {
    pub(super) recovery: Mutex<Recovery>,
    config: Config,
}

impl Materializer for BLinkMaterializer {
    type PageFrag = Frag;

    // a vector of (root, prev root, max counter) for deterministic recovery
    type Recovery = Recovery;

    fn new(
        config: Config,
        recovery: &Option<Self::Recovery>,
    ) -> Self {
        let recovery = recovery.clone().unwrap_or_default();

        BLinkMaterializer {
            recovery: Mutex::new(recovery),
            config,
        }
    }

    fn merge<'a, I>(&'a self, frags: I) -> Self::PageFrag
    where
        I: IntoIterator<Item = &'a Self::PageFrag>,
    {
        let mut frag_iter = frags.into_iter();

        let possible_base = frag_iter.next().expect(
            "merge should only be called on non-empty sets of Frag's",
        );

        match possible_base {
            Frag::Base(ref base_node_ref) => {
                let mut base_node = base_node_ref.clone();
                for frag in frag_iter {
                    base_node.apply(frag, self.config.merge_operator);
                }

                Frag::Base(base_node)
            }
            Frag::Counter(ref count) => {
                let mut max = *count;
                for frag in frag_iter {
                    if let Frag::Counter(count) = frag {
                        max = std::cmp::max(*count, max);
                    } else {
                        panic!(
                            "got non-BumpCounter in frag chain: {:?}",
                            frag
                        );
                    }
                }

                Frag::Counter(max)
            }
            Frag::Meta(_meta) => unimplemented!(
                "the Meta page should always be replaced, not linked"
            ),
            Frag::Versions(ref versions) => {
                let mut versions = versions.clone();
                for version in frag_iter {
                    versions.apply(version);
                }

                Frag::Versions(versions)
            }
            _ => panic!("non-Base in first element of frags slice"),
        }
    }

    fn recover(&self, frag: &Frag) -> Option<Recovery> {
        match *frag {
            Frag::Counter(count) => {
                let mut recovery = self.recovery.lock().expect(
                    "a thread panicked and poisoned the BLinkMaterializer's
                    roots mutex.",
                );
                recovery.counter =
                    std::cmp::max(count, recovery.counter);

                Some(recovery.clone())
            }
            _ => None,
        }
    }

    fn size_in_bytes(&self, frag: &Frag) -> usize {
        match *frag {
            Frag::Base(ref node) => std::mem::size_of::<Frag>()
                .saturating_add(node.size_in_bytes() as usize),
            _ => std::mem::size_of::<Frag>(),
        }
    }
}
