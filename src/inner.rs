use std::collections::BTreeMap;
use unbounded_interval_tree::IntervalTree;

#[cfg(not(feature = "smallvec"))]
pub(crate) type Values<T> = Vec<T>;

#[cfg(feature = "smallvec")]
pub(crate) type Values<T> = smallvec::SmallVec<[T; 1]>;

pub(crate) struct Inner<K, V, M>
where
    K: Ord + Clone,
{
    pub(crate) data: BTreeMap<K, Values<V>>,
    pub(crate) tree: Option<IntervalTree<K>>,
    pub(crate) meta: M,
    ready: bool,
}

impl<K, V, M> Clone for Inner<K, V, M>
where
    K: Ord + Clone,
    M: Clone,
{
    fn clone(&self) -> Self {
        assert!(self.data.is_empty());
        Inner {
            data: BTreeMap::new(),
            tree: self.tree.clone(),
            meta: self.meta.clone(),
            ready: self.ready,
        }
    }
}

impl<K, V, M> Inner<K, V, M>
where
    K: Ord + Clone,
{
    pub fn with_meta(m: M) -> Self {
        Inner {
            data: BTreeMap::default(),
            tree: Some(IntervalTree::default()),
            meta: m,
            ready: false,
        }
    }

    pub fn mark_ready(&mut self) {
        self.ready = true;
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}
