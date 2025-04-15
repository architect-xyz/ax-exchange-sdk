use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Sequence id for distinguishing runs of sequence numbers.
type SequenceId = u64;

/// Unique sequence id and number.
#[derive(
    Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub struct SequenceIdAndNumber {
    #[serde(rename = "sid")]
    #[schemars(title = "sequence_id")]
    pub sequence_id: SequenceId,
    #[serde(rename = "sn")]
    #[schemars(title = "sequence_number")]
    pub sequence_number: u64,
}

impl SequenceIdAndNumber {
    pub fn new(sequence_id: SequenceId, sequence_number: u64) -> Self {
        Self { sequence_id, sequence_number }
    }

    pub fn new_random() -> Self {
        Self::new(rand::random::<SequenceId>(), 0)
    }

    pub fn next(&self) -> Self {
        Self::new(self.sequence_id, self.sequence_number + 1)
    }

    pub fn is_next_in_sequence(&self, previous: &Self) -> bool {
        self.sequence_id == previous.sequence_id
            && self.sequence_number == previous.sequence_number + 1
    }

    pub fn advance(&mut self) {
        self.sequence_number += 1;
    }
}

impl PartialOrd for SequenceIdAndNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.sequence_id != other.sequence_id {
            // sequence numbers are not from the same sequence--incomparable
            None
        } else {
            Some(self.sequence_number.cmp(&other.sequence_number))
        }
    }
}

impl std::fmt::Display for SequenceIdAndNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.sequence_id, self.sequence_number)
    }
}
