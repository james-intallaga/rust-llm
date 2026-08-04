//! Batch memory management with arena allocation

use llama_cpp_sys::{
    llama_batch, llama_batch_free, llama_batch_init, llama_pos, llama_seq_id, llama_token,
};

/// Arena-style batch allocator
///
/// Instead of allocating/freeing individual batches, this arena
/// pre-allocates a batch of a fixed size and reuses it.
/// This eliminates fragmentation and provides O(1) reset.
pub struct BatchArena {
    /// The underlying llama_batch
    batch: llama_batch,
    /// Maximum number of tokens this batch can hold
    capacity: i32,
    /// Current number of tokens in the batch
    len: i32,
}

// Safety: BatchArena owns the batch and controls access
unsafe impl Send for BatchArena {}

impl BatchArena {
    /// Create a new batch arena with the specified capacity
    ///
    /// # Arguments
    ///
    /// * `n_tokens` - Maximum number of tokens the batch can hold
    /// * `n_seq_max` - Maximum number of sequences (usually 1)
    pub fn new(n_tokens: i32, n_seq_max: i32) -> Self {
        log::debug!(
            "Creating batch arena: n_tokens={}, n_seq_max={}",
            n_tokens,
            n_seq_max
        );

        let batch = unsafe { llama_batch_init(n_tokens, 0, n_seq_max) };

        Self {
            batch,
            capacity: n_tokens,
            len: 0,
        }
    }

    /// Reset the batch for reuse (O(1) operation)
    pub fn reset(&mut self) {
        self.len = 0;
        self.batch.n_tokens = 0;
    }

    /// Add a token to the batch
    ///
    /// # Returns
    ///
    /// `true` if the token was added, `false` if the batch is full
    pub fn add_token(
        &mut self,
        token: llama_token,
        pos: llama_pos,
        seq_id: llama_seq_id,
        logits: bool,
    ) -> bool {
        if self.len >= self.capacity {
            return false;
        }

        unsafe {
            let i = self.len as usize;
            *self.batch.token.add(i) = token;
            *self.batch.pos.add(i) = pos;
            *self.batch.n_seq_id.add(i) = 1;
            *(*self.batch.seq_id.add(i)) = seq_id;
            *self.batch.logits.add(i) = if logits { 1 } else { 0 };
        }

        self.len += 1;
        self.batch.n_tokens = self.len;

        true
    }

    /// Add multiple tokens to the batch
    ///
    /// # Returns
    ///
    /// Number of tokens actually added (may be less than requested if batch fills)
    pub fn add_tokens(
        &mut self,
        tokens: &[llama_token],
        start_pos: llama_pos,
        seq_id: llama_seq_id,
    ) -> i32 {
        let mut added = 0;
        for (i, &token) in tokens.iter().enumerate() {
            let pos = start_pos + i as i32;
            let is_last = i == tokens.len() - 1;
            if !self.add_token(token, pos, seq_id, is_last) {
                break;
            }
            added += 1;
        }
        added
    }

    /// Get the current number of tokens in the batch
    pub fn len(&self) -> i32 {
        self.len
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the maximum capacity
    pub fn capacity(&self) -> i32 {
        self.capacity
    }

    /// Get the remaining capacity
    pub fn remaining(&self) -> i32 {
        self.capacity - self.len
    }

    /// Get the underlying batch for decode operations
    ///
    /// # Safety
    ///
    /// The returned batch is valid only while this arena exists and hasn't been reset.
    pub fn as_batch(&self) -> llama_batch {
        self.batch
    }
}

impl Drop for BatchArena {
    fn drop(&mut self) {
        log::debug!("Freeing batch arena (capacity={})", self.capacity);
        unsafe {
            llama_batch_free(self.batch);
        }
        log::debug!("Batch arena freed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_arena() {
        let mut arena = BatchArena::new(128, 1);

        assert_eq!(arena.len(), 0);
        assert_eq!(arena.capacity(), 128);

        // Add some tokens
        assert!(arena.add_token(1, 0, 0, false));
        assert!(arena.add_token(2, 1, 0, false));
        assert!(arena.add_token(3, 2, 0, true));

        assert_eq!(arena.len(), 3);

        // Reset
        arena.reset();
        assert_eq!(arena.len(), 0);
    }
}
