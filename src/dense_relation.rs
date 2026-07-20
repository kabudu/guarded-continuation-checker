//! Resource-bounded finite relations for proof-carrying interface composition.

use std::error::Error;
use std::fmt;

pub const DENSE_RELATION_VERSION: u32 = 1;
pub const MAX_DENSE_RELATION_STATES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenseRelation {
    states: usize,
    words: usize,
    rows: Vec<Vec<u64>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenseRelationError(pub String);

impl fmt::Display for DenseRelationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for DenseRelationError {}

fn reject(message: impl Into<String>) -> DenseRelationError {
    DenseRelationError(message.into())
}

impl DenseRelation {
    pub fn empty(states: usize) -> Result<Self, DenseRelationError> {
        if states == 0 || states > MAX_DENSE_RELATION_STATES {
            return Err(reject("dense relation state count is outside limit"));
        }
        let words = states.div_ceil(u64::BITS as usize);
        Ok(Self {
            states,
            words,
            rows: vec![vec![0; words]; states],
        })
    }

    pub fn identity(states: usize) -> Result<Self, DenseRelationError> {
        let mut relation = Self::empty(states)?;
        for state in 0..states {
            relation.insert(state, state)?;
        }
        Ok(relation)
    }

    pub fn states(&self) -> usize {
        self.states
    }

    pub fn row_words(&self) -> &[Vec<u64>] {
        &self.rows
    }

    pub fn insert(&mut self, source: usize, target: usize) -> Result<(), DenseRelationError> {
        if source >= self.states || target >= self.states {
            return Err(reject("dense relation edge is outside state space"));
        }
        self.rows[source][target / u64::BITS as usize] |= 1u64 << (target % u64::BITS as usize);
        Ok(())
    }

    pub fn contains(&self, source: usize, target: usize) -> Result<bool, DenseRelationError> {
        if source >= self.states || target >= self.states {
            return Err(reject("dense relation query is outside state space"));
        }
        Ok(
            self.rows[source][target / u64::BITS as usize]
                & (1u64 << (target % u64::BITS as usize))
                != 0,
        )
    }

    pub fn targets(&self, source: usize) -> Result<Vec<usize>, DenseRelationError> {
        if source >= self.states {
            return Err(reject("dense relation source is outside state space"));
        }
        let mut targets = Vec::new();
        for (word_index, word) in self.rows[source].iter().enumerate() {
            let mut remaining = *word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                remaining &= remaining - 1;
                let target = word_index * u64::BITS as usize + bit;
                if target < self.states {
                    targets.push(target);
                }
            }
        }
        Ok(targets)
    }

    pub fn compose(left: &Self, right: &Self) -> Result<Self, DenseRelationError> {
        if left.states != right.states {
            return Err(reject("dense relation state-space mismatch"));
        }
        let mut result = Self::empty(left.states)?;
        for source in 0..left.states {
            for middle in left.targets(source)? {
                for word in 0..result.words {
                    result.rows[source][word] |= right.rows[middle][word];
                }
            }
        }
        Ok(result)
    }

    pub fn power(base: &Self, mut exponent: u64) -> Result<Self, DenseRelationError> {
        let mut result = Self::identity(base.states)?;
        let mut factor = base.clone();
        while exponent != 0 {
            if exponent & 1 == 1 {
                result = Self::compose(&result, &factor)?;
            }
            exponent >>= 1;
            if exponent != 0 {
                factor = Self::compose(&factor, &factor)?;
            }
        }
        Ok(result)
    }

    pub fn pair_count(&self) -> usize {
        self.rows
            .iter()
            .flat_map(|row| row.iter())
            .map(|word| word.count_ones() as usize)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_and_power_preserve_exact_pairs() {
        let mut step = DenseRelation::empty(4).unwrap();
        step.insert(0, 1).unwrap();
        step.insert(1, 2).unwrap();
        step.insert(2, 3).unwrap();
        step.insert(3, 3).unwrap();

        let square = DenseRelation::compose(&step, &step).unwrap();
        assert_eq!(square.targets(0).unwrap(), [2]);
        assert_eq!(square.targets(1).unwrap(), [3]);
        assert_eq!(DenseRelation::power(&step, 2).unwrap(), square);
        assert_eq!(DenseRelation::power(&step, 0).unwrap().pair_count(), 4);
    }

    #[test]
    fn nondeterministic_rows_compose_without_losing_edges() {
        let mut left = DenseRelation::empty(3).unwrap();
        left.insert(0, 0).unwrap();
        left.insert(0, 1).unwrap();
        let mut right = DenseRelation::empty(3).unwrap();
        right.insert(0, 1).unwrap();
        right.insert(1, 2).unwrap();
        let composed = DenseRelation::compose(&left, &right).unwrap();
        assert_eq!(composed.targets(0).unwrap(), [1, 2]);
        assert_eq!(composed.pair_count(), 2);
    }

    #[test]
    fn resource_and_index_errors_fail_closed() {
        assert!(DenseRelation::empty(0).is_err());
        assert!(DenseRelation::empty(MAX_DENSE_RELATION_STATES + 1).is_err());
        let mut relation = DenseRelation::empty(2).unwrap();
        assert!(relation.insert(2, 0).is_err());
        assert!(relation.insert(0, 2).is_err());
        assert!(relation.contains(2, 0).is_err());
        assert!(relation.targets(2).is_err());
        assert!(DenseRelation::compose(&relation, &DenseRelation::empty(3).unwrap()).is_err());
    }
}
