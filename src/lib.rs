#![feature(test)]

#[cfg(test)]
extern crate test;

#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::marker::PhantomData;

/// A fixed-size hash map which only remembers the minimum value set for a given hash. Satisfies the
/// property that
/// ```ignore
/// map.set(v1, k1);
/// map.set(v2, k2);
/// ```
/// is equivalent to both
/// ```ignore
/// map.set(v1, min(k1, k2));
/// ```
/// and
/// ```ignore
/// map.set(v2, min(k1, k2));
/// ```
/// if and only if `hash(v1) == hash(v2)`.
pub struct MinMap<K: Hash, V: Copy + Ord + Sized, H: BuildHasher, const SIZE: usize> {
    table: [V; SIZE],
    hash_builder: H,
    key_type: PhantomData<K>,
}

impl<K: Hash, V: Copy + Ord + Sized, const SIZE: usize> MinMap<K, V, RandomState, SIZE> {
    pub fn new(init: V) -> Self {
        Self {
            hash_builder: RandomState::new(),
            table: [init; SIZE],
            key_type: PhantomData,
        }
    }
}

impl<K: Hash, V: Copy + Ord + Sized, H: BuildHasher, const SIZE: usize> MinMap<K, V, H, SIZE> {
    pub fn with_hash_builder(hash_builder: H, init: V) -> Self {
        Self {
            hash_builder,
            table: [init; SIZE],
            key_type: PhantomData,
        }
    }

    pub fn set(&mut self, key: K, value: V) {
        let hash = self.hash(key);
        self.table[hash] = std::cmp::min(self.table[hash], value);
    }

    pub fn get(&self, key: K) -> V {
        self.table[self.hash(key)]
    }

    fn hash(&self, key: K) -> usize {
        let mut hasher = self.hash_builder.build_hasher();
        key.hash(&mut hasher);
        hasher.finish() as usize % SIZE
    }
}

impl<K: Hash, V: Copy + Ord + Sized, H: BuildHasher, const SIZE: usize> std::ops::Index<K>
    for MinMap<K, V, H, SIZE>
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        &self.table[self.hash(index)]
    }
}

impl<K: Hash, V: Copy + Ord + Sized, H: BuildHasher, const SIZE: usize> std::ops::IndexMut<K>
    for MinMap<K, V, H, SIZE>
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        &mut self.table[self.hash(index)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::min;

    #[quickcheck]
    fn single_set(init: usize, k: usize, v: usize) -> bool {
        let mut map = MinMap::<usize, usize, RandomState, 42>::new(init);

        map.set(k, v);

        map.get(k) == min(init, v)
    }

    #[quickcheck]
    fn double_set(init: usize, k1: usize, k2: usize, v1: usize, v2: usize) -> bool {
        let mut map = MinMap::<usize, usize, RandomState, 42>::new(init);

        map.set(k1, v1);
        map.set(k2, v2);

        if map.hash(k1) == map.hash(k2) {
            map.get(k1) == min(init, min(v1, v2))
        } else {
            map.get(k1) == min(init, v1)
        }
    }
}

#[cfg(test)]
mod benches {
    use super::*;
    use rand::{Rng, SeedableRng};
    use test::{black_box, Bencher};

    mod creation {
        use super::*;
        fn create<const SIZE: usize, const INIT: isize>(b: &mut Bencher) {
            b.iter(|| black_box(MinMap::<isize, isize, RandomState, SIZE>::new(INIT)))
        }

        #[bench]
        fn create_100_at_0(b: &mut Bencher) {
            create::<100, 0>(b)
        }

        #[bench]
        fn create_100_at_123456789(b: &mut Bencher) {
            create::<100, 123456789>(b)
        }

        #[bench]
        fn create_100_000_at_0(b: &mut Bencher) {
            create::<100_000, 0>(b)
        }

        #[bench]
        fn create_100_000_at_123456789(b: &mut Bencher) {
            create::<100_000, 123456789>(b)
        }
    }

    mod setting {
        use super::*;
        fn set<const N: usize, const SIZE: usize>(b: &mut Bencher) {
            let rng = rand::rngs::SmallRng::seed_from_u64(123456789);
            let keys = rng
                .sample_iter(rand::distributions::Uniform::new(isize::MIN, isize::MAX))
                .take(N)
                .collect::<Vec<_>>();

            let rng = rand::rngs::SmallRng::seed_from_u64(987654321);
            let vals = rng
                .sample_iter(rand::distributions::Uniform::new(isize::MIN, isize::MAX))
                .take(N)
                .collect::<Vec<_>>();

            b.iter(|| {
                let keys = black_box(keys.iter());
                let vals = black_box(vals.iter());
                let mut map = black_box(MinMap::<isize, isize, RandomState, SIZE>::new(0));

                for (key, val) in keys.zip(vals) {
                    map.set(*key, *val);
                }
            })
        }

        #[bench]
        fn set_10_000_in_100(b: &mut Bencher) {
            set::<10_000, 100>(b)
        }

        #[bench]
        fn set_10_000_in_100_000(b: &mut Bencher) {
            set::<10_000, 100_000>(b)
        }

        #[bench]
        fn set_100_000_in_100(b: &mut Bencher) {
            set::<100_000, 100>(b)
        }

        #[bench]
        fn set_100_000_in_100_000(b: &mut Bencher) {
            set::<100_000, 100_000>(b)
        }

        #[bench]
        fn set_1_000_000_in_100(b: &mut Bencher) {
            set::<1_000_000, 100>(b)
        }

        #[bench]
        fn set_1_000_000_in_100_000(b: &mut Bencher) {
            set::<1_000_000, 100_000>(b)
        }
    }

    mod getting {
        use super::*;

        fn get<const N: usize, const SIZE: usize>(b: &mut Bencher) {
            let rng = rand::rngs::SmallRng::seed_from_u64(123456789);
            let keys = rng
                .sample_iter(rand::distributions::Uniform::new(isize::MIN, isize::MAX))
                .take(N);

            let rng = rand::rngs::SmallRng::seed_from_u64(987654321);
            let vals = rng
                .sample_iter(rand::distributions::Uniform::new(isize::MIN, isize::MAX))
                .take(N);

            let map = {
                let mut map = black_box(MinMap::<isize, isize, RandomState, SIZE>::new(0));
                for (key, val) in keys.zip(vals) {
                    map.set(key, val);
                }
                map
            };

            let rng = rand::rngs::SmallRng::seed_from_u64(543216789);
            let keys = rng
                .sample_iter(rand::distributions::Uniform::new(isize::MIN, isize::MAX))
                .take(N)
                .collect::<Vec<_>>();

            b.iter(|| {
                for key in keys.iter() {
                    map.get(*key);
                }
            })
        }

        #[bench]
        fn get_100_from_100(b: &mut Bencher) {
            get::<100, 100>(b)
        }

        #[bench]
        fn get_100_from_100_000(b: &mut Bencher) {
            get::<100, 100_000>(b)
        }

        #[bench]
        fn get_100_000_from_100(b: &mut Bencher) {
            get::<100_000, 100>(b)
        }

        #[bench]
        fn get_100_000_from_100_000(b: &mut Bencher) {
            get::<100_000, 100_000>(b)
        }

        #[bench]
        fn get_1_000_000_from_100(b: &mut Bencher) {
            get::<1_000_000, 100>(b)
        }

        #[bench]
        fn get_1_000_000_from_100_000(b: &mut Bencher) {
            get::<1_000_000, 100_000>(b)
        }
    }
}
