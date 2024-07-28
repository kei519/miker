//! Provides collections.

use alloc::vec;
use alloc::vec::Vec;
use core::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
    mem,
};

use crate::hash::BuildFnvHasher;

/// key-value entry.
#[derive(Debug, Clone)]
struct Entry<K, V> {
    key: K,
    value: V,
}

/// Bucket for key-value entry.
#[derive(Debug, Clone)]
enum Bucket<K, V> {
    /// Contains no item.
    None,
    /// Contains an entry.
    Some(Entry<K, V>),
    /// Used before and now empty. Skip it on searching values.
    Tombstone,
}

/// FNV hash map.
#[derive(Debug, Clone)]
pub struct HashMap<K: Hash + Eq, V> {
    /// Holding buckets.
    buckets: Vec<Bucket<K, V>>,
    /// Number of not empty buckets.
    used: usize,
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    /// Capacity when initialized.
    const INIT_CAP: usize = 16;
    /// Keep used ratio below this value when rehased.
    const LOW_LIMIT: usize = 50;
    /// When used ratio is no less than this value, rehash.
    const HIGH_LIMIT: usize = 70;

    /// Constructs new [`HashMap`].
    pub const fn new() -> Self {
        // To make `new()` a const function, initialization will be carried out at first insertion.
        Self {
            buckets: vec![],
            used: 0,
        }
    }

    /// Insert `value`, whose key is `key`. If the same `key` value is already contained, returns
    /// it.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.used_ratio() >= Self::HIGH_LIMIT {
            self.rehash();
        }

        let index = BuildFnvHasher.hash_one(&key) as usize % self.capacity();
        let (second, first) = self.buckets.split_at_mut(index);
        for bucket in first.iter_mut().chain(second.iter_mut()) {
            match *bucket {
                // Tombstone is also usable.
                Bucket::None | Bucket::Tombstone => {
                    self.used += 1;
                    *bucket = Bucket::Some(Entry { key, value });
                    return None;
                }
                Bucket::Some(ref mut entry) => {
                    if entry.key != key {
                        continue;
                    }
                    let old = mem::replace(entry, Entry { key, value });
                    return Some(old.value);
                }
            }
        }
        // There must be some empty spaces.
        unreachable!();
    }

    /// Removes and returns the value, whose key is `key` if it is contained.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.capacity() == 0 {
            return None;
        }

        let index = BuildFnvHasher.hash_one(key) as usize % self.capacity();
        let (second, first) = self.buckets.split_at_mut(index);
        for bucket in first.iter_mut().chain(second.iter_mut()) {
            match *bucket {
                Bucket::None => return None,
                Bucket::Tombstone => continue,
                Bucket::Some(ref entry) => {
                    if entry.key.borrow() == key {
                        let taken = mem::replace(bucket, Bucket::Tombstone);
                        let Bucket::Some(entry) = taken else {
                            // Because of matching.
                            unreachable!()
                        };
                        self.used -= 1;
                        return Some(entry.value);
                    } else {
                        continue;
                    }
                }
            }
        }
        // There must be some empty spaces.
        unreachable!();
    }

    /// Returns the shared reference to value, whose key is `key` if it is contained.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.capacity() == 0 {
            return None;
        }

        let index = BuildFnvHasher.hash_one(key) as usize % self.capacity();
        let (second, first) = self.buckets.split_at(index);
        for bucket in first.iter().chain(second.iter()) {
            match *bucket {
                Bucket::None => return None,
                Bucket::Tombstone => continue,
                Bucket::Some(ref entry) => {
                    if entry.key.borrow() == key {
                        return Some(&entry.value);
                    } else {
                        continue;
                    }
                }
            }
        }
        // There must be some empty spaces.
        unreachable!()
    }

    /// Returns the exclusive reference to value, whose key is `key` if it is contained.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if self.capacity() == 0 {
            return None;
        }

        let index = BuildFnvHasher.hash_one(key) as usize % self.capacity();
        let (second, first) = self.buckets.split_at_mut(index);
        for bucket in first.iter_mut().chain(second.iter_mut()) {
            match *bucket {
                Bucket::None => return None,
                Bucket::Tombstone => continue,
                Bucket::Some(ref mut entry) => {
                    if entry.key.borrow() == key {
                        return Some(&mut entry.value);
                    } else {
                        continue;
                    }
                }
            }
        }
        // There must be some empty spaces.
        unreachable!()
    }

    /// Capacity of [`HashMpa`].
    pub fn capacity(&self) -> usize {
        self.buckets.len()
    }

    fn used_ratio(&self) -> usize {
        // Returns 100 because there is no space to insert.
        if self.capacity() == 0 {
            100
        } else {
            self.used * 100 / self.capacity()
        }
    }

    /// Rehash the map.
    fn rehash(&mut self) {
        let mut cap = self.capacity();
        // When not initialized.
        if cap == 0 {
            cap = Self::INIT_CAP as _
        } else {
            // Keep ratio below `LOW_LIMIT`.
            while self.used * 100 / cap >= Self::LOW_LIMIT {
                cap *= 2;
            }
        };
        let new_cap = cap;

        let new_buckets = (0..new_cap).map(|_| Bucket::None).collect();
        let old_buckets = mem::replace(&mut self.buckets, new_buckets);
        self.used = 0;

        // Rehasing.
        for bucket in old_buckets {
            let Bucket::Some(entry) = bucket else {
                continue;
            };
            self.insert(entry.key, entry.value);
        }
    }
}

impl<K: Hash + Eq, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
