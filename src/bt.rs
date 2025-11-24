use std::marker::PhantomData;

use crate::{execution::executor_factory::{Engine, EngineFactory, Engines}, nodes_bin::{node_handle::NodeHandle, node_index::NodeIndex}};

pub(crate) const CHANNEL_SIZE: usize = 20;

pub struct BT<T: BuildState> {
    name: String,
    pub(crate) root: NodeHandle,
    pub(crate) node_index: NodeIndex,
    executor_factory: EngineFactory,
    result: Option<bool>,
    marker: PhantomData<T>,
}

impl<T: BuildState> BT<T> {
    fn into_state<S: BuildState>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            node_index: self.node_index,
            executor_factory: self.executor_factory,
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
            executor_factory: self.executor_factory,
            result: self.result,
            marker: PhantomData,
        }
    }
}

impl BT<Init> {
    pub fn new(mut root: NodeHandle) -> Self {
        let handles = root.take_handles();
        Self {
            name: "Unnamed Behavior Tree".to_string(),
            root,
            node_index: NodeIndex::new(handles),
            executor_factory: EngineFactory { engine: Engines::Default },
            result: None,
            marker: PhantomData,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn set_executor(&mut self, engine: Engines) {
        self.executor_factory.set(engine);
    }
}

impl BT<Ready> {
    pub async fn execute(mut self) -> BT<Done> {
        let mut exec = self.executor_factory.create(&self);
        self.result = Some(exec.execute().await);

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
pub struct Init;
pub struct Ready;
pub struct Done;

impl BuildState for Init {}
impl BuildState for Ready {}
impl BuildState for Done {}