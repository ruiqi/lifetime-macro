/*!
`alias_trie` is a trie that supports nodes with aliases. One or more aliases can be set for each node so that the stored value can be retrieved through the alias. Unfortunately, the aliases in the search path may conflict, making it impossible to locate a specific node. This situation is called grammatical ambiguity, and the search cannot be completed using ambiguity grammar.

`alias_trie` can easily and quickly deal with grammatical ambiguities caused by abbreviations, abbreviations or abbreviations in daily grammar, and provide correct and fast retrieval capabilities.

```nothing
                        ROOT
                     ↙        ↘
                A(a): 1        G: 5
            ↙      ↓     ↘                // The alias "f" conflicts.
          B       C: 4    D(d,dd): 2       // The path "A->d->f" is an ambiguous grammar.
          ↓              ↙       ↘
      E(eee): 2     F1(f): 3    F2(f,f2): 4
```

# Usage

This crate is [on crates.io](https://crates.io/crates/alias_trie) and can be
used by adding `alias_trie` to your dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
alias_trie = "0.9"
```

# Examples

```rust
use alias_trie::Trie;
use alias_trie::UniqueOption;

let mut trie = Trie::new();

trie.insert(&["A"], 1);
trie.insert(&["A", "B", "E"], 2);
trie.insert(&["A", "C"], 4);
trie.insert(&["A", "D"], 2);
trie.insert(&["A", "D", "F1"], 3);
trie.insert(&["A", "D", "F2"], 4);
trie.insert(&["G"], 5);

trie.update_aliases(&["A", "B", "E"], &[&["a"], &[], &["eee"]]);
trie.update_aliases(&["A", "D", "F1"], &[&[], &["d", "dd"], &["f"]]);
trie.update_aliases(&["A", "D", "F2"], &[&[], &[], &["f", "f2"]]);

assert_eq!(trie.get(&["a", "d", "F1"]), UniqueOption::Some(&3));
assert_eq!(trie.get(&["a", "d", "f2"]), UniqueOption::Some(&4));
assert_eq!(trie.get(&["a", "d", "f"]), UniqueOption::NonUnique);
assert_eq!(trie.get(&["a", "d", "g"]), UniqueOption::None);
```

!*/

use std::alloc::{alloc, Layout};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum UniqueOption<V> {
    Some(V),
    NonUnique,
    None,
}

impl<V> UniqueOption<V> {
    pub fn unwrap(self) -> V {
        match self {
            UniqueOption::Some(v) => v,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
struct Node<K, V> {
    value: Option<V>,
    links: HashMap<K, Option<*mut Node<K, V>>>,
}

impl<K, V> Node<K, V>
where
    K: Eq + Hash + Clone + Debug,
    V: Debug,
{
    fn new() -> *mut Node<K, V> {
        unsafe {
            let layout = Layout::new::<Node<K, V>>();
            let curr = alloc(layout) as *mut Node<K, V>;

            curr.write(Node {
                value: None,
                links: HashMap::new(),
            });

            curr
        }
    }

    fn insert(&mut self, path: &[K], value: V) -> bool {
        //println!("[node] insert(path, value): {:?}, {:?}", path, value);

        let name = path[0].clone();
        let next = self.links.entry(name).or_insert(Some(Node::new()));

        //println!("[node] insert(next): {:?}", next);

        match next {
            Some(next) => unsafe {
                if path.len() == 1 {
                    (**next).value = Some(value);
                    //println!("[node] insert(next:value): {:?}", (**next));

                    true
                } else {
                    (**next).insert(&path[1..], value)
                }
            },
            None => false,
        }
    }

    fn update_aliases<Q>(&mut self, path: &[&Q], path_aliases: &[&[K]])
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + Debug,
    {
        //println!(
        //    "[node] update_aliases(path, path_aliases): {:?}, {:?}",
        //    path, path_aliases
        //);
        //println!("[node] update_aliases(self): {:?}", self);

        if !path.is_empty() {
            let name = path[0].clone();
            let aliases = path_aliases[0].clone();

            let next1 = self.links.get(name).unwrap().unwrap();
            for alies in aliases {
                match self.links.get(alies.borrow()) {
                    Some(Some(next2)) => {
                        if next1 != *next2 {
                            self.links.insert((*alies).clone(), None);
                        }
                    }
                    Some(None) => {}
                    None => {
                        self.links.insert((*alies).clone(), Some(next1));
                    }
                }
            }

            unsafe {
                (*next1).update_aliases(&path[1..], &path_aliases[1..]);
            }
        }
    }

    fn get_mut<Q>(&mut self, path: &[&Q]) -> UniqueOption<*mut Node<K, V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + Debug,
    {
        //println!("[node] get_mut(path): {:?}", path);
        //println!("[node] get_mut(self): {:?}", self);

        if path.len() == 0 {
            UniqueOption::Some(self as *mut Node<K, V>)
        } else {
            let name = path[0];

            //println!("[node] get_mut(self.links.get({:?})): {:?}", name, self.links.get(name));

            match self.links.get(name) {
                Some(Some(next)) => {
                    if path.len() == 1 {
                        UniqueOption::Some(*next)
                    } else {
                        unsafe { (**next).get_mut(&path[1..]) }
                    }
                }
                Some(None) => UniqueOption::NonUnique,
                None => UniqueOption::None,
            }
        }
    }
}

#[derive(Debug)]
pub struct Trie<K, V> {
    root: *mut Node<K, V>,
}

impl<K, V> Trie<K, V>
where
    K: Eq + Hash + Clone + Debug,
    V: Debug,
{
    pub fn new() -> Self {
        Self { root: Node::new() }
    }

    pub fn insert(&mut self, path: &[K], value: V) {
        //println!("\n[trie] insert(path, value): {:?}, {:?}", path, value);
        unsafe {
            (*self.root).insert(path, value);
        }
    }

    pub fn update_aliases<Q>(&mut self, path: &[&Q], aliases: &[&[K]]) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + Debug,
    {
        //println!("\n[trie] update_aliases(path, aliases): {:?}, {:?}", path, aliases);
        unsafe {
            match (*self.root).get_mut(path) {
                UniqueOption::Some(_) => {
                    (*self.root).update_aliases(path, aliases);
                    true
                }
                _ => false,
            }
        }
    }

    pub fn get<Q>(&self, path: &[&Q]) -> UniqueOption<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + Debug,
    {
        //println!("\n[trie] get(path): {:?}", path);
        unsafe {
            match (*self.root).get_mut(path) {
                UniqueOption::Some(node_ptr) => {
                    //println!("[trie] get(node_ptr): {:?}", node_ptr);

                    match (*node_ptr).value {
                        Some(ref v) => UniqueOption::Some(v),
                        None => UniqueOption::None,
                    }
                }
                UniqueOption::None => UniqueOption::None,
                UniqueOption::NonUnique => UniqueOption::NonUnique,
            }
        }
    }

    pub fn get_mut<Q>(&mut self, path: &[&Q]) -> UniqueOption<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + Debug,
    {
        //println!("\n[trie] get_mut(path): {:?}", path);
        unsafe {
            match (*self.root).get_mut(path) {
                UniqueOption::Some(node_ptr) => {
                    //println!("[trie] get_mut(node_ptr): {:?}", node_ptr);

                    match (*node_ptr).value {
                        Some(ref mut v) => UniqueOption::Some(v),
                        None => UniqueOption::None,
                    }
                }
                UniqueOption::None => UniqueOption::None,
                UniqueOption::NonUnique => UniqueOption::NonUnique,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test1() {
        let mut trie = Trie::new();
        trie.insert(&["a", "b", "c"], 100);
        assert_eq!(
            trie.update_aliases(&["a", "b"], &[&["A", "aaa"], &["bbb", "b4"]]),
            true
        );

        let value = trie.get(&["A", "b4", "c"]);
        assert_eq!(value, UniqueOption::Some(&100));
    }

    #[test]
    fn test2() {
        let mut trie = Trie::new();
        trie.insert(&["a", "b", "c"], 100);
        assert_eq!(
            trie.update_aliases(&["a", "b"], &[&["A", "aaa"], &["bbb", "b4"]]),
            true
        );

        let value = trie.get(&["A", "b4", "C"]);
        assert_eq!(value, UniqueOption::None);
    }

    #[test]
    fn test3() {
        let mut trie = Trie::new();
        trie.insert(&["a", "b", "c"], "c");
        trie.insert(&["a", "b", "d", "e"], "e");
        assert_eq!(
            trie.update_aliases(
                &["a", "b", "c"],
                &[&["A", "aaa"], &["bbb", "b4"], &["C", "X"]],
            ),
            true
        );
        assert_eq!(
            trie.update_aliases(
                &["a", "b", "d", "e"],
                &[&[], &[], &["D", "X"], &["E", "ee"]],
            ),
            true
        );
        assert_eq!(
            trie.update_aliases(&["a", "b", "X", "e"], &[&[], &[], &["DDD"], &["e5"]],),
            false
        );
        assert_eq!(
            trie.update_aliases(&["A", "b", "D", "ee"], &[&[], &[], &["DDDD"], &["e6"]],),
            true
        );

        let value = trie.get(&["A", "b4", "C"]);
        assert_eq!(value, UniqueOption::Some(&"c"));

        let value = trie.get(&["A", "b4", "X"]);
        assert_eq!(value, UniqueOption::NonUnique);

        let value = trie.get(&["A", "b4", "d", "ee"]);
        assert_eq!(value, UniqueOption::Some(&"e"));

        let value = trie.get(&["A", "b4", "X", "e6"]);
        assert_eq!(value, UniqueOption::NonUnique);

        let value = trie.get(&["A", "b4", "DDDD", "ee"]);
        assert_eq!(value, UniqueOption::Some(&"e"));
    }

    #[test]
    fn test4() {
        let mut trie = Trie::new();
        trie.insert(&["a", "b", "c"], 100);
        trie.update_aliases(&["a", "b"], &[&["A", "aaa"], &["bbb", "b4"]]);

        let value = trie.get_mut(&["A", "b4", "c"]).unwrap();
        *value = 1000;

        let value = trie.get(&["A", "b4", "c"]);
        assert_eq!(value, UniqueOption::Some(&1000));
    }

    #[test]
    fn test5() {
        let names = vec![
            "first/Output!",
            "first/Output![Demo6A,0].0",
            "first/Output![Demo6A,0].1",
            "first/self",
            "self.b[Demo6B,0].Double",
            "self.b[Demo6B,0].Double",
            "self.b[Demo6B,0].Double[Demo6A,0].0",
            "self.b[Demo6B,0].Double[Demo6A,1].0",
            "self.b[Demo6B,0].Double[Demo6A,0].1",
            "self.b[Demo6B,0].Double[Demo6A,1].1",
            "self.b[Demo6B,0].Multiple",
            "self.b[Demo6B,0].Multiple",
            "self.b[Demo6B,0].Multiple[Demo6A,0].0",
            "self.b[Demo6B,0].Multiple[Demo6A,0].1",
            "self.b[Demo6B,0].Single",
            "self.b[Demo6B,0].Single[Demo6A,0].0",
            "self.b[Demo6B,0].Single[Demo6A,0].1",
        ];

        let mut trie = Trie::new();
        for name in names {
            let name = format!("{}$", name);
            let path = &name.split(".").map(|s| s.to_string()).collect::<Vec<_>>()[..];
            trie.insert(path, name);

            let re = Regex::new(r"\[[^\[\]]+\]").unwrap();
            let path_aliases = path
                .iter()
                .map(|cell1| {
                    let cell2 = re.replace(cell1, "").to_string();
                    if cell1.len() == cell2.len() {
                        vec![]
                    } else {
                        vec![cell2]
                    }
                })
                .collect::<Vec<_>>();

            let path = &path.iter().map(|s| s).collect::<Vec<_>>()[..];
            let path_aliases = &path_aliases
                .iter()
                .map(|alies| &alies[..])
                .collect::<Vec<_>>()[..];

            trie.update_aliases(path, path_aliases);
        }

        let value = trie.get(&["first/Output!$"]);
        assert_eq!(value, UniqueOption::Some(&"first/Output!$".to_string()));

        let value = trie.get(&["first/Output!", "0$"]);
        assert_eq!(
            value,
            UniqueOption::Some(&"first/Output![Demo6A,0].0$".to_string())
        );

        let value = trie.get(&["first/Output!", "1$"]);
        assert_eq!(
            value,
            UniqueOption::Some(&"first/Output![Demo6A,0].1$".to_string())
        );

        let value = trie.get(&["self", "b", "Double$"]);
        assert_eq!(
            value,
            UniqueOption::Some(&"self.b[Demo6B,0].Double$".to_string())
        );

        let value = trie.get(&["self", "b", "Double", "0$"]);
        assert_eq!(value, UniqueOption::NonUnique);

        let value = trie.get(&["self", "b", "Double", "1$"]);
        assert_eq!(value, UniqueOption::NonUnique);

        let value = trie.get(&["self", "b", "Double[Demo6A,1]", "1$"]);
        assert_eq!(
            value,
            UniqueOption::Some(&"self.b[Demo6B,0].Double[Demo6A,1].1$".to_string())
        );
    }
}
