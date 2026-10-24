//! Index-based red-black tree over the node pool.
//!
//! The tree never allocates: callers hand it a node whose key is already set.
//! Keys are unique `u128`s. All links are `u32` indices with [`NIL`] as null.

use crate::node::{Node, FLAG_RED, NIL};
use crate::{EngineError, Result};

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

impl Tree<'_> {
    #[inline]
    fn n(&self, i: u32) -> Result<&Node> {
        node(self.nodes, i)
    }

    #[inline]
    fn m(&mut self, i: u32) -> Result<&mut Node> {
        self.nodes.get_mut(i as usize).ok_or(EngineError::Corrupted)
    }

    #[inline]
    fn left(&self, i: u32) -> Result<u32> {
        Ok(self.n(i)?.left)
    }

    #[inline]
    fn right(&self, i: u32) -> Result<u32> {
        Ok(self.n(i)?.right)
    }

    #[inline]
    fn parent(&self, i: u32) -> Result<u32> {
        Ok(self.n(i)?.parent)
    }

    #[inline]
    fn is_red(&self, i: u32) -> Result<bool> {
        if i == NIL {
            return Ok(false);
        }
        Ok(self.n(i)?.flags & FLAG_RED != 0)
    }

    #[inline]
    fn set_red(&mut self, i: u32, red: bool) -> Result<()> {
        if i == NIL {
            return Ok(());
        }
        let n = self.m(i)?;
        if red {
            n.flags |= FLAG_RED;
        } else {
            n.flags &= !FLAG_RED;
        }
        Ok(())
    }

    fn set_left(&mut self, i: u32, v: u32) -> Result<()> {
        self.m(i)?.left = v;
        Ok(())
    }

    fn set_right(&mut self, i: u32, v: u32) -> Result<()> {
        self.m(i)?.right = v;
        Ok(())
    }

    fn set_parent(&mut self, i: u32, v: u32) -> Result<()> {
        if i != NIL {
            self.m(i)?.parent = v;
        }
        Ok(())
    }

    /// Replaces `old` with `new` in `old`'s parent (or the root).
    fn replace_child(&mut self, p: u32, old: u32, new: u32) -> Result<()> {
        if p == NIL {
            *self.root = new;
        } else if self.left(p)? == old {
            self.set_left(p, new)?;
        } else {
            self.set_right(p, new)?;
        }
        Ok(())
    }

    fn rotate_left(&mut self, x: u32) -> Result<()> {
        let y = self.right(x)?;
        let yl = self.left(y)?;
        self.set_right(x, yl)?;
        self.set_parent(yl, x)?;
        let xp = self.parent(x)?;
        self.set_parent(y, xp)?;
        self.replace_child(xp, x, y)?;
        self.set_left(y, x)?;
        self.set_parent(x, y)
    }

    fn rotate_right(&mut self, x: u32) -> Result<()> {
        let y = self.left(x)?;
        let yr = self.right(y)?;
        self.set_left(x, yr)?;
        self.set_parent(yr, x)?;
        let xp = self.parent(x)?;
        self.set_parent(y, xp)?;
        self.replace_child(xp, x, y)?;
        self.set_right(y, x)?;
        self.set_parent(x, y)
    }

    /// Inserts node `z` (key already set). Fails on duplicate keys without
    /// modifying the tree.
    pub fn insert(&mut self, z: u32) -> Result<()> {
        let key = self.n(z)?.key();
        let mut y = NIL;
        let mut x = *self.root;
        let mut go_left = false;
        while x != NIL {
            y = x;
            let k = self.n(x)?.key();
            if key == k {
                return Err(EngineError::Corrupted);
            }
            go_left = key < k;
            x = if go_left { self.left(x)? } else { self.right(x)? };
        }
        {
            let zn = self.m(z)?;
            zn.parent = y;
            zn.left = NIL;
            zn.right = NIL;
            zn.flags |= FLAG_RED;
        }
        if y == NIL {
            *self.root = z;
        } else if go_left {
            self.set_left(y, z)?;
        } else {
            self.set_right(y, z)?;
        }
        self.insert_fixup(z)
    }

    fn insert_fixup(&mut self, mut z: u32) -> Result<()> {
        while z != *self.root && self.is_red(self.parent(z)?)? {
            let p = self.parent(z)?;
            let g = self.parent(p)?;
            if p == self.left(g)? {
                let u = self.right(g)?;
                if self.is_red(u)? {
                    self.set_red(p, false)?;
                    self.set_red(u, false)?;
                    self.set_red(g, true)?;
                    z = g;
                } else {
                    if z == self.right(p)? {
                        z = p;
                        self.rotate_left(z)?;
                    }
                    let p = self.parent(z)?;
                    let g = self.parent(p)?;
                    self.set_red(p, false)?;
                    self.set_red(g, true)?;
                    self.rotate_right(g)?;
                }
            } else {
                let u = self.left(g)?;
                if self.is_red(u)? {
                    self.set_red(p, false)?;
                    self.set_red(u, false)?;
                    self.set_red(g, true)?;
                    z = g;
                } else {
                    if z == self.left(p)? {
                        z = p;
                        self.rotate_right(z)?;
                    }
                    let p = self.parent(z)?;
                    let g = self.parent(p)?;
                    self.set_red(p, false)?;
                    self.set_red(g, true)?;
                    self.rotate_left(g)?;
                }
            }
        }
        let r = *self.root;
        self.set_red(r, false)
    }

    fn transplant(&mut self, u: u32, v: u32) -> Result<()> {
        let up = self.parent(u)?;
        self.replace_child(up, u, v)?;
        self.set_parent(v, up)
    }

    /// Unlinks node `z` from the tree. The node itself is left for the caller
    /// to free.
    pub fn remove(&mut self, z: u32) -> Result<()> {
        let mut y_red = self.is_red(z)?;
        let x;
        let xp;
        let zl = self.left(z)?;
        let zr = self.right(z)?;
        if zl == NIL {
            x = zr;
            xp = self.parent(z)?;
            self.transplant(z, zr)?;
        } else if zr == NIL {
            x = zl;
            xp = self.parent(z)?;
            self.transplant(z, zl)?;
        } else {
            let y = min(self.nodes, zr)?;
            y_red = self.is_red(y)?;
            x = self.right(y)?;
            if self.parent(y)? == z {
                xp = y;
            } else {
                xp = self.parent(y)?;
                self.transplant(y, x)?;
                self.set_right(y, zr)?;
                self.set_parent(zr, y)?;
            }
            self.transplant(z, y)?;
            self.set_left(y, zl)?;
            self.set_parent(zl, y)?;
            let z_red = self.is_red(z)?;
            self.set_red(y, z_red)?;
        }
        if !y_red {
            self.remove_fixup(x, xp)?;
        }
        Ok(())
    }

    fn remove_fixup(&mut self, mut x: u32, mut xp: u32) -> Result<()> {
        while x != *self.root && !self.is_red(x)? {
            if xp == NIL {
                break;
            }
            if x == self.left(xp)? {
                let mut w = self.right(xp)?;
                if self.is_red(w)? {
                    self.set_red(w, false)?;
                    self.set_red(xp, true)?;
                    self.rotate_left(xp)?;
                    w = self.right(xp)?;
                }
                if !self.is_red(self.left(w)?)? && !self.is_red(self.right(w)?)? {
                    self.set_red(w, true)?;
                    x = xp;
                    xp = self.parent(x)?;
                } else {
                    if !self.is_red(self.right(w)?)? {
                        let wl = self.left(w)?;
                        self.set_red(wl, false)?;
                        self.set_red(w, true)?;
                        self.rotate_right(w)?;
                        w = self.right(xp)?;
                    }
                    let xp_red = self.is_red(xp)?;
                    self.set_red(w, xp_red)?;
                    self.set_red(xp, false)?;
                    let wr = self.right(w)?;
                    self.set_red(wr, false)?;
                    self.rotate_left(xp)?;
                    x = *self.root;
                    xp = NIL;
                }
            } else {
                let mut w = self.left(xp)?;
                if self.is_red(w)? {
                    self.set_red(w, false)?;
                    self.set_red(xp, true)?;
                    self.rotate_right(xp)?;
                    w = self.left(xp)?;
                }
                if !self.is_red(self.left(w)?)? && !self.is_red(self.right(w)?)? {
                    self.set_red(w, true)?;
                    x = xp;
                    xp = self.parent(x)?;
                } else {
                    if !self.is_red(self.left(w)?)? {
                        let wr = self.right(w)?;
                        self.set_red(wr, false)?;
                        self.set_red(w, true)?;
                        self.rotate_left(w)?;
                        w = self.left(xp)?;
                    }
                    let xp_red = self.is_red(xp)?;
                    self.set_red(w, xp_red)?;
                    self.set_red(xp, false)?;
                    let wl = self.left(w)?;
                    self.set_red(wl, false)?;
                    self.rotate_right(xp)?;
                    x = *self.root;
                    xp = NIL;
                }
            }
        }
        self.set_red(x, false)
    }
}

/// Checks red-black and ordering properties. Returns the node count.
pub(crate) fn validate(nodes: &[Node], root: u32) -> Result<u32> {
    fn walk(nodes: &[Node], i: u32, parent: u32, lo: Option<u128>, hi: Option<u128>, count: &mut u32) -> Result<u32> {
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
