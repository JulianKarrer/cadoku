use std::{
    fmt::Debug,
    ops::{Sub, SubAssign},
    u8,
};

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::constants::{PEERS, UNITS};

pub fn generate_subtractive(hints: usize) -> (Sudoku, [u8; 81]) {
    assert!(
        17 <= hints && hints <= 81,
        "Number of hints must be between 17 and 81"
    );
    // generate a random solution
    let mut sudoku = Sudoku::generate_random_solution();
    let solution = sudoku.grid.clone();
    // generate a random order of squares to remove hints from
    let mut order = get_random_square_permutation();
    let mut i = 0;
    loop {
        // if the right number of hints is left, return
        if sudoku.count_cues() <= hints {
            return (sudoku, solution);
        }
        // if there are no more squares to try, we are stuck:
        // reshuffle and try again
        if i >= 80 {
            order = get_random_square_permutation();
            sudoku.grid = solution;
            i = 0;
        }
        // remove the hint from a copy of the current sudoku
        // in case backtracking is required
        let mut new = sudoku.clone();
        new.grid[order[i]] = 0;
        // square order[i] has now been tried, remove it
        i += 1;
        if let Some(sol) = constrain(&new) {
            debug_assert!(sol.grid == solution);
            // if the solution is still unique and matches the
            // target solution, update the current sudoku
            sudoku = new;
        }
        // otherwise, the `new` sudoku will just be dropped
        // and we try again in the next iteration
    }
}

/// Generate a random sudoku with the given number of hints that can be solved
/// purely by repeatedly propagating the trivial constraints in `constrain` with
/// no additional search. This guarantees that a human does not have to guess at
/// any point to obtain the solution.
pub fn _generate_additive(hints: usize) -> (Sudoku, [u8; 81]) {
    assert!(
        17 <= hints && hints <= 81,
        "Number of hints must be between 17 and 81"
    );
    loop {
        let mut sets: Vec<(usize, u8, Set)> = (0..81usize)
            .map(|i| (i, _get_random_u8(u8::MAX), Set::full()))
            .collect();
        let mut grid = [Set::full(); 81];
        let mut res = Sudoku::empty();
        while
        // while the sudoku is not yet complete
        !sets.iter().all(|(_, _, s)|s.is_single()) && 
            // and there are more hints left to give
            res.grid.iter().filter(|n| (**n) != 0u8).count() < hints
        {
            // sort squares by number of possible values, descending
            // -> less constrained squares are assigned first
            //    so lower numbers of hints can be generated quickly
            sets.sort_unstable_by(|(_, ra, a), (_, rb, b)| match b.count().cmp(&a.count()) {
                // use the random values `ra`, `rb` as tiebreakers within
                // each bucket of equally unconstrained squares
                // -> random sudokus every time
                std::cmp::Ordering::Equal => rb.cmp(&ra),
                ord => ord,
            });
            // for the least constrained set of possible values:
            // - select a random value
            let rand_feasible_digit = sets[0].2.select_random();
            // - and enter it into the sudoku as a hint
            let square = sets[0].0;
            res.grid[square] = rand_feasible_digit.single_to_number().unwrap();
            // then update the sets of possible values in accordance with the new hint
            if !assign(&mut grid, square, rand_feasible_digit) {
                break;
            }
            // make sets and sudoku reflect the updated grid
            sets.iter_mut().for_each(|t| t.2 = grid[t.0]);
        }
        // check if result is solvable and if the number of hints is low enough
        if sets.iter().all(|(_, _, s)| s.is_single())
            && res.grid.iter().filter(|n| (**n) != 0u8).count() == hints
        {
            let mut solution = [0u8; 81];
            for (i, _, s) in sets {
                solution[i] = s.single_to_number().unwrap();
            }
            debug_assert!(res.grid.iter().enumerate().all(|(i, num)| if *num > 0 {
                *num == solution[i]
            } else {
                true
            }));
            debug_assert!(constrain(&res).unwrap().grid == solution);
            return (res, solution);
        }
    }
}

/// Attempt to propagate any constraints formed by the hints in the sudoku by
pub fn constrain(sudoku: &Sudoku) -> Option<Sudoku> {
    let mut grid = [Set::full(); 81];
    // assign all hints
    for (s, hint) in sudoku.grid.iter().enumerate() {
        if *hint != 0 {
            if !assign(&mut grid, s, Set::new(*hint)) {
                return None;
            }
        }
    }
    // check all solutions, copying them to the result
    let mut res = Sudoku::empty();
    for s in 0..81 {
        if let Some(v) = grid[s].single_to_number() {
            res.set(s, v);
        } else {
            return None;
        }
    }
    Some(res)
}

/// Fill square `s` of the `grid` with the single digit in the set `d`.
/// `d` MUST be a single digit!
/// This function is as described in Peter Norvig's blog post.
fn assign(grid: &mut [Set; 81], s: usize, d: Set) -> bool {
    grid[s] == d || grid[s].all_neq_predicate(d, |d2| eliminate(grid, s, d2))
}

/// Eliminate digit `d` from square `s` of the `grid`.
/// Recursively calls itself and `fill`, mutating the grrid in-place.
/// This function is as described in Peter Norvig's blog post.
fn eliminate(grid: &mut [Set; 81], s: usize, d: Set) -> bool {
    if grid[s].doesnt_contain(d) {
        // digit was not in set removed, do nothing
        return true;
    }
    let updated = grid[s] - d;
    if updated == EMPTY {
        // no digit left
        return false;
    }
    // update the grid
    grid[s] = updated;
    if updated.is_single() {
        // one digit left, this one belongs at s and can be eliminated from peers
        for peer_s in PEERS[s] {
            if !eliminate(grid, peer_s, updated) {
                // contradiction encountered in consequence of this elimination
                return false;
            }
        }
    }
    // see where else to place this digit in the same unit
    for unit in UNITS[s] {
        let mut feasible_iter = unit.iter().filter(|s| grid[**s].contains(d));
        if let Some(s_n) = feasible_iter.next() {
            if let None = feasible_iter.next() {
                // exactly one feasible neighbour, try to fill it
                if !assign(grid, *s_n, d) {
                    return false;
                }
            }
        } else {
            // no feasible neighbours
            return false;
        }
    }
    true
}

/// A sudoku, stored as a flat, row-major array of 81 bytes,
/// where each `u8` is a value 1-9 or zero for the empty field.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Sudoku {
    #[serde(with = "BigArray")]
    grid: [u8; 81],
}
impl Sudoku {
    /// Return an empty sudoku with no clues
    pub fn empty() -> Self {
        Sudoku { grid: [0; 81] }
    }
    /// Set the given square index (0 to 80 inclusive in row-major order) to the given value
    /// where `0u8` to `9u8` are valid.
    pub fn set(&mut self, square: usize, val: u8) {
        debug_assert!(square < 81);
        debug_assert!(val <= 9);
        self.grid[square] = val
    }
    /// Check if the sudoku is filled by testing if any square is zero, i.e. empty. 
    /// Does not check correctness of the solution.
    pub fn filled(&self) -> bool {
        for v in self.grid {
            if v == 0 {
                return false;
            }
        }
        true
    }
    /// Check whether the square at the given x and y index (column and row) is zero, i.e. empty.
    /// This is valid for `x<9` and `y<9` only.
    pub fn is_zero(&self, x: usize, y: usize) -> bool {
        debug_assert!(x < 9 && y < 9);
        self.grid[x + y * 9] == 0
    }
    /// Count the number of filled units, i.e. rows, columns or cells.
    pub fn count_filled_units(&self) -> usize {
        let mut count = 0;
        for ss in UNITS {
            for s in ss {
                if s.iter().all(|i| self.grid[*i] > 0) {
                    count += 1;
                }
            }
        }
        count
    }
    /// Count the number of cures or non-zero entries in the sudoku grid
    fn count_cues(&self) -> usize {
        self.grid.iter().filter(|n| **n > 0).count()
    }
    /// Generate a random, solved (filled) sudoku grid
    fn generate_random_solution() -> Self {
        let mut grid = [Set::full(); 81];

        while !grid.iter().all(|s| s.is_single()) {
            let square = get_random_usize(80);
            let rand_feasible_digit = grid[square].select_random();
            if !assign(&mut grid, square, rand_feasible_digit) {
                grid = [Set::full(); 81];
            };
        }

        let mut res = Sudoku::empty();
        for (square, set) in grid.iter().enumerate() {
            res.grid[square] = set.single_to_number().unwrap();
        }

        res
    }
}

/// A set of values from 1 to 9 with corresponding functions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Set {
    data: u16,
}
const VALS: [Set; 9] = [
    Set { data: 1 << 0 },
    Set { data: 1 << 1 },
    Set { data: 1 << 2 },
    Set { data: 1 << 3 },
    Set { data: 1 << 4 },
    Set { data: 1 << 5 },
    Set { data: 1 << 6 },
    Set { data: 1 << 7 },
    Set { data: 1 << 8 },
];
const EMPTY: Set = Set { data: 0 };

impl Set {
    /// returns the full set, representing all values 1-9
    fn full() -> Self {
        Self { data: 0b111111111 }
    }
    /// returns a new singleton set, where `val` may range from `1u8` to `9u8`
    fn new(val: u8) -> Self {
        debug_assert!(0 < val && val < 10);
        Self {
            data: 1 << (val - 1),
        }
    }
    /// checks if only a single value is contained in the set
    fn is_single(&self) -> bool {
        self.data != 0 && (self.data & (self.data - 1)) == 0
    }
    /// if only a single value is contained in the set, return it as a `u8`, otherwise returns `None`
    fn single_to_number(&self) -> Option<u8> {
        // check if a single bit is set
        if self.data != 0 && (self.data & (self.data - 1)) == 0 {
            Some(self.data.trailing_zeros() as u8 + 1)
        } else {
            None
        }
    }
    /// count the number of values in the set
    fn count(&self) -> u32 {
        self.data.count_ones()
    }
    /// selects a random value from the set, returning it as a singleton [`Set`], panicking if the set is empty
    fn select_random(&self) -> Self {
        debug_assert!(self.data != 0);
        *VALS
            .iter()
            .filter(|v| self.contains(**v))
            .nth(get_random_usize(self.count() as usize))
            .unwrap()
    }
    /// checks if the set does NOT contain a given other set, i.e. if `self` is NOT a subset of `rhs`
    fn doesnt_contain(&self, rhs: Set) -> bool {
        self.data & rhs.data == 0
    }
    /// checks if the set contains a given other set, i.e. if `self` is a subset of `rhs`
    fn contains(&self, rhs: Set) -> bool {
        self.data & rhs.data == rhs.data
    }
    /// Applies the predicate `p` to all values of the set which are not equal to `neq`.
    /// Returns whether or not all predicates were true.
    fn all_neq_predicate(self, neq: Set, mut f: impl FnMut(Set) -> bool) -> bool {
        for v in VALS {
            if self.contains(v) && (v != neq) {
                if !f(v) {
                    return false;
                };
            }
        }
        true
    }
}
impl SubAssign for Set {
    fn sub_assign(&mut self, rhs: Self) {
        self.data &= !rhs.data
    }
}
impl Sub for Set {
    type Output = Set;
    fn sub(self, rhs: Self) -> Self::Output {
        Set {
            data: self.data & !rhs.data,
        }
    }
}

fn _get_random_u8(upper_lim_exclusive: u8) -> u8 {
    let mut buf = [0u8; 1];
    getrandom::fill(&mut buf).unwrap();
    buf[0] % upper_lim_exclusive
}
/// generates a random [`usize`]
fn get_random_usize(upper_lim_exclusive: usize) -> usize {
    let mut buf = [0u8; (usize::BITS / 8u32) as usize];
    getrandom::fill(&mut buf).unwrap();
    usize::from_le_bytes(buf) % upper_lim_exclusive
}
fn get_random_square_permutation() -> [usize; 81] {
    // fill a buffer with random bytes
    let mut rands = [0u8; 81];
    getrandom::fill(&mut rands).unwrap();
    // create an array holding the sequence of numbers 0 to 80
    let mut res = [0; 81];
    for i in 0..81 {
        res[i] = i
    }
    // sort the sequence with respect to the random bytes,
    // creating a random permutation
    res.sort_unstable_by(|a, b| rands[*a].cmp(&rands[*b]));
    res
}
