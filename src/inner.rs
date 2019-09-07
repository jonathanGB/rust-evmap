use std::collections::BTreeMap;

#[cfg(not(feature = "smallvec"))]
pub(crate) type Values<T> = Vec<T>;

#[cfg(feature = "smallvec")]
pub(crate) type Values<T> = smallvec::SmallVec<[T; 1]>;

pub(crate) struct Inner<K, V, M>
where
    K: Ord,
{
    pub(crate) data: BTreeMap<K, Values<V>>,
    pub(crate) meta: M,
    ready: bool,
}

impl<K, V, M> Clone for Inner<K, V, M>
where
    K: Ord,
    M: Clone,
{
    fn clone(&self) -> Self {
        assert!(self.data.is_empty());
        Inner {
            data: BTreeMap::new(),
            meta: self.meta.clone(),
            ready: self.ready,
        }
    }
}

impl<K, V, M> Inner<K, V, M>
where
    K: Ord,
{
    pub fn with_meta(m: M) -> Self {
        Inner {
            data: BTreeMap::default(),
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
