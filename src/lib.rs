extern crate libc;
extern crate crossbeam;
extern crate rustc_serialize;
extern crate bincode;
extern crate time;

// transactional kv with multi-key ops
pub use db::DB;
// atomic lock-free tree
pub use tree::Tree;
// lock-free pagecache
pub use page::{Pager, MemPager, PersistentPager};
// lock-free log-structured storage
pub use log::Log;
// lock-free stack
pub use stack::Stack;
// lock-free radix tree
pub use radix::Radix;

use crc16::crc16_arr;

macro_rules! rep_no_copy {
    ($e:expr; $n:expr) => {
        {
            let mut v = Vec::with_capacity($n);
            for _ in 0..$n {
                v.push($e);
            }
            v
        }
    };
}

macro_rules! read_or_break {
    ($file:expr, $buf:expr, $count:expr) => (
        match $file.read(&mut $buf) {
            Ok(n) if n == $buf.len() => {
                $count += n;
            },
            Ok(_) => {
                // tear occurred here
                break;
            },
            Err(_) => {
                break
            }
        }
    )
}

mod db;
mod tree;
mod log;
mod crc16;
mod stack;
mod map;
mod page;
mod radix;

pub mod ops;

// ID types
type PageID = u64;
type LogID = u64; // LogID == file position to simplify file mapping
type TxID = u64;

type Key = Vec<u8>;
type Value = Vec<u8>;
struct Tx;

#[derive(Clone)]
pub struct Annotation;

#[derive(Clone)]
pub struct Page {
    data: Data,
    lo_k: Key,
    hi_k: Key,
    next: PageID,
}

impl Default for Page {
    fn default() -> Page {
        Page {
            data: Data::Leaf(vec![]),
            lo_k: vec![],
            hi_k: vec![],
            next: 0,
        }
    }
}

#[derive(Clone)]
pub enum Data {
    // (separator, pointer)
    Index(Vec<(Key, PageID)>),
    Leaf(Vec<(Key, Value)>),
    Delta(Delta),
}

#[derive(Clone)]
pub enum Delta {
    Update(Key, Value),
    Insert(Key, Value),
    Delete(Key),
    DeleteNode,
    MergePage {
        right: *mut stack::Node<Page>,
        right_hi_k: Key,
    },
    MergeIndex {
        lo_k: Key,
        hi_k: Key,
    },
    SplitPage {
        split_key: Key,
        right: PageID,
    },
    SplitIndex {
        left_k: Key,
        right_k: Key,
        right: PageID,
    },
    TxBegin(TxID), // in-mem
    TxCommit(TxID), // in-mem
    TxAbort(TxID), // in-mem
    Load, // should this be a swap operation on the data pointer?
    Flush {
        annotation: Annotation,
        highest_lsn: TxID,
    }, // in-mem
    PartialSwap(LogID), /* indicates part of page has been swapped out,
                         * shows where to find it */
}