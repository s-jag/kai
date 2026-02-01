/// Transposition Table implementation
use crate::moves::Move;

/// Bound type for TT entries
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum Bound {
    #[default]
    None = 0,
    Upper = 1, // Alpha (fail-low) - score is at most this
    Lower = 2, // Beta (fail-high) - score is at least this
    Exact = 3, // PV node - exact score
}

/// Transposition table entry (16 bytes for cache efficiency)
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct TTEntry {
    /// Upper 32 bits of hash key for verification
    pub key: u32,
    /// Best move found
    pub best_move: Move,
    /// Evaluation score
    pub score: i16,
    /// Search depth
    pub depth: i8,
    /// Bound type
    pub bound: Bound,
    /// Age counter for replacement
    pub age: u8,
    /// Padding for alignment
    _padding: [u8; 3],
}

impl TTEntry {
    /// Score constants
    pub const MATE_SCORE: i16 = 30000;
    pub const MAX_PLY: i16 = 128;

    /// Adjust score for mate distance when storing
    pub fn score_to_tt(score: i16, ply: i32) -> i16 {
        if score >= Self::MATE_SCORE - Self::MAX_PLY {
            score + ply as i16
        } else if score <= -Self::MATE_SCORE + Self::MAX_PLY {
            score - ply as i16
        } else {
            score
        }
    }

    /// Adjust score for mate distance when retrieving
    pub fn score_from_tt(score: i16, ply: i32) -> i16 {
        if score >= Self::MATE_SCORE - Self::MAX_PLY {
            score - ply as i16
        } else if score <= -Self::MATE_SCORE + Self::MAX_PLY {
            score + ply as i16
        } else {
            score
        }
    }

    /// Check if this entry is valid for the given hash
    #[inline(always)]
    pub fn is_valid(&self, hash: u64) -> bool {
        self.key == (hash >> 32) as u32
    }

    /// Check if this entry's depth is sufficient
    #[inline(always)]
    pub fn depth_ok(&self, depth: i32) -> bool {
        self.depth >= depth as i8
    }

    /// Get the adjusted score
    #[inline(always)]
    pub fn adjusted_score(&self, ply: i32) -> i16 {
        Self::score_from_tt(self.score, ply)
    }
}

/// Transposition table
pub struct TranspositionTable {
    /// Table entries
    table: Vec<TTEntry>,
    /// Mask for indexing (size - 1)
    mask: usize,
    /// Current age
    age: u8,
    /// Number of entries
    num_entries: usize,
}

impl TranspositionTable {
    /// Create a new transposition table with the given size in MB
    pub fn new(size_mb: usize) -> Self {
        let size_bytes = size_mb * 1024 * 1024;
        let entry_size = std::mem::size_of::<TTEntry>();
        let num_entries = (size_bytes / entry_size).next_power_of_two();

        TranspositionTable {
            table: vec![TTEntry::default(); num_entries],
            mask: num_entries - 1,
            age: 0,
            num_entries,
        }
    }

    /// Resize the table to the given size in MB
    pub fn resize(&mut self, size_mb: usize) {
        let size_bytes = size_mb * 1024 * 1024;
        let entry_size = std::mem::size_of::<TTEntry>();
        let num_entries = (size_bytes / entry_size).next_power_of_two();

        if num_entries != self.num_entries {
            self.table = vec![TTEntry::default(); num_entries];
            self.mask = num_entries - 1;
            self.num_entries = num_entries;
            self.age = 0;
        }
    }

    /// Get the index for a hash
    #[inline(always)]
    fn index(&self, hash: u64) -> usize {
        (hash as usize) & self.mask
    }

    /// Probe the table for an entry
    #[inline(always)]
    pub fn probe(&self, hash: u64) -> Option<&TTEntry> {
        let entry = &self.table[self.index(hash)];
        if entry.is_valid(hash) {
            Some(entry)
        } else {
            None
        }
    }

    /// Store an entry in the table
    pub fn store(
        &mut self,
        hash: u64,
        depth: i32,
        score: i16,
        bound: Bound,
        best_move: Move,
        ply: i32,
    ) {
        let idx = self.index(hash);
        let existing = &self.table[idx];
        let key = (hash >> 32) as u32;

        // Replacement policy:
        // - Always replace if different position
        // - Replace if same position and deeper search
        // - Replace if same position and older entry
        // - Replace if exact bound (PV nodes are valuable)
        let should_replace = existing.key != key
            || existing.age != self.age
            || depth >= existing.depth as i32
            || bound == Bound::Exact;

        if should_replace {
            self.table[idx] = TTEntry {
                key,
                best_move,
                score: TTEntry::score_to_tt(score, ply),
                depth: depth as i8,
                bound,
                age: self.age,
                _padding: [0; 3],
            };
        } else if !best_move.is_null() && existing.best_move.is_null() {
            // Always update move if we have one and existing doesn't
            self.table[idx].best_move = best_move;
        }
    }

    /// Prefetch the entry for a hash (for better cache performance)
    #[inline(always)]
    pub fn prefetch(&self, hash: u64) {
        let idx = self.index(hash);
        let ptr = &self.table[idx] as *const TTEntry;
        #[cfg(target_arch = "x86_64")]
        unsafe {
            std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
        }
        #[cfg(not(target_arch = "x86_64"))]
        let _ = ptr; // Avoid unused warning
    }

    /// Clear the table
    pub fn clear(&mut self) {
        self.table.fill(TTEntry::default());
        self.age = 0;
    }

    /// Increment age for new search
    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    /// Get occupancy percentage (for UCI info)
    pub fn hashfull(&self) -> usize {
        let sample_size = 1000.min(self.num_entries);
        let used = self.table[..sample_size]
            .iter()
            .filter(|e| e.bound != Bound::None && e.age == self.age)
            .count();
        (used * 1000) / sample_size
    }

    /// Get the size in MB
    pub fn size_mb(&self) -> usize {
        (self.num_entries * std::mem::size_of::<TTEntry>()) / (1024 * 1024)
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(64) // 64 MB default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Square;

    #[test]
    fn test_tt_entry_size() {
        // Ensure entry is 16 bytes for cache efficiency
        assert_eq!(std::mem::size_of::<TTEntry>(), 16);
    }

    #[test]
    fn test_tt_store_and_probe() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0x123456789ABCDEF0u64;
        let mv = Move::quiet(Square::E2, Square::E4);

        tt.store(hash, 5, 100, Bound::Exact, mv, 0);

        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.depth, 5);
        assert_eq!(entry.score, 100);
        assert_eq!(entry.bound, Bound::Exact);
        assert_eq!(entry.best_move, mv);
    }

    #[test]
    fn test_tt_miss() {
        let tt = TranspositionTable::new(1);
        let hash = 0x123456789ABCDEF0u64;

        assert!(tt.probe(hash).is_none());
    }

    #[test]
    fn test_mate_score_adjustment() {
        let mate_in_3 = TTEntry::MATE_SCORE - 6; // Mate in 3 from search
        let stored = TTEntry::score_to_tt(mate_in_3, 2); // At ply 2
        let retrieved = TTEntry::score_from_tt(stored, 2);
        assert_eq!(retrieved, mate_in_3);

        // At different ply, should be different
        let at_ply_0 = TTEntry::score_from_tt(stored, 0);
        assert_ne!(at_ply_0, mate_in_3);
    }

    #[test]
    fn test_tt_replacement() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0x123456789ABCDEF0u64;
        let mv1 = Move::quiet(Square::E2, Square::E4);
        let mv2 = Move::quiet(Square::D2, Square::D4);

        // Store shallow entry
        tt.store(hash, 3, 50, Bound::Lower, mv1, 0);

        // Deeper entry should replace
        tt.store(hash, 5, 100, Bound::Exact, mv2, 0);

        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.depth, 5);
        assert_eq!(entry.best_move, mv2);
    }

    #[test]
    fn test_tt_new_search() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0x123456789ABCDEF0u64;
        let mv = Move::quiet(Square::E2, Square::E4);

        tt.store(hash, 5, 100, Bound::Exact, mv, 0);

        // New search should increment age
        tt.new_search();

        // Entry still there
        let entry = tt.probe(hash).unwrap();
        assert_eq!(entry.depth, 5);

        // But hashfull should be 0 (old entries don't count)
        assert_eq!(tt.hashfull(), 0);
    }
}
