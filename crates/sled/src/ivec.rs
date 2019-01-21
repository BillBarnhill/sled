use super::*;

use std::{
    alloc::{alloc, dealloc, Layout},
    fmt,
    ops::Deref,
};

const INLINE_LEN_MASK: u8 = 0b0000_1111;
const KIND_MASK: u8 = 0b1111_0000;
const KIND_INLINE: u8 = 0b1000_0000;
const KIND_REMOTE: u8 = 0b0100_0000;

#[cfg(target_pointer_width = "64")]
const CUTOFF: usize = 15;
#[cfg(target_pointer_width = "64")]
type Inner = [u8; 16];

#[cfg(target_pointer_width = "32")]
const CUTOFF: usize = 7;
#[cfg(target_pointer_width = "32")]
type Inner = [u8; 8];

#[derive(Default, Clone, Copy, Ord, Eq, Serialize, Deserialize)]
pub(crate) struct IVec {
    #[serde(with = "ser")]
    data: Inner,
}

impl IVec {
    pub(crate) fn new(v: &[u8]) -> IVec {
        if v.len() <= CUTOFF {
            let sz = v.len() as u8;
            let tag = KIND_INLINE | sz;

            let mut data: Inner = [0; std::mem::size_of::<Inner>()];
            data[CUTOFF] = tag;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    v.as_ptr(),
                    data.as_mut_ptr(),
                    v.len(),
                );
            }

            IVec { data }
        } else {
            let slice = unsafe {
                let layout = Layout::from_size_align_unchecked(
                    v.len(),
                    std::mem::size_of::<u8>(),
                );
                let dst = alloc(layout);
                assert_ne!(dst, std::ptr::null_mut());
                std::ptr::copy_nonoverlapping(
                    v.as_ptr(),
                    dst,
                    v.len(),
                );

                std::slice::from_raw_parts(dst, v.len())
            };

            let mut data: Inner =
                unsafe { std::mem::transmute(slice) };

            assert_eq!(
                data[CUTOFF], 0,
                "we incorrectly assumed that we could use \
                 the highest bits in the length field. please \
                 report this bug ASAP!"
            );

            data[CUTOFF] = KIND_REMOTE;

            IVec { data }
        }
    }

    #[inline]
    pub(crate) fn size_in_bytes(&self) -> u64 {
        if self.data[CUTOFF] == KIND_INLINE {
            std::mem::size_of::<IVec>() as u64
        } else {
            let sz = std::mem::size_of::<IVec>() as u64;
            sz.saturating_add(self.len() as u64)
        }
    }

    pub(crate) fn deallocate(self, guard: &Guard) {
        if self.data[CUTOFF] == KIND_REMOTE {
                let mut data = self.data.clone();
                data[CUTOFF] = 0;

                unsafe {
                    let slice: &mut [u8] = std::mem::transmute(data);
                    let len = slice.len();
                    let layout = Layout::from_size_align_unchecked(
                        len,
                        std::mem::size_of::<u8>(),
                    );

                    guard.defer(move || dealloc(slice.as_mut_ptr(), layout));
                }
        }
    }
}

impl From<Vec<u8>> for IVec {
    fn from(v: Vec<u8>) -> IVec {
        if v.len() <= CUTOFF {
            IVec::new(&v)
        } else {
            let bs = v.into_boxed_slice();
            let slice: &[u8] = bs.deref();
            let mut data: Inner = unsafe {
                std::mem::transmute(slice)
            };
            std::mem::forget(bs);
            assert_eq!(data[CUTOFF], 0);
            data[CUTOFF] = KIND_REMOTE;
            IVec {
                data
            }
        }
    }
}

impl Deref for IVec {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        let tag = self.data[CUTOFF];
        let kind = tag & KIND_MASK;
        if kind == KIND_INLINE {
            let base = self.data.as_ptr();
            let len = (tag & INLINE_LEN_MASK) as usize;
            unsafe { std::slice::from_raw_parts(base, len) }
        } else {
            let mut data: Inner = self.data;
            data[CUTOFF] = 0;

            unsafe {
                let ptr: *mut [u8] = std::mem::transmute(data);
                &*ptr
            }
        }
    }
}

impl PartialOrd for IVec {
    fn partial_cmp(
        &self,
        other: &IVec,
    ) -> Option<std::cmp::Ordering> {
        Some(self.deref().cmp(other.deref()))
    }
}

impl PartialEq for IVec {
    fn eq(&self, other: &IVec) -> bool {
        self.deref() == other.deref()
    }
}

impl<'a, T: AsRef<[u8]>> PartialEq<T> for IVec {
    fn eq(&self, other: &T) -> bool {
        self.deref() == other.as_ref()
    }
}

impl PartialEq<[u8]> for IVec {
    fn eq(&self, other: &[u8]) -> bool {
        self.deref() == other
    }
}

impl fmt::Debug for IVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IVec( {:?} )", self.deref())
    }
}

pub(crate) mod ser {
    use super::{IVec, Inner};

    use std::ops::Deref;

    use serde::de::{Deserializer, Visitor};
    use serde::ser::Serializer;

    pub(crate) fn serialize<S>(
        data: &Inner,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let iv = IVec { data: *data };
        let ivr: &[u8] = iv.deref();

        serializer.serialize_bytes(ivr)
    }

    struct IVecVisitor;

    impl<'de> Visitor<'de> for IVecVisitor {
        type Value = IVec;

        fn expecting(
            &self,
            formatter: &mut std::fmt::Formatter,
        ) -> std::fmt::Result {
            formatter.write_str("a borrowed byte array")
        }

        #[inline]
        fn visit_borrowed_bytes<E>(
            self,
            v: &'de [u8],
        ) -> Result<IVec, E>
        {
            Ok(IVec::new(v))
        }
    }

    pub(crate) fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Inner, D::Error>
    where
        D: Deserializer<'de>,
    {
        let iv = deserializer.deserialize_bytes(IVecVisitor)?;
        let data: Inner = iv.data;
        std::mem::forget(iv);
        Ok(data)
    }
}

#[test]
fn ivec_usage() {
    let iv1: IVec = vec![1, 2, 3].into();
    assert_eq!(iv1, vec![1, 2, 3]);
    let iv2 = IVec::new(&[4; 128]);
    assert_eq!(iv2, vec![4; 128]);
}
