//! Mark-and-sweep heap.
//!
//! Cells keep their indexes. A sweep pushes an unmarked cell onto a free list
//! instead of compacting, so a value's index stays valid until that cell is
//! reused.

use alloc::{string::String, vec::Vec};

use crate::error::Error;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Data {
    Undefined,
    Boolean(bool),
    Number(f64),
    String(u32),
    Object(u32),
}

impl Data {
    fn heap_index(self) -> Option<u32> {
        match self {
            Self::String(index) | Self::Object(index) => Some(index),
            Self::Undefined | Self::Boolean(_) | Self::Number(_) => None,
        }
    }
}

enum Object {
    String(String),
    Function {
        chunk: u16,
        env: u32,
    },
    Env {
        slots: Vec<Data>,
        parent: Option<u32>,
    },
}

struct Cell {
    marked: bool,
    data: Option<Object>,
}

pub(crate) struct Heap {
    cells: Vec<Cell>,
    free: Vec<u32>,
    since_gc: usize,
}

impl Heap {
    pub(crate) fn new() -> Self {
        Self {
            cells: Vec::new(),
            free: Vec::new(),
            since_gc: 0,
        }
    }

    pub(crate) const fn needs_gc(&self) -> bool {
        self.since_gc >= GC_EVERY
    }

    pub(crate) fn alloc_string(&mut self, text: String) -> Result<u32, Error> {
        self.alloc(Object::String(text))
    }

    pub(crate) fn alloc_function(&mut self, chunk: u16, env: u32) -> Result<u32, Error> {
        self.alloc(Object::Function { chunk, env })
    }

    pub(crate) fn alloc_env(&mut self, locals: u16, parent: Option<u32>) -> Result<u32, Error> {
        let mut slots = Vec::new();
        slots.resize(usize::from(locals), Data::Undefined);
        self.alloc(Object::Env { slots, parent })
    }

    pub(crate) fn string(&self, index: u32) -> Option<&str> {
        match self.object(index)? {
            Object::String(text) => Some(text.as_str()),
            Object::Function { .. } | Object::Env { .. } => None,
        }
    }

    pub(crate) fn string_nonempty(&self, index: u32) -> bool {
        self.string(index).is_some_and(|text| !text.is_empty())
    }

    pub(crate) fn function(&self, index: u32) -> Option<(u16, u32)> {
        match self.object(index)? {
            Object::Function { chunk, env } => Some((*chunk, *env)),
            Object::String(_) | Object::Env { .. } => None,
        }
    }

    pub(crate) fn parent(&self, index: u32) -> Option<u32> {
        match self.object(index)? {
            Object::Env { parent, .. } => *parent,
            Object::String(_) | Object::Function { .. } => None,
        }
    }

    pub(crate) fn slot(&self, env: u32, slot: u16) -> Option<Data> {
        match self.object(env)? {
            Object::Env { slots, .. } => slots.get(usize::from(slot)).copied(),
            Object::String(_) | Object::Function { .. } => None,
        }
    }

    pub(crate) fn set_slot(&mut self, env: u32, slot: u16, value: Data) -> Result<(), Error> {
        let Some(object) = self.object_mut(env) else {
            return Err(Error::runtime());
        };
        let Object::Env { slots, .. } = object else {
            return Err(Error::runtime());
        };
        let Some(cell) = slots.get_mut(usize::from(slot)) else {
            return Err(Error::runtime());
        };
        *cell = value;
        Ok(())
    }

    pub(crate) fn collect(&mut self, values: &[Data], envs: &[u32]) {
        let mut work = Vec::new();
        for value in values {
            if let Some(index) = value.heap_index() {
                work.push(index);
            }
        }
        work.extend_from_slice(envs);
        self.mark(&mut work);
        self.sweep();
        self.since_gc = 0;
    }

    fn alloc(&mut self, object: Object) -> Result<u32, Error> {
        self.since_gc = self.since_gc.saturating_add(1);
        if let Some(index) = self.free.pop() {
            return self.reuse(index, object);
        }
        let index = u32::try_from(self.cells.len()).map_err(|_| Error::runtime())?;
        self.cells.push(Cell {
            marked: false,
            data: Some(object),
        });
        Ok(index)
    }

    fn reuse(&mut self, index: u32, object: Object) -> Result<u32, Error> {
        let slot = usize::try_from(index).map_err(|_| Error::runtime())?;
        let Some(cell) = self.cells.get_mut(slot) else {
            return Err(Error::runtime());
        };
        cell.marked = false;
        cell.data = Some(object);
        Ok(index)
    }

    fn mark(&mut self, work: &mut Vec<u32>) {
        while let Some(index) = work.pop() {
            if self.try_mark(index) {
                self.push_children(index, work);
            }
        }
    }

    fn try_mark(&mut self, index: u32) -> bool {
        let Some(cell) = self.cell_mut(index) else {
            return false;
        };
        if cell.marked || cell.data.is_none() {
            return false;
        }
        cell.marked = true;
        true
    }

    fn push_children(&self, index: u32, work: &mut Vec<u32>) {
        let Some(object) = self.object(index) else {
            return;
        };
        match object {
            Object::String(_) => {}
            Object::Function { env, .. } => work.push(*env),
            Object::Env { slots, parent } => push_env(slots, *parent, work),
        }
    }

    fn sweep(&mut self) {
        let mut freed = Vec::new();
        for (index, cell) in self.cells.iter_mut().enumerate() {
            if cell.marked {
                cell.marked = false;
            } else if cell.data.take().is_some() {
                freed.push(index);
            }
        }
        for index in freed {
            if let Ok(index) = u32::try_from(index) {
                self.free.push(index);
            }
        }
    }

    fn object(&self, index: u32) -> Option<&Object> {
        self.cell(index).and_then(|cell| cell.data.as_ref())
    }

    fn object_mut(&mut self, index: u32) -> Option<&mut Object> {
        self.cell_mut(index).and_then(|cell| cell.data.as_mut())
    }

    fn cell(&self, index: u32) -> Option<&Cell> {
        let index = usize::try_from(index).ok()?;
        self.cells.get(index)
    }

    fn cell_mut(&mut self, index: u32) -> Option<&mut Cell> {
        let index = usize::try_from(index).ok()?;
        self.cells.get_mut(index)
    }
}

fn push_env(slots: &[Data], parent: Option<u32>, work: &mut Vec<u32>) {
    if let Some(parent) = parent {
        work.push(parent);
    }
    for slot in slots {
        if let Some(index) = slot.heap_index() {
            work.push(index);
        }
    }
}

const GC_EVERY: usize = 16;

#[cfg(test)]
mod tests {
    use super::{Data, Heap};

    #[test]
    fn sweep_reuses_an_unmarked_string() {
        let mut heap = Heap::new();
        let keep = heap.alloc_string("keep".into()).expect("keep");
        let discard = heap.alloc_string("discard".into()).expect("discard");
        heap.collect(&[Data::String(keep)], &[]);
        assert_eq!(heap.string(keep), Some("keep"));
        assert_eq!(heap.string(discard), None);
        let reused = heap.alloc_string("new".into()).expect("new");
        assert_eq!(reused, discard);
        assert_eq!(heap.string(reused), Some("new"));
    }

    #[test]
    fn mark_follows_the_env_and_stops_on_a_cycle() {
        let mut heap = Heap::new();
        let env = heap.alloc_env(1, None).expect("env");
        let function = heap.alloc_function(0, env).expect("function");
        heap.set_slot(env, 0, Data::Object(function)).expect("slot");
        let discard = heap.alloc_string("gone".into()).expect("gone");
        heap.collect(&[], &[env]);
        assert!(heap.function(function).is_some());
        assert_eq!(heap.string(discard), None);
        let Some(Data::Object(slot)) = heap.slot(env, 0) else {
            panic!("slot");
        };
        assert_eq!(slot, function);
    }
}
