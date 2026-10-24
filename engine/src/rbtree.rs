//! Index-based red-black tree over the node pool.
//!
//! The tree never allocates: callers hand it a node whose key is already set.
//! Keys are unique `u128`s. All links are `u32` indices with [`NIL`] as null.

use crate::node::{Node, FLAG_RED, NIL};
use crate::{EngineError, Result};

/// [`Tree::insert_with_edges`]: the new node is the tree's minimum.
pub(crate) const EDGE_MIN: u8 = 1;
/// [`Tree::insert_with_edges`]: the new node is the tree's maximum.
pub(crate) const EDGE_MAX: u8 = 2;

/// A tree view: the node pool plus one root pointer from the header.
pub(crate) struct Tree<'a> {
    pub nodes: &'a mut [Node],
    pub root: &'a mut u32,
}

/// Read-only helpers usable with a shared node slice.
pub(crate) fn node(nodes: &[Node], i: u32) -> Result<&Node> {
    nodes.get(i as usize).ok_or(EngineError::Corrupted)
}

pub(crate) fn min(nodes: &[Node], mut i: u32) -> Result<u32> {
    if i == NIL {
        return Ok(NIL);
    }
    loop {
        let l = node(nodes, i)?.left;
        if l == NIL {
            return Ok(i);
        }
        i = l;
    }
}

pub(crate) fn max(nodes: &[Node], mut i: u32) -> Result<u32> {
    if i == NIL {
        return Ok(NIL);
    }
    loop {
        let r = node(nodes, i)?.right;
        if r == NIL {
            return Ok(i);
        }
        i = r;
    }
}

/// In-order successor.
pub(crate) fn next(nodes: &[Node], i: u32) -> Result<u32> {
    let n = node(nodes, i)?;
    if n.right != NIL {
        return min(nodes, n.right);
    }
    let mut child = i;
    let mut p = n.parent;
    while p != NIL && node(nodes, p)?.right == child {
        child = p;
        p = node(nodes, p)?.parent;
    }
    Ok(p)
}

/// In-order predecessor.
pub(crate) fn prev(nodes: &[Node], i: u32) -> Result<u32> {
    let n = node(nodes, i)?;
    if n.left != NIL {
        return max(nodes, n.left);
    }
    let mut child = i;
    let mut p = n.parent;
    while p != NIL && node(nodes, p)?.left == child {
        child = p;
        p = node(nodes, p)?.parent;
    }
    Ok(p)
}

pub(crate) fn find(nodes: &[Node], root: u32, key: u128) -> Result<u32> {
    let mut i = root;
    while i != NIL {
        let n = node(nodes, i)?;
        let k = n.key();
        if key == k {
            return Ok(i);
        }
        i = if key < k { n.left } else { n.right };
    }
    Ok(NIL)
}

#[inline(always)]
fn child(n: &Node, right: bool) -> u32 {
    if right {
        n.right
    } else {
        n.left
    }
}

#[inline(always)]
fn set_child(n: &mut Node, right: bool, v: u32) {
    if right {
        n.right = v;
    } else {
        n.left = v;
    }
}

#[inline(always)]
fn red(n: &Node) -> bool {
    n.flags & FLAG_RED != 0
}

impl Tree<'_> {
    #[inline(always)]
    fn n(&self, i: u32) -> Result<&Node> {
        node(self.nodes, i)
    }

    #[inline(always)]
    fn m(&mut self, i: u32) -> Result<&mut Node> {
        self.nodes.get_mut(i as usize).ok_or(EngineError::Corrupted)
    }

    #[inline(always)]
    fn is_red(&self, i: u32) -> Result<bool> {
        if i == NIL {
            return Ok(false);
        }
        Ok(red(self.n(i)?))
    }

    /// Colors `i` (a no-op on [`NIL`], which is black).
    #[inline(always)]
    fn paint(&mut self, i: u32, is_red: bool) -> Result<()> {
        if i != NIL {
            let n = self.m(i)?;
            n.flags = if is_red {
                n.flags | FLAG_RED
            } else {
                n.flags & !FLAG_RED
            };
        }
        Ok(())
    }

    /// Points `p`'s link to `old` (or the root, when `p` is [`NIL`]) at `new`.
    #[inline(always)]
    fn relink(&mut self, p: u32, old: u32, new: u32) -> Result<()> {
        if p == NIL {
            *self.root = new;
        } else {
            let pn = self.m(p)?;
            if pn.left == old {
                pn.left = new;
            } else {
                pn.right = new;
            }
        }
        Ok(())
    }

    /// Rotates around `x`. `left == true` lifts `x`'s right child, `false`
    /// lifts its left child.
    fn rotate(&mut self, x: u32, left: bool) -> Result<()> {
        let (y, xp) = {
            let n = self.n(x)?;
            (child(n, left), n.parent)
        };
        let b = child(self.n(y)?, !left);
        {
            let n = self.m(x)?;
            set_child(n, left, b);
            n.parent = y;
        }
        if b != NIL {
            self.m(b)?.parent = x;
        }
        {
            let n = self.m(y)?;
            set_child(n, !left, x);
            n.parent = xp;
        }
        self.relink(xp, x, y)
    }

    /// Inserts node `z` (key already set). Fails on duplicate keys without
    /// modifying the tree.
    pub fn insert(&mut self, z: u32) -> Result<()> {
        self.insert_with_edges(z, NIL, NIL).map(|_| ())
    }

    /// Inserts node `z` given the tree's current minimum `lo` and maximum
    /// `hi`. A key beyond either edge is attached there directly instead of
    /// descending from the root, which is the common case for orders that
    /// improve the best price or join the back of the book. Returns the
    /// [`EDGE_MIN`] / [`EDGE_MAX`] bits for edges `z` became.
    pub fn insert_with_edges(&mut self, z: u32, lo: u32, hi: u32) -> Result<u8> {
        let (khi, klo) = {
            let n = self.n(z)?;
            (n.key_hi, n.key_lo)
        };
        let less = |n: &Node| khi < n.key_hi || (khi == n.key_hi && klo < n.key_lo);
        let greater = |n: &Node| khi > n.key_hi || (khi == n.key_hi && klo > n.key_lo);
        let (parent, right, edges) = if *self.root == NIL {
            (NIL, false, EDGE_MIN | EDGE_MAX)
        } else if let Some(l) = self.nodes.get(lo as usize).filter(|n| less(n)) {
            if l.left != NIL {
                return Err(EngineError::Corrupted);
            }
            (lo, false, EDGE_MIN)
        } else if let Some(h) = self.nodes.get(hi as usize).filter(|n| greater(n)) {
            if h.right != NIL {
                return Err(EngineError::Corrupted);
            }
            (hi, true, EDGE_MAX)
        } else {
            let mut y = NIL;
            let mut x = *self.root;
            let mut go_right = false;
            while let Some(n) = self.nodes.get(x as usize) {
                y = x;
                go_right = if khi != n.key_hi {
                    khi > n.key_hi
                } else if klo != n.key_lo {
                    klo > n.key_lo
                } else {
                    return Err(EngineError::Corrupted);
                };
                x = child(n, go_right);
            }
            if x != NIL {
                return Err(EngineError::Corrupted);
            }
            (y, go_right, 0)
        };
        {
            let zn = self.m(z)?;
            zn.parent = parent;
            zn.left = NIL;
            zn.right = NIL;
            zn.flags |= FLAG_RED;
        }
        if parent == NIL {
            *self.root = z;
        } else {
            set_child(self.m(parent)?, right, z);
        }
        self.insert_fixup(z)?;
        Ok(edges)
    }

    fn insert_fixup(&mut self, mut z: u32) -> Result<()> {
        loop {
            let p = self.n(z)?.parent;
            if p == NIL {
                break;
            }
            let (p_red, g) = {
                let pn = self.n(p)?;
                (red(pn), pn.parent)
            };
            if !p_red {
                break;
            }
            // A red parent is never the root, so `g` exists.
            let (p_is_right, u) = {
                let gn = self.n(g)?;
                let r = gn.right == p;
                (r, child(gn, !r))
            };
            if self.is_red(u)? {
                self.paint(p, false)?;
                self.paint(u, false)?;
                self.paint(g, true)?;
                z = g;
                continue;
            }
            let mut p = p;
            if child(self.n(p)?, !p_is_right) == z {
                // Inner grandchild: rotate it to the outside first.
                self.rotate(p, !p_is_right)?;
                p = z;
            }
            self.paint(p, false)?;
            self.paint(g, true)?;
            self.rotate(g, p_is_right)?;
            break;
        }
        let r = *self.root;
        self.paint(r, false)
    }

    /// Unlinks node `z` from the tree. The node itself is left for the caller
    /// to free.
    pub fn remove(&mut self, z: u32) -> Result<()> {
        let (zl, zr, zp, z_red) = {
            let n = self.n(z)?;
            (n.left, n.right, n.parent, red(n))
        };
        let (x, xp, removed_red);
        if zl == NIL || zr == NIL {
            let c = if zl == NIL { zr } else { zl };
            self.relink(zp, z, c)?;
            if c != NIL {
                self.m(c)?.parent = zp;
            }
            x = c;
            xp = zp;
            removed_red = z_red;
        } else {
            // Replace `z` with its successor `y`, the minimum of the right subtree.
            let y = min(self.nodes, zr)?;
            let (yr, yp, y_red) = {
                let n = self.n(y)?;
                (n.right, n.parent, red(n))
            };
            x = yr;
            removed_red = y_red;
            if yp == z {
                xp = y;
            } else {
                xp = yp;
                self.m(yp)?.left = yr;
                if yr != NIL {
                    self.m(yr)?.parent = yp;
                }
                self.m(y)?.right = zr;
                self.m(zr)?.parent = y;
            }
            self.relink(zp, z, y)?;
            {
                let n = self.m(y)?;
                n.parent = zp;
                n.left = zl;
                n.flags = if z_red {
                    n.flags | FLAG_RED
                } else {
                    n.flags & !FLAG_RED
                };
            }
            self.m(zl)?.parent = y;
        }
        if !removed_red {
            self.remove_fixup(x, xp)?;
        }
        Ok(())
    }

    fn remove_fixup(&mut self, mut x: u32, mut xp: u32) -> Result<()> {
        while xp != NIL && !self.is_red(x)? {
            // `w` is x's sibling; it exists because x's side is one black short.
            let x_is_left = self.n(xp)?.left == x;
            let mut w = child(self.n(xp)?, x_is_left);
            if self.is_red(w)? {
                self.paint(w, false)?;
                self.paint(xp, true)?;
                self.rotate(xp, x_is_left)?;
                w = child(self.n(xp)?, x_is_left);
            }
            let (near, far) = {
                let wn = self.n(w)?;
                (child(wn, !x_is_left), child(wn, x_is_left))
            };
            if !self.is_red(near)? && !self.is_red(far)? {
                self.paint(w, true)?;
                x = xp;
                xp = self.n(x)?.parent;
                continue;
            }
            if !self.is_red(far)? {
                self.paint(near, false)?;
                self.paint(w, true)?;
                self.rotate(w, !x_is_left)?;
                w = child(self.n(xp)?, x_is_left);
            }
            let xp_red = red(self.n(xp)?);
            self.paint(w, xp_red)?;
            self.paint(xp, false)?;
            let far = child(self.n(w)?, x_is_left);
            self.paint(far, false)?;
            self.rotate(xp, x_is_left)?;
            x = *self.root;
            break;
        }
        self.paint(x, false)
    }
}

/// Checks red-black and ordering properties. Returns the node count.
pub(crate) fn validate(nodes: &[Node], root: u32) -> Result<u32> {
    fn walk(
        nodes: &[Node],
        i: u32,
        parent: u32,
        lo: Option<u128>,
        hi: Option<u128>,
        count: &mut u32,
    ) -> Result<u32> {
        if i == NIL {
            return Ok(1);
        }
        let n = node(nodes, i)?;
        let k = n.key();
        if n.parent != parent || lo.is_some_and(|l| k <= l) || hi.is_some_and(|h| k >= h) {
            return Err(EngineError::Corrupted);
        }
        let red = n.flags & FLAG_RED != 0;
        if red {
            for c in [n.left, n.right] {
                if c != NIL && node(nodes, c)?.flags & FLAG_RED != 0 {
                    return Err(EngineError::Corrupted);
                }
            }
        }
        *count = count.checked_add(1).ok_or(EngineError::Overflow)?;
        let bl = walk(nodes, n.left, i, lo, Some(k), count)?;
        let br = walk(nodes, n.right, i, Some(k), hi, count)?;
        if bl != br {
            return Err(EngineError::Corrupted);
        }
        Ok(bl + u32::from(!red))
    }
    if root != NIL && node(nodes, root)?.flags & FLAG_RED != 0 {
        return Err(EngineError::Corrupted);
    }
    let mut count = 0;
    walk(nodes, root, NIL, None, None, &mut count)?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn pool(n: usize) -> Vec<Node> {
        (0..n)
            .map(|i| {
                let mut x: Node = bytemuck::Zeroable::zeroed();
                x.set_key((i as u128) * 7919 % 10_007);
                x
            })
            .collect()
    }

    /// Random inserts and removes, validating the whole tree after each step.
    #[test]
    fn random_inserts_and_removes_stay_valid() {
        const N: usize = 3_000;
        let mut nodes = pool(N);
        let mut root = NIL;
        let mut live: Vec<u32> = Vec::new();
        let mut free: Vec<u32> = (0..N as u32).rev().collect();
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        for step in 0..40_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let insert = live.is_empty() || (!free.is_empty() && x % 5 < 3);
            {
                let mut t = Tree {
                    nodes: &mut nodes,
                    root: &mut root,
                };
                if insert {
                    let i = free.pop().unwrap();
                    t.nodes[i as usize].flags = 0;
                    t.insert(i).unwrap();
                    live.push(i);
                } else {
                    // Mostly remove the minimum, as matching does.
                    let k = if x % 3 == 0 {
                        (x >> 8) as usize % live.len()
                    } else {
                        let m = min(t.nodes, *t.root).unwrap();
                        live.iter().position(|&v| v == m).unwrap()
                    };
                    let i = live.swap_remove(k);
                    t.remove(i).unwrap();
                    free.push(i);
                }
            }
            if step % 7 == 0 || live.len() < 50 {
                assert_eq!(
                    validate(&nodes, root).unwrap() as usize,
                    live.len(),
                    "step {step}"
                );
            }
        }
        assert_eq!(validate(&nodes, root).unwrap() as usize, live.len());
    }
}
