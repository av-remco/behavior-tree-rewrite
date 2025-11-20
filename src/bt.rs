use std::marker::PhantomData;

use crate::nodes_bin::{node_handle::NodeHandle, node_index::NodeIndex};

pub(crate) const CHANNEL_SIZE: usize = 20;

pub struct BT<T: BuildState> {
    pub name: String,
    pub root: NodeHandle,
    pub(crate) index: NodeIndex,
    marker: std::marker::PhantomData<T>,
}

impl<T: BuildState> BT<T> {
    fn into_state<S: BuildState>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            index: self.index,
            marker: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn test_into_state<S: BuildState>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            index: self.index,
            marker: PhantomData,
        }
    }
}

impl BT<Init> {
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

    pub fn build(self) -> BT<Ready> {
        let bt: BT<Processing> = self.into_state();

        // Some building logic

        bt.into_state()
    }
}

impl BT<Ready> {
    pub fn execute(self) -> BT<Done> {
        let bt = self.into_state::<Executing>();

        // Some execution logic

        bt.into_state::<Done>()
    }
}

pub trait BuildState {}
pub struct Init;
pub struct Processing;
pub struct Ready;
pub struct Executing;
pub struct Done;

impl BuildState for Init {}
impl BuildState for Processing {}
impl BuildState for Ready {}
impl BuildState for Executing {}
impl BuildState for Done {}