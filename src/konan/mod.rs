use std::{cmp::Ordering, rc::Rc};

pub mod parser;
pub mod cli;

struct IntervalStats {
    valid_elements: usize,
    positions_in_vec: usize,
}

/// SPMA Linear Naive Implementation
/// [Paper](https://itshelenxu.github.io/files/papers/spma-alenex-23.pdf)
pub struct Konan<T: Ord> {
    data: Vec<Option<Rc<T>>>,
    leaf_heads: Vec<Rc<T>>,
    segment_size: usize,
    height: usize,
}

struct Leaf {
    start: usize,
    end: usize,
}

impl<T: Ord> Default for Konan<T> {
    fn default() -> Self {
        let a: [Option<Rc<T>>; 2] = Default::default();
        let data = Vec::from(a);
        Self {
            data,
            leaf_heads: Vec::new(),
            segment_size: 2,
            height: 0,
        }
    }
}

impl<T: Ord> Konan<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> KonanIter<T> {
        KonanIter {
            curr_index: 0,
            konan: self,
        }
    }

    #[inline]
    fn search_leaf_to_insert(&self, v: &T) -> Leaf {
        if self.leaf_heads.is_empty() {
            return Leaf { start: 0, end: 1 };
        }
        let mut low: i32 = 0;
        let mut high: i32 = self.leaf_heads.len() as i32 - 1;

        while low <= high {
            let mid = (low as usize + high as usize) / 2;
            match v.cmp(&self.leaf_heads[mid]) {
                Ordering::Less | Ordering::Equal => high = mid as i32 - 1,
                Ordering::Greater => low = mid as i32 + 1,
            }
        }
        // We always tend to return the leaf head that is greater than the value, because we
        // accumulate on left the smaller value on a segment
        if high < 0 {
            return Leaf {
                start: 0,
                end: self.segment_size - 1,
            };
        }
        let start = self.leaf_head_to_data(high as usize);
        Leaf {
            start,
            end: start + self.segment_size - 1,
        }
    }

    #[inline]
    fn search_leaf_to_remove(&self, v: &T) -> Leaf {
        if self.leaf_heads.is_empty() {
            return Leaf { start: 0, end: 1 };
        }
        let mut low: i32 = 0;
        let mut high: i32 = self.leaf_heads.len() as i32 - 1;

        while low <= high {
            let mid = (low as usize + high as usize) / 2;
            match v.cmp(&self.leaf_heads[mid]) {
                Ordering::Less => high = mid as i32 - 1,
                Ordering::Greater => low = mid as i32 + 1,
                Ordering::Equal => {
                    high = mid as i32;
                    break;
                }
            }
        }
        if high < 0 {
            return Leaf {
                start: 0,
                end: self.segment_size - 1,
            };
        }
        let start = self.leaf_head_to_data(high as usize);
        Leaf {
            start,
            end: start + self.segment_size - 1,
        }
    }

    #[inline]
    fn leaf_head_to_data(&self, idx: usize) -> usize {
        idx * self.segment_size
    }

    #[inline]
    fn data_to_leaf_head(&self, leaf: &Leaf) -> usize {
        leaf.start / self.segment_size
    }

    #[inline]
    fn scan(&self, start: usize, end: usize) -> IntervalStats {
        let positions_in_vec = end - start + 1;
        let mut num_valid_elements = 0;

        for idx in start..=end {
            if self.data[idx].is_some() {
                num_valid_elements += 1;
            }
        }

        IntervalStats {
            valid_elements: num_valid_elements,
            positions_in_vec,
        }
    }

    #[inline]
    fn get_leaf_head(&self, leaf: &Leaf) -> &Option<Rc<T>> {
        let mut idx = leaf.start;
        while idx < leaf.end && self.data[idx].is_none() {
            idx += 1;
        }
        &self.data[idx]
    }

    #[inline]
    fn insert_on_leaf(&mut self, v: T, leaf: &Leaf, position: usize) {
        let head = self.get_leaf_head(leaf).clone();

        let value = Rc::new(v);
        self.insert_element_at(value.clone(), position, leaf);

        // There is no head in leaf, AKA empty leaf
        if head.is_none() {
            self.leaf_heads.push(value);
            return;
        }

        // The value is smaller than head
        if head.is_some_and(|v| value < v) {
            let leaf_head_position = self.data_to_leaf_head(leaf);
            self.leaf_heads[leaf_head_position] = value;
        }
    }

    #[inline]
    fn insert_element_at(&mut self, v: Rc<T>, idx: usize, leaf: &Leaf) {
        if self.data[idx].is_none() {
            self.data[idx] = Some(v);
            return;
        }

        // Smaller than head
        if idx == leaf.start {
            let mut none_idx = leaf.start;
            while none_idx < leaf.end && self.data[none_idx].is_some() {
                none_idx += 1;
            }
            self.data[leaf.start..=none_idx].rotate_right(1);
            self.data[idx] = Some(v);
            return;
        }

        let mut right_none_idx = idx;
        while right_none_idx < leaf.end && self.data[right_none_idx].is_some() {
            right_none_idx += 1;
        }

        if self.data[right_none_idx].is_none() {
            self.data[idx..=right_none_idx].rotate_right(1);
            self.data[idx] = Some(v);
            return;
        }

        // Here we don't have any space left on the right side to push
        // So we must push to the left
        let mut left_none_idx = idx;
        while left_none_idx > leaf.start && self.data[left_none_idx].is_some() {
            left_none_idx -= 1;
        }

        self.data[left_none_idx..idx].rotate_left(1);

        self.data[idx - 1] = Some(v.clone());
        // We have to update the leaf head if the value is smaller than the head
        if *v > **self.data[idx].as_ref().unwrap() {
            self.data.swap(idx, idx - 1);
        }
    }

    #[inline]
    fn find_element_position_in_leaf(&self, v: &T, leaf: &Leaf) -> usize {
        let mut pos = leaf.start;
        for i in leaf.start..=leaf.end {
            let current_value = &self.data[i];
            if current_value.is_none() {
                continue;
            }

            let existing_value = current_value.as_ref().unwrap();

            if v < existing_value {
                break;
            }

            if v > existing_value {
                if i == leaf.end {
                    pos = i;
                } else {
                    pos = i + 1;
                }
            }
        }

        pos
    }

    #[inline]
    fn find_position_to_remove(&self, v: &T, leaf: &Leaf) -> Option<usize> {
        let mut pos = None;
        for i in leaf.start..leaf.end {
            let current_value = &self.data[i];
            if current_value.is_none() {
                continue;
            }
            let existing_value = current_value.as_ref().unwrap();
            if *v == **existing_value {
                pos = Some(i);
                break;
            }
        }
        pos
    }

    fn remove_element_at(&mut self, position: usize, leaf: &Leaf) {
        let value = self.data[position].clone();
        assert!(value.is_some(), "expected value to be some");

        self.data[position] = None;

        if position != leaf.start {
            return;
        }

        let mut idx = leaf.start;
        while idx < leaf.end && self.data[idx].is_none() {
            idx += 1;
        }
        self.data.swap(position, idx);
    }

    #[inline]
    fn is_right_density(&self, depth: usize, density: f64) -> bool {
        let depth_over_height: f64 = match self.height {
            0 => 0.0,
            _ => (depth / self.height) as f64,
        };

        0.5 - (0.25 * depth_over_height) <= density && 0.75 + (0.25 * depth_over_height) >= density
    }

    #[inline]
    fn expand(&mut self) {
        let new_len = self.data.len() << 1;
        self.data.resize(new_len, None);

        // We only increase segment size to match leaf head address space
        if (self.data.len() as f64).log2() < (self.leaf_heads.len() as f64 * 2.0) {
            self.segment_size <<= 1;
        }
        // If not, we increase the height of the tree and duplicates leaf head
        else {
            self.height += 1;
        }
    }

    #[inline]
    fn halving(&mut self) {
        if self.data.len() == 2 {
            return;
        }

        let new_len = self.data.len() >> 1;
        let new_data: Vec<Option<Rc<T>>> =
            self.data.iter().filter(|x| x.is_some()).cloned().collect();

        self.data = new_data;
        self.data.resize(new_len, None);

        // We only decrease segment size to match leaf head address space
        if (self.data.len() as f64).log2() < (self.leaf_heads.len() as f64 * 2.0) && self.height > 0
        {
            self.height -= 1;
        }
        // If not, we decrease the segment size
        else {
            self.segment_size >>= 1;
        }
    }

    /// Rebalance algorithm
    /// Prerequisite:
    /// - Data must be withing right density
    /// - Segment size must be already set
    /// - Leaf heads must be duplicated or halved
    #[inline]
    fn rebalance(&mut self, start: usize, end: usize) {
        let interval_stats = self.scan(start, end);
        let amount_of_segments = usize::div_ceil(end - start + 1, self.segment_size);
        let mut maximum_number_of_elements_per_segment =
            interval_stats.valid_elements.div_ceil(amount_of_segments);
        let minimum_number_of_elements_per_segment =
            interval_stats.valid_elements / amount_of_segments;

        let copy_vec: Vec<Option<Rc<T>>> = self.data[start..=end].to_vec();
        let copy_vec: Vec<&Option<Rc<T>>> = copy_vec.iter().filter(|v| v.is_some()).collect();

        self.data[start..=end].iter_mut().for_each(|v| *v = None);

        let will_have_empty_segments = maximum_number_of_elements_per_segment
            * (amount_of_segments - 1)
            >= interval_stats.valid_elements;

        if will_have_empty_segments {
            maximum_number_of_elements_per_segment = minimum_number_of_elements_per_segment;
        }

        let mut copy_vec_iter = copy_vec.iter();
        for segment_start in (start..=end).step_by(self.segment_size) {
            for idx in 0..maximum_number_of_elements_per_segment {
                let element = match copy_vec_iter.next() {
                    Some(v) => (*v).clone(),
                    None => None,
                };
                let data_idx = idx + segment_start;
                self.data[data_idx] = element;
            }
        }

        if will_have_empty_segments {
            let last_segment_start =
                (end - self.segment_size) + maximum_number_of_elements_per_segment + 1;
            for (idx, element) in copy_vec_iter.enumerate() {
                self.data[last_segment_start + idx] = (*element).clone();
            }
        }

        self.update_new_leaf_heads();
    }

    #[inline]
    fn update_new_leaf_heads(&mut self) {
        let mut new_leaf_heads = Vec::new();
        for i in (0..self.data.len()).step_by(self.segment_size) {
            let leaf_head = match self.data[i].clone() {
                Some(v) => v,
                None => panic!("Should never have none after rebalascing"),
            };
            new_leaf_heads.push(leaf_head);
        }
        self.leaf_heads = new_leaf_heads;
    }

    #[inline]
    fn is_node_right_child(&self, leaf: &Leaf) -> bool {
        (leaf.start / (leaf.end - leaf.start + 1)) % 2 != 0
    }

    pub fn insert(&mut self, v: T) {
        let mut leaf = self.search_leaf_to_insert(&v);
        let position_to_insert = self.find_element_position_in_leaf(&v, &leaf);

        if self.data[position_to_insert].is_none() {
            self.insert_on_leaf(v, &leaf, position_to_insert);
            return;
        }

        let mut interval_stats = self.scan(leaf.start, leaf.end);
        let mut depth = self.height;
        let mut density =
            (interval_stats.valid_elements + 1) as f64 / interval_stats.positions_in_vec as f64;

        while depth > 0 && !self.is_right_density(depth, density) {
            depth -= 1;
            if self.is_node_right_child(&leaf) {
                leaf.start -= leaf.end - leaf.start + 1;
                interval_stats = self.scan(leaf.start, leaf.end);
                density = (interval_stats.valid_elements + 1) as f64
                    / interval_stats.positions_in_vec as f64;
            } else {
                leaf.end += leaf.end - leaf.start + 1;
                interval_stats = self.scan(leaf.start, leaf.end);
                density = (interval_stats.valid_elements + 1) as f64
                    / interval_stats.positions_in_vec as f64;
            }
        }

        if depth == 0 && !self.is_right_density(depth, density) {
            self.expand();
            leaf.end = self.data.len() - 1;
        }

        let position = self.find_element_position_in_leaf(&v, &leaf);
        self.insert_on_leaf(v, &leaf, position);
        self.rebalance(leaf.start, leaf.end);
    }

    pub fn remove(&mut self, v: &T) {
        let mut leaf = self.search_leaf_to_remove(v);

        let position_to_remove = self.find_position_to_remove(v, &leaf);
        if position_to_remove.is_none() {
            return;
        }

        self.remove_element_at(position_to_remove.unwrap(), &leaf);

        let mut interval_stats = self.scan(leaf.start, leaf.end);
        let mut depth = self.height;
        let mut density =
            interval_stats.valid_elements as f64 / interval_stats.positions_in_vec as f64;

        if self.is_right_density(depth, density) {
            self.update_new_leaf_heads();
            return;
        }

        while depth > 0 && !self.is_right_density(depth, density) {
            depth -= 1;
            if self.is_node_right_child(&leaf) {
                leaf.start -= leaf.end - leaf.start + 1;
                interval_stats = self.scan(leaf.start, leaf.end);
                density =
                    interval_stats.valid_elements as f64 / interval_stats.positions_in_vec as f64;
            } else {
                leaf.end += leaf.end - leaf.start + 1;
                interval_stats = self.scan(leaf.start, leaf.end);
                density =
                    interval_stats.valid_elements as f64 / interval_stats.positions_in_vec as f64;
            }
        }

        if depth == 0 && !self.is_right_density(depth, density) {
            self.halving();
            leaf.end = self.data.len() - 1;
        }

        // We have to check if we have to rebalance the tree because we have to remove the last element
        if self.data.len() == 2 {
            self.leaf_heads.clear();
            return;
        }

        self.rebalance(leaf.start, leaf.end);
    }

    pub fn successor(&mut self, v: &T) -> Option<&T> {
        if self.leaf_heads.is_empty() {
            return None;
        }

        let mut low: i32 = 0;
        let mut high: i32 = self.leaf_heads.len() as i32 - 1;

        while low <= high {
            let mid = (low as usize + high as usize) / 2;
            match v.cmp(&self.leaf_heads[mid]) {
                Ordering::Less => high = mid as i32 - 1,
                Ordering::Greater | Ordering::Equal => low = mid as i32 + 1,
            }
        }

        if low == self.leaf_heads.len() as i32 {
            low = self.leaf_heads.len() as i32 - 1;
        }

        let real_low = self.leaf_head_to_data(low as usize);
        let successor = self.data[real_low..self.data.len()]
            .iter()
            .find(|x| x.is_some() && x.as_deref() > Some(v));

        match successor {
            Some(v) => v.as_deref(),
            None => None,
        }
    }
}

pub struct KonanIter<'a, T: Ord> {
    konan: &'a Konan<T>,
    curr_index: usize,
}

impl<'a, T: Ord> Iterator for KonanIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut current: Option<Self::Item> = None;
        while self.curr_index < self.konan.data.len() && current.is_none() {
            current = self.konan.data[self.curr_index].as_deref();
            self.curr_index += 1;
        }

        current
    }
}

#[cfg(test)]
mod konan_test {
    use crate::konan::Konan;
    use pretty_assertions::{assert_eq, assert_ne};
    use rand::prelude::*;

    #[test]
    fn increasing_insertion() {
        let mut konan: Konan<usize> = Konan::new();
        let lower = 0;
        let upper = 10;
        for i in lower..upper {
            konan.insert(i);
        }

        for (expected_value, &actual_value) in konan.iter().enumerate() {
            assert_eq!(expected_value, actual_value)
        }
    }

    #[test]
    fn decreasing_insertion() {
        let mut konan: Konan<i32> = Konan::new();
        let lower = 0;
        let upper = 1000;
        for i in (lower..upper).rev() {
            konan.insert(i);
        }

        let mut iter = konan.iter();

        for expected_value in lower..upper {
            let actual_value = iter.next();
            assert_eq!(Some(&expected_value), actual_value);
        }
    }

    #[test]
    fn random_insertion() {
        let mut konan: Konan<i32> = Konan::new();

        let mut rng = rand::thread_rng();
        let mut nums: Vec<i32> = (1..1000).collect();
        nums.shuffle(&mut rng);

        for &item in nums.iter() {
            konan.insert(item);
        }

        let expected_nums: Vec<i32> = (1..1000).collect();

        let mut actual_nums: Vec<i32> = Vec::with_capacity(nums.len());
        for &item in konan.iter() {
            actual_nums.push(item);
        }
        assert_eq!(expected_nums, actual_nums);
    }

    #[test]
    fn no_successor() {
        let mut konan: Konan<i32> = Konan::new();
        let upper = 10;

        for i in 0..upper {
            konan.insert(i);
        }

        let expected_successor = None;
        assert_eq!(expected_successor, konan.successor(&10))
    }

    #[test]
    fn successor_on_empty_konan() {
        let mut konan: Konan<i32> = Konan::new();

        let expected_successor = None;
        assert_eq!(expected_successor, konan.successor(&10));
    }

    #[test]
    fn successor() {
        let mut konan: Konan<i32> = Konan::new();
        let upper = 10;

        for i in 0..upper {
            konan.insert(i);
        }

        let expected_successor = Some(&9);
        assert_eq!(expected_successor, konan.successor(&8));
    }

    #[test]
    fn remove_number_that_dont_exist() {
        let mut konan: Konan<i32> = Konan::new();

        for i in 0..10 {
            konan.insert(i);
        }

        konan.remove(&10);

        let expected: Vec<i32> = (0..10).collect();
        let actual: Vec<i32> = konan.iter().copied().collect();
        assert_eq!(expected, actual);
    }

    #[test]
    fn remove_number_that_exists() {
        let mut konan: Konan<i32> = Konan::new();

        for i in 0..10 {
            konan.insert(i);
        }

        konan.remove(&5);

        let expected: Vec<i32> = (0..10).filter(|&v| v != 5).collect();
        let actual: Vec<i32> = konan.iter().copied().collect();
        assert_eq!(expected, actual);
    }

    #[test]
    fn remove_all_items() {
        let mut konan: Konan<i32> = Konan::new();

        for i in 0..10 {
            konan.insert(i);
        }

        for i in 0..10 {
            konan.remove(&i);
        }

        let expected: Vec<i32> = Vec::new();
        let actual: Vec<i32> = konan.iter().copied().collect();
        assert_eq!(expected, actual);
    }

    #[test]
    fn remove_duplicated_item_should_remove_just_one() {
        let mut konan: Konan<i32> = Konan::new();

        for i in 0..4 {
            konan.insert(i);
            konan.insert(i);
        }

        konan.remove(&2);

        let expected = vec![0, 0, 1, 1, 2, 3, 3];
        let actual: Vec<i32> = konan.iter().copied().collect();
        assert_eq!(expected, actual);
    }

    #[test]
    fn insertions_and_deletions() {
        let mut konan: Konan<i32> = Konan::new();

        for i in 0..10 {
            konan.insert(i);
        }

        konan.remove(&5);

        for i in 10..20 {
            konan.insert(i);
        }

        konan.remove(&15);

        let expected: Vec<i32> = (0..10)
            .chain(10..20)
            .filter(|&v| v != 5 && v != 15)
            .collect();
        let actual: Vec<i32> = konan.iter().copied().collect();
        assert_eq!(expected, actual);
    }
}
