use std::alloc::{alloc, dealloc, Layout};
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::iter::Iterator;
use std::marker;
use std::ptr;

#[derive(Debug)]
pub struct Node<T> {
    value: T,
    next: *mut Node<T>,
    prev: *mut Node<T>,
}

impl<T> Node<T> {
    unsafe fn new(value: T) -> *mut Node<T> {
        let layout = Layout::new::<Node<T>>();
        let curr = alloc(layout) as *mut Node<T>;
        curr.write(Node {
            value: value,
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
        });

        curr
    }

    unsafe fn drop(curr: *mut Node<T>) {
        let layout = Layout::new::<T>();
        dealloc(curr as *mut u8, layout);
    }
}

pub struct MutOrderedSet<T, S = RandomState> {
    map: HashMap<u64, *mut Node<T>, S>,
    head: *mut Node<T>,
    tail: *mut Node<T>,
}

impl<T> MutOrderedSet<T, RandomState> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            head: ptr::null_mut(),
            tail: ptr::null_mut(),
        }
    }
}

impl<T, S> MutOrderedSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    #[inline]
    pub fn insert(&mut self, value: T) -> &mut T {
        let key = self.get_key(&value);
        let curr = self.map.get(&key);

        match curr {
            Some(&curr) => unsafe {
                (*curr).value = value;

                &mut (*curr).value
            },
            None => unsafe {
                let curr = Node::new(value);

                self.map.insert(key, curr);
                self.attach(curr);

                &mut (*curr).value
            },
        }
    }

    #[inline]
    pub fn remove(&mut self, value: &T) -> bool {
        let key = self.get_key(&value);
        let curr = self.map.remove(&key);

        match curr {
            Some(curr) => unsafe {
                self.detach(curr);
                Node::drop(curr);

                true
            },
            None => false,
        }
    }

    #[inline]
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for value in iter {
            self.insert(value);
        }
    }

    fn get_key(&self, value: &T) -> u64 {
        use core::hash::Hasher;

        let mut hasher = self.map.hasher().build_hasher();
        value.hash(&mut hasher);
        hasher.finish()
    }

    unsafe fn attach(&mut self, curr: *mut Node<T>) {
        // prev and next
        (*curr).prev = self.tail;
        if !self.tail.is_null() {
            (*self.tail).next = curr;
        }

        // head and tail
        if self.head.is_null() {
            self.head = curr;
        }
        self.tail = curr;
    }

    unsafe fn detach(&mut self, curr: *mut Node<T>) {
        let prev = (*curr).prev;
        let next = (*curr).next;

        // prev and next
        if !prev.is_null() {
            (*prev).next = (*curr).next;
        }
        if !next.is_null() {
            (*next).prev = (*curr).prev;
        }

        // head and tail pointer
        if self.head == curr {
            self.head = next;
        }
        if self.tail == curr {
            self.tail = prev;
        }
    }
}

impl<T, S> MutOrderedSet<T, S> {
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            custer: self.head,
            _marker: marker::PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            custer: self.head,
            _marker: marker::PhantomData,
        }
    }
}

impl<T, S> IntoIterator for MutOrderedSet<T, S> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    #[inline]
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { custer: self.head }
    }
}

pub struct IntoIter<T> {
    custer: *mut Node<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.custer != ptr::null_mut() {
            unsafe {
                let node = *Box::from_raw(self.custer);
                self.custer = node.next;

                Some(node.value)
            }
        } else {
            None
        }
    }
}

pub struct Iter<'a, T> {
    custer: *mut Node<T>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if !self.custer.is_null() {
            unsafe {
                let r = Some(&(*self.custer).value);
                self.custer = (*self.custer).next;
                r
            }
        } else {
            None
        }
    }
}

pub struct IterMut<'a, T> {
    custer: *mut Node<T>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        if !self.custer.is_null() {
            unsafe {
                let r = Some(&mut (*self.custer).value);
                self.custer = (*self.custer).next;
                r
            }
        } else {
            None
        }
    }
}

impl<T, S> fmt::Debug for MutOrderedSet<T, S>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nodes = self
            .map
            .iter()
            .map(|(k, v)| unsafe { (k, &**v as &Node<T>) });

        f.debug_map()
            .key(&"head")
            .value(&self.head)
            .key(&"tail")
            .value(&self.tail)
            .entries(nodes)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert() {
        let mut set = MutOrderedSet::new();

        set.insert(16);
        set.insert(1);
        set.insert(13);
        set.insert(12);

        let set_v: Vec<i32> = set.into_iter().map(|x| x).collect();
        assert_eq!(set_v, [16, 1, 13, 12]);
    }

    #[test]
    fn insert_repeat_value() {
        let mut set = MutOrderedSet::new();

        set.insert(16);
        set.insert(1);
        set.insert(13);
        set.insert(12);
        set.insert(1);
        set.insert(16);

        let set_v: Vec<i32> = set.into_iter().map(|x| x).collect();
        assert_eq!(set_v, [16, 1, 13, 12]);
    }

    #[test]
    fn insert_and_remove() {
        let mut set = MutOrderedSet::new();

        set.insert(16);
        set.insert(1);
        set.insert(13);
        set.remove(&16);
        set.remove(&1);
        set.insert(16);
        set.insert(13);
        set.insert(9);

        let set_v: Vec<i32> = set.into_iter().map(|x| x).collect();
        assert_eq!(set_v, [13, 16, 9]);
    }

    #[test]
    fn iter() {
        let mut set = MutOrderedSet::new();

        set.insert(1);
        set.insert(5);
        set.insert(2);

        let mut set_v1: Vec<i32> = set.iter().map(|x| x * x).collect();
        let set_v2: Vec<i32> = set.iter().map(|x| x * 2).collect();
        set_v1.extend(set_v2);

        assert_eq!(set_v1, [1, 25, 4, 2, 10, 4]);
    }

    #[test]
    fn iter_mut() {
        let mut set = MutOrderedSet::new();

        set.insert(1);
        set.insert(5);
        set.insert(2);

        for x in set.iter_mut() {
            *x *= 2;
        }
        let set_v: Vec<&i32> = set.iter().collect();

        assert_eq!(set_v, [&2, &10, &4]);
    }
}
