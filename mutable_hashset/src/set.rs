use std::collections::hash_map;
use std::collections::hash_map::{RandomState, Values, ValuesMut};
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::iter::{Extend, Iterator};

pub struct MutSet<T, S = RandomState> {
    map: HashMap<u64, T, S>,
}

impl<T> MutSet<T, RandomState> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl<T, S> MutSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    pub fn insert(&mut self, value: T) -> bool {
        use core::hash::Hasher;

        let mut hasher = self.map.hasher().build_hasher();
        value.hash(&mut hasher);
        let key = hasher.finish();

        self.map.insert(key, value).is_none()
    }
}

impl<T, S> MutSet<T, S> {
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.map.values(),
        }
    }

    pub fn iter_mut(&mut self) -> ValuesMut<'_, u64, T> {
        self.map.values_mut()
    }
}

impl<T, S> Extend<T> for MutSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        use core::hash::Hasher;

        let mut hasher = self.map.hasher().build_hasher();

        self.map.extend(iter.into_iter().map(|value| {
            value.hash(&mut hasher);
            let key = hasher.finish();
            (key, value)
        }));
    }
}

impl<'a, T, S> IntoIterator for &'a MutSet<T, S> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T, S> IntoIterator for MutSet<T, S> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    #[inline]
    fn into_iter(self) -> IntoIter<T> {
        IntoIter {
            iter: self.map.into_iter(),
        }
    }
}

impl<T, S> fmt::Debug for MutSet<T, S>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}
pub struct Iter<'a, V: 'a> {
    iter: Values<'a, u64, V>,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<&'a V> {
        self.iter.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct IntoIter<V> {
    iter: hash_map::IntoIter<u64, V>,
}

impl<V> Iterator for IntoIter<V> {
    type Item = V;

    #[inline]
    fn next(&mut self) -> Option<V> {
        self.iter.next().map(|(_, v)| v)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}