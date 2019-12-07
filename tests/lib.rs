extern crate evmap;

#[test]
fn it_works() {
    let x = ('x', 42);

    let (r, mut w) = evmap::new();

    // the map is uninitialized, so all lookups should return None
    assert_eq!(r.get_and(&x.0, |rs| rs.len()), None);

    w.refresh();

    // after the first refresh, it is empty, but ready
    assert_eq!(r.get_and(&x.0, |rs| rs.len()), None);
    // since we're not using `meta`, we get ()
    assert_eq!(r.meta_get_and(&x.0, |rs| rs.len()), Some((None, ())));

    w.insert(x.0, x);

    // it is empty even after an add (we haven't refresh yet)
    assert_eq!(r.get_and(&x.0, |rs| rs.len()), None);
    assert_eq!(r.meta_get_and(&x.0, |rs| rs.len()), Some((None, ())));

    w.refresh();

    // but after the swap, the record is there!
    assert_eq!(r.get_and(&x.0, |rs| rs.len()), Some(1));
    assert_eq!(r.meta_get_and(&x.0, |rs| rs.len()), Some((Some(1), ())));
    assert_eq!(
        r.get_and(&x.0, |rs| rs.iter().any(|v| v.0 == x.0 && v.1 == x.1)),
        Some(true)
    );

    // non-existing records return None
    assert_eq!(r.get_and(&'y', |rs| rs.len()), None);
    assert_eq!(r.meta_get_and(&'y', |rs| rs.len()), Some((None, ())));

    // if we purge, the readers still see the values
    w.purge();
    assert_eq!(
        r.get_and(&x.0, |rs| rs.iter().any(|v| v.0 == x.0 && v.1 == x.1)),
        Some(true)
    );

    // but once we refresh, things will be empty
    w.refresh();
    assert_eq!(r.get_and(&x.0, |rs| rs.len()), None);
    assert_eq!(r.meta_get_and(&x.0, |rs| rs.len()), Some((None, ())));
}

#[test]
#[cfg(not(miri))]
// https://github.com/rust-lang/miri/issues/658
fn paniced_reader_doesnt_block_writer() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.refresh();

    // reader panics
    let r = std::panic::catch_unwind(move || r.get_and(&1, |_| panic!()));
    assert!(r.is_err());

    // writer should still be able to continue
    w.insert(1, "b");
    w.refresh();
    w.refresh();
}

#[test]
fn read_after_drop() {
    let x = ('x', 42);

    let (r, mut w) = evmap::new();
    w.insert(x.0, x);
    w.refresh();
    assert_eq!(r.get_and(&x.0, |rs| rs.len()), Some(1));

    // once we drop the writer, the readers should see empty maps
    drop(w);
    assert_eq!(r.get_and(&x.0, |rs| rs.len()), None);
    assert_eq!(r.meta_get_and(&x.0, |rs| rs.len()), None);
}

#[test]
fn clone_types() {
    let x = evmap::shallow_copy::CopyValue::from(b"xyz");

    let (r, mut w) = evmap::new();
    w.insert(&*x, x);
    w.refresh();

    assert_eq!(r.get_and(&*x, |rs| rs.len()), Some(1));
    assert_eq!(r.meta_get_and(&*x, |rs| rs.len()), Some((Some(1), ())));
    assert_eq!(r.get_and(&*x, |rs| rs.iter().any(|v| *v == x)), Some(true));
}

#[test]
#[cfg(not(miri))]
fn busybusybusy_fast() {
    busybusybusy_inner(false);
}
#[test]
#[cfg(not(miri))]
fn busybusybusy_slow() {
    busybusybusy_inner(true);
}

#[cfg(not(miri))]
fn busybusybusy_inner(slow: bool) {
    use std::thread;
    use std::time;

    let threads = 4;
    let mut n = 1000;
    if !slow {
        n *= 100;
    }
    let (r, mut w) = evmap::new();

    let rs: Vec<_> = (0..threads)
        .map(|_| {
            let r = r.clone();
            thread::spawn(move || {
                // rustfmt
                for i in 0..n {
                    let i = i.into();
                    loop {
                        match r.get_and(&i, |rs| {
                            if slow {
                                thread::sleep(time::Duration::from_millis(2));
                            }
                            Vec::from(rs)
                        }) {
                            Some(rs) => {
                                assert_eq!(rs.len(), 1);
                                assert_eq!(rs[0], i);
                                break;
                            }
                            None => {
                                thread::yield_now();
                            }
                        }
                    }
                }
            })
        })
        .collect();

    for i in 0..n {
        w.insert(i, i);
        w.refresh();
    }

    for r in rs {
        r.join().unwrap();
    }
}

#[test]
#[cfg(not(miri))]
fn busybusybusy_heap() {
    use std::thread;

    let threads = 2;
    let n = 1000;
    let (r, mut w) = evmap::new::<_, Vec<_>>();

    let rs: Vec<_> = (0..threads)
        .map(|_| {
            let r = r.clone();
            thread::spawn(move || {
                for i in 0..n {
                    let i = i.into();
                    loop {
                        match r.get_and(&i, |rs| Vec::from(rs)) {
                            Some(rs) => {
                                assert_eq!(rs.len(), 1);
                                assert_eq!(rs[0].len(), i);
                                break;
                            }
                            None => {
                                thread::yield_now();
                            }
                        }
                    }
                }
            })
        })
        .collect();

    for i in 0..n {
        w.insert(i, (0..i).collect());
        w.refresh();
    }

    for r in rs {
        r.join().unwrap();
    }
}

#[test]
fn minimal_query() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.refresh();
    w.insert(1, "b");

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(1));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"a")).unwrap());
}

#[test]
fn clear_vs_empty() {
    let (r, mut w) = evmap::new::<_, ()>();
    w.refresh();
    assert_eq!(r.get_and(&1, |rs| rs.len()), None);
    w.clear(1);
    w.refresh();
    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(0));
    w.empty(1);
    w.refresh();
    assert_eq!(r.get_and(&1, |rs| rs.len()), None);
    // and again to test both apply_first and apply_second
    w.clear(1);
    w.refresh();
    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(0));
    w.empty(1);
    w.refresh();
    assert_eq!(r.get_and(&1, |rs| rs.len()), None);
}

#[test]
fn non_copy_values() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a".to_string());
    w.refresh();
    w.insert(1, "b".to_string());

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(1));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == "a")).unwrap());
}

#[test]
fn non_minimal_query() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.refresh();
    w.insert(1, "c");

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(2));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"a")).unwrap());
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"b")).unwrap());
}

#[test]
fn absorb_negative_immediate() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.remove(1, "a");
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(1));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"b")).unwrap());
}

#[test]
fn absorb_negative_later() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.refresh();
    w.remove(1, "a");
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(1));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"b")).unwrap());
}

#[test]
fn absorb_multi() {
    let (r, mut w) = evmap::new();
    w.extend(vec![(1, "a"), (1, "b")]);
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(2));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"a")).unwrap());
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"b")).unwrap());

    w.remove(1, "a");
    w.insert(1, "c");
    w.remove(1, "c");
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(1));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"b")).unwrap());
}

#[test]
fn empty() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.insert(2, "c");
    w.empty(1);
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), None);
    assert_eq!(r.get_and(&2, |rs| rs.len()), Some(1));
    assert!(r.get_and(&2, |rs| rs.iter().any(|r| r == &"c")).unwrap());
}

#[test]
#[cfg(feature = "indexed")]
fn empty_random() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.insert(2, "c");
    w.empty_at_index(0);
    w.refresh();

    // should only have one value set left
    assert_eq!(r.len(), 1);
    // one of them must have gone away
    assert!(
        (!r.contains_key(&1) && r.contains_key(&2)) || (r.contains_key(&1) && !r.contains_key(&2))
    );
}

#[test]
fn empty_post_refresh() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.insert(2, "c");
    w.refresh();
    w.empty(1);
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), None);
    assert_eq!(r.get_and(&2, |rs| rs.len()), Some(1));
    assert!(r.get_and(&2, |rs| rs.iter().any(|r| r == &"c")).unwrap());
}

#[test]
fn clear() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.insert(2, "c");
    w.clear(1);
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(0));
    assert_eq!(r.get_and(&2, |rs| rs.len()), Some(1));

    w.clear(2);
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(0));
    assert_eq!(r.get_and(&2, |rs| rs.len()), Some(0));

    w.empty(1);
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), None);
    assert_eq!(r.get_and(&2, |rs| rs.len()), Some(0));
}

#[test]
fn replace() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.insert(2, "c");
    w.update(1, "x");
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(1));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"x")).unwrap());
    assert_eq!(r.get_and(&2, |rs| rs.len()), Some(1));
    assert!(r.get_and(&2, |rs| rs.iter().any(|r| r == &"c")).unwrap());
}

#[test]
fn replace_post_refresh() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.insert(2, "c");
    w.refresh();
    w.update(1, "x");
    w.refresh();

    assert_eq!(r.get_and(&1, |rs| rs.len()), Some(1));
    assert!(r.get_and(&1, |rs| rs.iter().any(|r| r == &"x")).unwrap());
    assert_eq!(r.get_and(&2, |rs| rs.len()), Some(1));
    assert!(r.get_and(&2, |rs| rs.iter().any(|r| r == &"c")).unwrap());
}

#[test]
fn with_meta() {
    let (r, mut w) = evmap::with_meta::<usize, usize, _>(42);
    assert_eq!(r.meta_get_and(&1, |rs| rs.len()), None);
    w.refresh();
    assert_eq!(r.meta_get_and(&1, |rs| rs.len()), Some((None, 42)));
    w.set_meta(43);
    assert_eq!(r.meta_get_and(&1, |rs| rs.len()), Some((None, 42)));
    w.refresh();
    assert_eq!(r.meta_get_and(&1, |rs| rs.len()), Some((None, 43)));
}

#[test]
fn map_into() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.insert(2, "c");
    w.refresh();
    w.insert(1, "x");

    use std::collections::HashMap;
    let copy: HashMap<_, Vec<_>> = r.map_into(|&k, vs| (k, vs.to_vec()));

    assert_eq!(copy.len(), 2);
    assert!(copy.contains_key(&1));
    assert!(copy.contains_key(&2));
    assert_eq!(copy[&1].len(), 2);
    assert_eq!(copy[&2].len(), 1);
    assert_eq!(copy[&1], vec!["a", "b"]);
    assert_eq!(copy[&2], vec!["c"]);
}

#[test]
#[cfg(not(miri))]
fn clone_churn() {
    use std::thread;
    let (r, mut w) = evmap::new();

    thread::spawn(move || loop {
        let r = r.clone();
        if let Some(_) = r.get_and(&1, |rs| rs.len()) {
            thread::yield_now();
        }
    });

    for i in 0..1000 {
        w.insert(1, i);
        w.refresh();
    }
}

#[test]
fn foreach() {
    let (r, mut w) = evmap::new();
    w.insert(1, "a");
    w.insert(1, "b");
    w.insert(2, "c");
    w.refresh();
    w.insert(1, "x");

    r.for_each(|&k, vs| match k {
        1 => assert_eq!(vs, &*vec!["a", "b"]),
        2 => assert_eq!(vs, &*vec!["c"]),
        _ => unreachable!(),
    });
}

#[test]
fn retain() {
    // do same operations on a plain vector
    // to verify retain implementation
    let mut v = Vec::new();
    let (r, mut w) = evmap::new();

    for i in 0..50 {
        w.insert(0, i);
        v.push(i);
    }

    fn is_even(num: &i32, _: bool) -> bool {
        num % 2 == 0
    }

    unsafe { w.retain(0, is_even) }.refresh();
    v.retain(|i| is_even(i, false));

    r.get_and(&0, |nums| assert_eq!(v, nums)).unwrap();
}

#[test]
fn range_works() {
    use evmap::RangeLookup;
    use evmap::Options;
    use std::ops::Bound::{Included, Unbounded};

    let mut evmap_options = Options::default();
    evmap_options.set_ignore_interval_tree(false);
    let (r, mut w) = evmap_options.construct();

    assert!(r.meta_get_range_and(3..4, |rs| rs.len()).is_err());

    w.refresh();

    w.insert_range(vec![(3, 9), (3, 30), (4, 10)],  vec![(Included(2), Unbounded)]);

    assert!(r.meta_get_range_and(3..4, |rs| rs.len()).is_miss());

    w.refresh();

    match r.meta_get_range_and(3..4, |rs| rs.to_vec()) {
        RangeLookup::Ok(results, _) => {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0], vec![9, 30]);
        }
        _ => unreachable!()
    }

    match r.meta_get_range_and(3..=4, |rs| rs.to_vec()) {
        RangeLookup::Ok(results, _) => {
            assert_eq!(results.len(), 2);
            assert_eq!(results[0], vec![9, 30]);
            assert_eq!(results[1], vec![10]);
        }
        _ => unreachable!()
    }}
