use std::{collections::HashMap, hash::Hash};

pub trait HashMapGetPairMut<K: Eq + std::hash::Hash, V> {
    fn get_pair_mut(&mut self, k1: &K, k2: &K) -> Option<(&mut V, &mut V)>;
}

impl<K: Eq + Hash, V> HashMapGetPairMut<K, V> for HashMap<K, V> {
    fn get_pair_mut(&mut self, k1: &K, k2: &K) -> Option<(&mut V, &mut V)> {
        if k1 == k2 {
            return None;
        }

        let (v1_ptr, v2_ptr) = {
            let v1 = self.get_mut(k1)? as *mut V;
            let v2 = self.get_mut(k2)? as *mut V;
            (v1, v2)
        };

        // safety: we ensure v1 and v2 are different by checking k1 != k2
        unsafe { Some((&mut *v1_ptr, &mut *v2_ptr)) }
    }
}

pub trait HashMapGetPair<K: Eq + std::hash::Hash, V> {
    fn get_pair(&self, k1: &K, k2: &K) -> Option<(&V, &V)>;
}

impl<K: Eq + Hash, V> HashMapGetPair<K, V> for HashMap<K, V> {
    fn get_pair(&self, k1: &K, k2: &K) -> Option<(&V, &V)> {
        let v1 = self.get(k1)?;
        let v2 = self.get(k2)?;
        Some((v1, v2))
    }
}
