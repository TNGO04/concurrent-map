#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate arbitrary;
extern crate concurrent_map;

use arbitrary::Arbitrary;

const KEYSPACE: u64 = 32;

#[derive(Debug)]
enum Op {
    Insert { key: u64, value: u64 },
    Remove { key: u64 },
    Range { start: u64, end: u64 },
}

impl<'a> Arbitrary<'a> for Op {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(if u.ratio(1, 2).unwrap_or(true) {
            Op::Insert {
                key: u.int_in_range(0..=KEYSPACE as u64).unwrap_or(0),
                value: u.int_in_range(0..=KEYSPACE as u64).unwrap_or(0),
            }
        } else if u.ratio(3, 4).unwrap_or(false) {
            Op::Remove {
                key: u.int_in_range(0..=KEYSPACE as u64).unwrap_or(0),
            }
        } else {
            let start = u.int_in_range(0..=KEYSPACE as u64).unwrap_or(0);
            let end = (start + 1).max(u.int_in_range(0..=KEYSPACE as u64).unwrap_or(0));
            Op::Range { start, end }
        })
    }
}

fuzz_target!(|ops: Vec<Op>| {
    let mut tree = concurrent_map::ConcurrentMap::default();
    let mut model = std::collections::BTreeMap::new();

    for op in ops {
        match op {
            Op::Insert { key, value } => {
                assert_eq!(
                    tree.insert(key, value).map(|arc| *arc),
                    model.insert(key, value)
                );
            }
            Op::Remove { key } => {
                assert_eq!(tree.remove(&key).map(|arc| *arc), model.remove(&key));
            }
            Op::Range { start, end } => {
                let mut model_iter = model.range(start..end);
                let mut tree_iter = tree.range(start..end);

                for (k1, v1) in &mut model_iter {
                    let (k2, v2) = tree_iter.next().unwrap();
                    assert_eq!((k1, v1), (&*k2, &*v2));
                }

                assert_eq!(tree_iter.next(), None);
            }
        };

        for (key, value) in &model {
            assert_eq!(tree.get(key).as_deref(), Some(value));
        }

        /* TODO
        for (key, value) in &tree {
            assert_eq!(model.get(key), Some(value));
        }
        */
    }

    let mut model_iter = model.iter();
    let mut tree_iter = tree.iter();

    for (k1, v1) in &mut model_iter {
        let (k2, v2) = tree_iter.next().unwrap();
        assert_eq!((k1, v1), (&*k2, &*v2));
    }

    assert_eq!(tree_iter.next(), None);
});
