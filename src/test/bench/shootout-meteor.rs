// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//
// Utilities.
//

// returns an infinite iterator of repeated applications of f to x,
// i.e. [x, f(x), f(f(x)), ...], as haskell iterate function.
fn iterate<'a, T>(f: &'a fn(&T) -> T, x: T) -> Iterate<'a, T> {
    Iterate::new(f, x)
}
struct Iterate<'self, T> {
    priv f: &'self fn(&T) -> T,
    priv next: T
}
impl<'self, T> Iterate<'self, T> {
    fn new<'a>(f: &'a fn(&T) -> T, x: T) -> Iterate<'a, T> {
        Iterate {f: f, next: x}
    }
}
impl<'self, T> Iterator<T> for Iterate<'self, T> {
    fn next(&mut self) -> Option<T> {
        let mut res = (self.f)(&self.next);
        std::util::swap(&mut res, &mut self.next);
        Some(res)
    }
}

// a linked list using borrowed next.
enum List<'self, T> {
    Nil,
    Cons(T, &'self List<'self, T>)
}
struct ListIterator<'self, T> {
    priv cur: &'self List<'self, T>
}
impl<'self, T> List<'self, T> {
    fn iter(&'self self) -> ListIterator<'self, T> {
        ListIterator{cur: self}
    }
}
impl<'self, T> Iterator<&'self T> for ListIterator<'self, T> {
    fn next(&mut self) -> Option<&'self T> {
        match *self.cur {
            Nil => None,
            Cons(ref elt, next) => {
                self.cur = next;
                Some(elt)
            }
        }
    }
}

//
// preprocess
//

// Takes a pieces p on the form [(y1, x1), (y2, x2), ...] and returns
// every possible transformations (the 6 rotations with their
// corresponding mirrored piece), with, as minimum coordinates, (0,
// 0).  If all is false, only generate half of the possibilities (used
// to break the symetry of the board).
fn transform(p: ~[(int, int)], all: bool) -> ~[~[(int, int)]] {
    let mut res =
        // rotations
        iterate(|p| p.iter().map(|&(y, x)| (x + y, -y)).collect(), p)
        .take(if all {6} else {3})
        // mirror
        .flat_map(|p| {
            iterate(|p| p.iter().map(|&(y, x)| (x, y)).collect(), p).take(2)
        }).to_owned_vec();

    // translating to (0, 0) as minimum coordinates.
    for p in res.mut_iter() {
        let (dy, dx) = *p.iter().min_by(|e| *e).unwrap();
        for &(ref mut y, ref mut x) in p.mut_iter() {
            *y -= dy; *x -= dx;
        }
    }

    res
}

// A mask is a piece somewere on the board.  It is represented as a
// u64: for i in the first 50 bits, m[i] = 1 if the cell at (i/5, i%5)
// is occuped.  m[50 + id] = 1 if the identifier of the piece is id.

// Takes a piece with minimum coordinate (0, 0) (as generated by
// transform).  Returns the corresponding mask if p translated by (dy,
// dx) is on the board.
fn mask(dy: int, dx: int, id: uint, p: &[(int, int)]) -> Option<u64> {
    let mut m = 1 << (50 + id);
    for &(y, x) in p.iter() {
        let x = x + dx + (y + (dy % 2)) / 2;
        if x < 0 || x > 4 {return None;}
        let y = y + dy;
        if y < 0 || y > 9 {return None;}
        m |= 1 << (y * 5 + x);
    }
    Some(m)
}

// Makes every possible masks.  masks[id][i] correspond to every
// possible masks for piece with identifier id with minimum coordinate
// (i/5, i%5).
fn make_masks() -> ~[~[~[u64]]] {
    let pieces = ~[
        ~[(0,0),(0,1),(0,2),(0,3),(1,3)],
        ~[(0,0),(0,2),(0,3),(1,0),(1,1)],
        ~[(0,0),(0,1),(0,2),(1,2),(2,1)],
        ~[(0,0),(0,1),(0,2),(1,1),(2,1)],
        ~[(0,0),(0,2),(1,0),(1,1),(2,1)],
        ~[(0,0),(0,1),(0,2),(1,1),(1,2)],
        ~[(0,0),(0,1),(1,1),(1,2),(2,1)],
        ~[(0,0),(0,1),(0,2),(1,0),(1,2)],
        ~[(0,0),(0,1),(0,2),(1,2),(1,3)],
        ~[(0,0),(0,1),(0,2),(0,3),(1,2)]];
    let mut res = ~[];
    for (id, p) in pieces.move_iter().enumerate() {
        // To break the central symetry of the problem, every
        // transformation must be taken except for one piece (piece 3
        // here).
        let trans = transform(p, id != 3);
        let mut cur_piece = ~[];
        for dy in range(0, 10) {
            for dx in range(0, 5) {
                let masks = 
                    trans.iter()
                    .filter_map(|t| mask(dy, dx, id, *t))
                    .collect();
                cur_piece.push(masks);
            }
        }
        res.push(cur_piece);
    }
    res
}

// Check if all coordinates can be covered by an unused piece and that
// all unused piece can be placed on the board.
fn is_board_unfeasible(board: u64, masks: &[~[~[u64]]]) -> bool {
    let mut coverable = board;
    for i in range(0, 50).filter(|&i| board & 1 << i == 0) {
        for (cur_id, pos_masks) in masks.iter().enumerate() {
            if board & 1 << (50 + cur_id) != 0 {continue;}
            for &cur_m in pos_masks[i].iter() {
                if cur_m & board == 0 {coverable |= cur_m;}
            }
        }
        if coverable & (1 << i) == 0 {return true;}
    }
    // check if every coordinates can be covered and every piece can
    // be used.
    coverable != (1 << 60) - 1
}

// Filter the masks that we can prove to result to unfeasible board.
fn filter_masks(masks: &[~[~[u64]]]) -> ~[~[~[u64]]] {
    masks.iter().map(
        |p| p.iter().map(
            |p| p.iter()
                .map(|&m| m)
                .filter(|&m| !is_board_unfeasible(m, masks))
                .collect())
            .collect())
        .collect()
}

// Gets the identifier of a mask.
fn get_id(m: u64) -> u8 {
    for id in range(0, 10) {
        if m & (1 << (id + 50)) != 0 {return id as u8;}
    }
    fail!("{:016x} does not have a valid identifier", m);
}

// Converts a list of mask to a ~str.
fn to_utf8(raw_sol: &List<u64>) -> ~str {
    let mut sol: ~[u8] = std::vec::from_elem(50, '.' as u8);
    for &m in raw_sol.iter() {
        let id = get_id(m);
        for i in range(0, 50) {
            if m & 1 << i != 0 {sol[i] = '0' as u8 + id;}
        }
    }
    std::str::from_utf8_owned(sol)
}

// Prints a solution in ~str form.
fn print_sol(sol: &str) {
    for (i, c) in sol.iter().enumerate() {
        if (i) % 5 == 0 {println("");}
        if (i + 5) % 10 == 0 {print(" ");}
        print!("{} ", c);
    }
    println("");
}

// The data managed during the search
struct Data {
    // If more than stop_after is found, stop the search.
    stop_after: int,
    // Number of solution found.
    nb: int,
    // Lexicographically minimal solution found.
    min: ~str,
    // Lexicographically maximal solution found.
    max: ~str
}

// Records a new found solution.  Returns false if the search must be
// stopped.
fn handle_sol(raw_sol: &List<u64>, data: &mut Data) -> bool {
    // because we break the symetry, 2 solutions correspond to a call
    // to this method: the normal solution, and the same solution in
    // reverse order, i.e. the board rotated by half a turn.
    data.nb += 2;
    let sol1 = to_utf8(raw_sol);
    let sol2: ~str = sol1.iter().invert().collect();

    if data.nb == 2 {
        data.min = sol1.clone();
        data.max = sol1.clone();
    }

    if sol1 < data.min {data.min = sol1.clone();}
    if sol2 < data.min {data.min = sol2.clone();}
    if sol1 > data.max {data.max = sol1;}
    if sol2 > data.max {data.max = sol2;}
    data.nb < data.stop_after
}

// Search for every solutions.  Returns false if the search was
// stopped before the end.
fn search(
    masks: &[~[~[u64]]],
    board: u64,
    mut i: int,
    cur: List<u64>,
    data: &mut Data)
    -> bool
{
    // Search for the lesser empty coordinate.
    while board & (1 << i)  != 0 && i < 50 {i += 1;}
    // the board is full: a solution is found.
    if i >= 50 {return handle_sol(&cur, data);}

    // for every unused piece
    for id in range(0, 10).filter(|id| board & (1 << (id + 50)) == 0) {
        // for each mask that fits on the board
        for &m in masks[id][i].iter().filter(|&m| board & *m == 0) {
            // This check is too costy.
            //if is_board_unfeasible(board | m, masks) {continue;}
            if !search(masks, board | m, i + 1, Cons(m, &cur), data) {
                return false;
            }
        }
    }
    return true;
}

fn main () {
    let args = std::os::args();
    let stop_after = if args.len() <= 1 {
        2098
    } else {
        from_str(args[1]).unwrap()
    };
    let masks = make_masks();
    let masks = filter_masks(masks);
    let mut data = Data {stop_after: stop_after, nb: 0, min: ~"", max: ~""};
    search(masks, 0, 0, Nil, &mut data);
    println!("{} solutions found", data.nb);
    print_sol(data.min);
    print_sol(data.max);
    println("");
}
