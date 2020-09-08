use std::clone::Clone;

/// Used to index packets that have been sent & received
pub type SequenceNumber = u16;

/// Collection to store data of any kind.
#[derive(Debug)]
pub struct SequenceBuffer<T: Clone> {
    sequence_num: SequenceNumber,
    entry_sequences: Box<[Option<SequenceNumber>]>,
    entries: Box<[Option<T>]>,
}

impl<T: Clone> SequenceBuffer<T> {
    /// Creates a SequenceBuffer with a desired capacity.
    pub fn with_capacity(size: u16) -> Self {
        Self {
            sequence_num: 0,
            entry_sequences: vec![None; size as usize].into_boxed_slice(),
            entries: vec![None; size as usize].into_boxed_slice(),
        }
    }

    /// Returns the most recently stored sequence number.
    pub fn sequence_num(&self) -> SequenceNumber {
        self.sequence_num
    }

    /// Returns a mutable reference to the entry with the given sequence number.
    pub fn get_mut(&mut self, sequence_num: SequenceNumber) -> Option<&mut T> {
        if self.exists(sequence_num) {
            let index = self.index(sequence_num);
            return self.entries[index].as_mut();
        }
        None
    }

    /// Returns a reference to the entry with the given sequence number.
    pub fn get(&self, sequence_num: SequenceNumber) -> Option<&T> {
        if self.exists(sequence_num) {
            let index = self.index(sequence_num);
            return self.entries[index].as_ref();
        }
        None
    }

    /// Inserts the entry data into the sequence buffer. If the requested
    /// sequence number is "too old", the entry will not be inserted and will
    /// return false
    pub fn insert(&mut self, sequence_num: SequenceNumber, entry: T) -> bool {
        // sequence number is too old to insert into the buffer
        if sequence_less_than(
            sequence_num,
            self.sequence_num
                .wrapping_sub(self.entry_sequences.len() as u16),
        ) {
            return false;
        }

        self.advance_sequence(sequence_num);

        let index = self.index(sequence_num);
        self.entry_sequences[index] = Some(sequence_num);
        self.entries[index] = Some(entry);

        return true;
    }

    /// Returns whether or not we have previously inserted an entry for the
    /// given sequence number.
    pub fn exists(&self, sequence_num: SequenceNumber) -> bool {
        let index = self.index(sequence_num);
        if let Some(s) = self.entry_sequences[index] {
            return s == sequence_num;
        }
        false
    }

    /// Removes an entry from the sequence buffer
    pub fn remove(&mut self, sequence_num: SequenceNumber) -> Option<T> {
        if self.exists(sequence_num) {
            let index = self.index(sequence_num);
            let value = std::mem::replace(&mut self.entries[index], None);
            self.entry_sequences[index] = None;
            return value;
        }
        None
    }

    // Advances the sequence number while removing older entries.
    fn advance_sequence(&mut self, sequence_num: SequenceNumber) {
        if sequence_greater_than(sequence_num.wrapping_add(1), self.sequence_num) {
            self.remove_entries(u32::from(sequence_num));
            self.sequence_num = sequence_num.wrapping_add(1);
        }
    }

    fn remove_entries(&mut self, mut finish_sequence: u32) {
        let start_sequence = u32::from(self.sequence_num);
        if finish_sequence < start_sequence {
            finish_sequence += 65536;
        }

        if finish_sequence - start_sequence < self.entry_sequences.len() as u32 {
            for sequence in start_sequence..=finish_sequence {
                self.remove(sequence as u16);
            }
        } else {
            for index in 0..self.entry_sequences.len() {
                self.entries[index] = None;
                self.entry_sequences[index] = None;
            }
        }
    }

    // Generates an index for use in `entry_sequences` and `entries`.
    fn index(&self, sequence: SequenceNumber) -> usize {
        sequence as usize % self.entry_sequences.len()
    }

    /// Gets the oldest stored sequence number
    pub fn oldest(&self) -> u16 {
        return self
            .sequence_num
            .wrapping_sub(self.entry_sequences.len() as u16);
    }

    /// Clear sequence buffer completely
    pub fn clear(&mut self) {
        let size = self.entry_sequences.len();
        self.sequence_num = 0;
        self.entry_sequences = vec![None; size].into_boxed_slice();
        self.entries = vec![None; size].into_boxed_slice();
    }

    /// Remove entries up until a specific sequence number
    pub fn remove_until(&mut self, finish_sequence: u16) {
        let oldest = self.oldest();
        for seq in oldest..finish_sequence {
            self.remove(seq);
        }
    }

    /// Get an iterator into the sequence
    pub fn iter(&self) -> SequenceIterator<T> {
        return SequenceIterator::new(self.oldest(), self.entry_sequences.len(), self);
    }
}

/// Iterator for a Sequence
pub struct SequenceIterator<'s, T>
where
    T: 's + Clone,
{
    index: u16,
    count: usize,
    buffer: &'s SequenceBuffer<T>,
}

impl<'s, T: Clone> SequenceIterator<'s, T> {
    /// Create a new iterator for a sequence
    pub fn new(
        start: u16,
        count: usize,
        seq_buf: &'s SequenceBuffer<T>,
    ) -> SequenceIterator<'s, T> {
        SequenceIterator::<T> {
            index: start,
            count,
            buffer: seq_buf,
        }
    }
}

impl<'s, T: Clone> Iterator for SequenceIterator<'s, T> {
    type Item = &'s T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.count < 0 {
                return None;
            }
            let current_item = self.buffer.get(self.index);
            self.index = self.index.wrapping_add(1);
            self.count -= 1;
            if current_item.is_some() {
                return current_item;
            }
        }
    }
}

pub fn sequence_greater_than(s1: u16, s2: u16) -> bool {
    ((s1 > s2) && (s1 - s2 <= 32768)) || ((s1 < s2) && (s2 - s1 > 32768))
}

pub fn sequence_less_than(s1: u16, s2: u16) -> bool {
    sequence_greater_than(s2, s1)
}
