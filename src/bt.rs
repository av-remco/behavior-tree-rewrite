use std::marker::PhantomData;

use crate::{execution::executor_factory::{Engine, EngineFactory, Engines}, nodes_bin::{node_handle::NodeHandle, node_index::NodeIndex}};

pub(crate) const CHANNEL_SIZE: usize = 20;

pub struct BT<T: BuildState> {
    name: String,
    pub(crate) root: Option<NodeHandle>,
    pub(crate) node_index: NodeIndex,
    engine_factory: EngineFactory,
    result: Option<bool>,
    marker: PhantomData<T>,
}

impl<T: BuildState> BT<T> {
    fn into_state<S: BuildState>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            node_index: self.node_index,
            engine_factory: self.engine_factory,
            result: self.result,
            marker: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn test_into_state<S: BuildState>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            node_index: self.node_index,
            engine_factory: self.engine_factory,
            result: self.result,
            marker: PhantomData,
        }
    }
}

impl BT<Init> {
    pub fn new() -> Self {
        Self {
            name: "Unnamed Behavior Tree".to_string(),
            root: None,
            node_index: NodeIndex::new(vec![]),
            engine_factory: EngineFactory { engine: Engines::Default },
            result: None,
            marker: PhantomData,
        }
    }

    pub fn root(mut self, mut root: NodeHandle) -> BT<Ready> {
        let handles = root.take_handles();
        self.node_index = NodeIndex::new(handles);
        self.root = Some(root);
        self.into_state::<Ready>()
    }
}

impl<T: NotDone> BT<T> {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn set_engine(&mut self, engine: Engines) {
        self.engine_factory.set(engine);
    }
}

impl BT<Ready> {
    pub async fn run(mut self) -> BT<Done> {
        let mut engine = self.engine_factory.create(&self);
        self.result = Some(engine.run().await);
        self.into_state::<Done>()
    }
}

impl BT<Done> {
    pub fn result(&self) -> bool {
        if let Some(res) = self.result {
            res
        } else {
            panic!("Unexpected None in Done behavior tree");
        }
    }
}

pub trait BuildState {}
pub trait NotDone {}
pub struct Init;
pub struct Ready;
pub struct Done;

impl BuildState for Done {}
impl NotDone for Init {}
impl NotDone for Ready {}
impl<T: NotDone> BuildState for T {}