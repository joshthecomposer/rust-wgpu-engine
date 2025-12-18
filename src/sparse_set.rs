use core::slice;
use std::ops::{Deref, DerefMut};

/// An implementation of a sparse set.
///
/// A sparse set is a specialized data structure for representing a set of integers.
/// It can be useful in some very narrow and specific cases, namely when the universe of possible
/// values is very large but used very sparingly and the set is iterated often or cleared often.
///
/// In this implement the SparseSet can hold an arbitrary value for every integer (key) in the set.
///
/// # Example
///
/// ```
/// use sparseset::SparseSet;
/// let mut set = SparseSet::with_capacity(128);
/// set.insert(42, 3);
/// set.insert(77, 5);
/// set.insert(23, 8);
///
/// assert_eq!(*set.get(42).unwrap(), 3);
///
/// set.remove(42);
/// assert!(!set.get(42).is_some());
///
/// for entry in set {
///     println!("- {} => {}", entry.key(), entry.value);
/// }
/// ```
///
/// # Performance
///
/// Note that SparseSet is *incredibly* inefficient in terms of space. The O(1) insertion time
/// assumes space for the element is already allocated.  Otherwise, a large key may require a
/// massive reallocation, with no direct relation to the number of elements in the collection.
/// SparseSet should only be seriously considered for small keys.
///
/// ## Runtime complexity
///
/// See how the runtime complexity of SparseSet compares to Hash and Btree maps:
///
/// |           | get       | insert   | remove   | iterate | clear        |
/// |-----------|-----------|----------|----------|---------|--------------|
/// | SparseSet | O(1)      | O(1)*    | O(1)     | O(n)    | O(1) / O(n)* |
/// | HashMap   | O(1)~     | O(1)~*   | O(1)~    | N/A     | N/A          |
/// | BTreeMap  | O(log n)  | O(log n) | O(log n) | N/A     | N/A          |
///
/// * Clear is O(1) on simple types and O(n) on types whom implements Drop.
/// * Iterating is really efficient, its iterating over a dense array. In fact, its even possible
/// to get an (even mutable) slice of the entries in the set.
///
/// See http://research.swtch.com/sparse for more details.
#[derive(Debug)]
pub struct SparseSet<T> {
    pub dense: Vec<Entry<T>>,

    /// The value stored in the entry. A reference to it is returned by value() and value_mut(), as
    /// well as get() and get_mut() directly from SparseSet. The field can be used without going
    /// trough the accessors functions since it is public.
    pub sparse: Vec<usize>,
}

/// An entry in the sparse set.
/// You can retrieve a slice (possibly mutable) of [Entry] from the SparseSet.
#[derive(Debug)]
pub struct Entry<T> {
    key: usize,
    pub value: T,
}

impl<T> Entry<T> {
    // readonly access to the entry's key
    pub fn key(&self) -> usize {
        self.key
    }

    // returns the value, mainly for symmetry with key() since the value is public
    pub fn value(&self) -> &T {
        &self.value
    }

    // returns the value mutably, again mainly for symmetry with key() since the value is public
    // anyway.
    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> SparseSet<T> {
    // create a new sparseset with a given capacity
    pub fn with_capacity(size: usize) -> Self {
        let mut sparse = Vec::with_capacity(size);
        #[allow(clippy::uninit_vec)]
        unsafe {
            sparse.set_len(size)
        }

        SparseSet {
            dense: Vec::with_capacity(size),
            sparse,
        }
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }
    pub fn capacity(&self) -> usize {
        self.sparse.len()
    }

    // Clears the SparseSet in O(1) for simple T and O(n) if T implements drop
    pub fn clear(&mut self) {
        self.dense.clear();
    }

    fn dense_idx(&self, key: usize) -> Option<usize> {
        let dense_idx = self.sparse[key];
        if dense_idx < self.len() {
            let entry = &self.dense[dense_idx];
            if entry.key == key {
                return Some(dense_idx);
            }
        }
        None
    }

    // returns a reference to the value corresponding to the given key in O(1).
    pub fn get(&self, key: usize) -> Option<&T> {
        if let Some(dense_idx) = self.dense_idx(key) {
            Some(&self.dense[dense_idx].value)
        } else {
            None
        }
    }

    // get a mutable reference to the value corresponding to the given key in O(1).
    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        if let Some(dense_idx) = self.dense_idx(key) {
            Some(&mut self.dense[dense_idx].value)
        } else {
            None
        }
    }

    // get two mutable references to two distinct values. Safe guard against the same value
    pub fn get_pair_mut(&mut self, key1: usize, key2: usize) -> Option<(&mut T, &mut T)> {
        if key1 == key2 {
            // I wanna panic here because I want to know if this happens and where.
            panic!("Can't access mutable references to the same key twice");
        }

        let idx1 = self.dense_idx(key1)?;
        let idx2 = self.dense_idx(key2)?;

        // SAFETY:
        //  - idx1 and idx2 are distinct (we returned above if equal)
        //  - both indexes are in-bounds
        //  - we take disjoint mutable borrows via raw pointers
        unsafe {
            let ptr = self.dense.as_mut_ptr();
            let val1 = &mut (*ptr.add(idx1)).value;
            let val2 = &mut (*ptr.add(idx2)).value;
            Some((val1, val2))
        }
    }

    // check if the given key is contained in the set in O(1).
    pub fn contains(&self, key: usize) -> bool {
        self.dense_idx(key).is_some()
    }

    // insert in the set a value for the given key in O(1).
    // returns true if the key was set
    // returns false if the key was already set
    // also: if the key was already set, the previous value is overridden.
    pub fn insert(&mut self, key: usize, value: T) -> bool {
        assert!(
            key < self.capacity(),
            "key ({}) must be under capacity ({})",
            key,
            self.capacity()
        );
        if let Some(stored_value) = self.get_mut(key) {
            *stored_value = value;
            return false;
        }
        let n = self.dense.len();
        self.dense.push(Entry { key, value });
        self.sparse[key] = n;
        true
    }

    // removes the given key in O(1).
    // returns the removed value or None if key not found.
    pub fn remove(&mut self, key: usize) -> Option<T> {
        if self.contains(key) {
            let dense_idx = self.sparse[key];
            let r = self.dense.swap_remove(dense_idx).value;
            if dense_idx < self.len() {
                let swapped_entry = &self.dense[dense_idx];
                self.sparse[swapped_entry.key] = dense_idx;
            }
            // not strictly necessary, just nice to
            // restrict any future contains(key) to one test.
            self.sparse[key] = self.capacity();
            Some(r)
        } else {
            None
        }
    }
}

// deref to a slice.
impl<T> Deref for SparseSet<T> {
    type Target = [Entry<T>];

    fn deref(&self) -> &Self::Target {
        &self.dense[..]
    }
}

// deref to a mutable slice.
impl<T> DerefMut for SparseSet<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.dense[..]
    }
}

/// Move into an iterator, consuming the SparseSet.
impl<T> IntoIterator for SparseSet<T> {
    type Item = Entry<T>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.dense.into_iter()
    }
}

/// An iterator over the elements of the SparseSet.
impl<'a, T> IntoIterator for &'a SparseSet<T> {
    type Item = &'a Entry<T>;
    type IntoIter = slice::Iter<'a, Entry<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over mutable elements of the SparseSet.
impl<'a, T> IntoIterator for &'a mut SparseSet<T> {
    type Item = &'a mut Entry<T>;
    type IntoIter = slice::IterMut<'a, Entry<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
