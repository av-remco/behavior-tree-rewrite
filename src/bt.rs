use std::marker::PhantomData;

use crate::nodes_bin::{node_handle::NodeHandle, node_index::NodeIndex};

pub(crate) const CHANNEL_SIZE: usize = 20;

pub struct BT<T: BuildState> {
    pub name: String,
    pub root: NodeHandle,
    pub(crate) index: NodeIndex,
    marker: std::marker::PhantomData<T>,
}

impl<T:BuildState> BT<T> {
    fn into_state<S:BuildState>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            index: self.index,
            marker: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn test_into_state<S:BuildState>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            index: self.index,
            marker: PhantomData,
        }
    }
}

impl BT<Building> {
    pub fn new<S: Into<String>>(root_node: NodeHandle, name: S) -> Self {
        BT::_new(root_node, name.into())
    }

    fn _new(mut root: NodeHandle, name: String) -> Self {
        let handles = root.take_handles();
        Self {
            name,
            root,
            index: NodeIndex::new(handles),
            marker: PhantomData,
        }
    }

    pub fn build(self) -> BT<Built> {
        let bt: BT<Converting> = self.into_state();

        // Some logic

        bt.into_state()
    }
}

pub trait BuildState {}
pub struct Building;
pub struct Converting;
pub struct Built;
pub struct Executing;
pub struct Finished;

impl BuildState for Building {}
impl BuildState for Converting {}
impl BuildState for Built {}
impl BuildState for Executing {}
impl BuildState for Finished {}