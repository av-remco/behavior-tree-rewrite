use std::{collections::HashMap, marker::PhantomData};

use actify::Handle;
use uuid::Uuid;

use crate::{Action, Condition, execution::engine_factory::{Engine, EngineFactory, Engines}, nodes::{action::Executor, condition::Evaluator}, nodes_bin::{node::Node, node_map::NodeIdToProcessHandleMap}};

pub(crate) const CHANNEL_SIZE: usize = 20;

pub struct BT<T: State> {
    name: String,
    pub(crate) root: Node,
    pub(crate) map: NodeIdToProcessHandleMap,
    engine_factory: EngineFactory,
    result: Option<bool>,
    marker: PhantomData<T>,
}

impl<T: State> BT<T> {
    fn into_state<S: State>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            map: self.map,
            engine_factory: self.engine_factory,
            result: self.result,
            marker: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn test_into_state<S: State>(self) -> BT<S> {
        BT::<S> {
            name: self.name,
            root: self.root,
            map: self.map,
            engine_factory: self.engine_factory,
            result: self.result,
            marker: PhantomData,
        }
    }
}

impl BT<Init> {
    pub fn new() -> BT<Preparing> {
        Self {
            name: "Unnamed Behavior Tree".to_string(),
            root: Node::Action("Initial action".to_string()), //TODO Risky non-existant mapping here, maybe Option<..>
            map: HashMap::new(),
            engine_factory: EngineFactory { engine: Engines::Dynamic },
            result: None,
            marker: PhantomData,
        }.into_state::<Preparing>()
    }

    pub fn action<T: Executor + Send + Sync + 'static>(inner: T) -> BT<Builder>{
        let uid = Uuid::new_v4();
        let root = Node::Action(uid.into());
        let mut map = HashMap::new();
        map.insert(uid.into(), Action::new(inner));
        
        let mut bt = BT::new();
        bt.root = root;
        bt.map = map;
        bt.into_state::<Builder>()
    }

    pub fn condition<V,T>(handle: Handle<V>, inner: T) -> BT<Builder> 
    where
        V: Clone + std::fmt::Debug + Send + Sync + 'static,
        T: Evaluator<V> + Sync + Send + Clone + 'static,
    {
        let uid = Uuid::new_v4();
        let root = Node::Condition(uid.into());
        let mut map = HashMap::new();
        map.insert(uid.into(), Condition::new_from(inner, handle));
        
        let mut bt = BT::new();
        bt.root = root;
        bt.map = map;
        bt.into_state::<Builder>()
    }

    pub fn seq(children: Vec<BT<Builder>>) -> BT<Builder>{
        let mut map = HashMap::new();
        let mut node_children = vec![];
        for child in children {
            map.extend(child.map);
            node_children.push(child.root);
        }
        let root = Node::Sequence(node_children);
        
        let mut bt = BT::new();
        bt.root = root;
        bt.map = map;
        bt.into_state::<Builder>()
    }

    pub fn fb(children: Vec<BT<Builder>>) -> BT<Builder>{
        let mut map = HashMap::new();
        let mut node_children = vec![];
        for child in children {
            map.extend(child.map);
            node_children.push(child.root);
        }
        let root = Node::Fallback(node_children);
        
        let mut bt = BT::new();
        bt.root = root;
        bt.map = map;
        bt.into_state::<Builder>()
    }
}

impl BT<Preparing> {
    pub fn root(mut self, tree: BT<Builder>) -> BT<Ready> {
        self.root = tree.root;
        self.map = tree.map;
        self.into_state::<Ready>()
    }

    #[cfg(test)]
    pub fn test_root(mut self, root: Node) -> BT<Ready> {
        self.root = root;
        self.into_state::<Ready>()
    }

    #[cfg(test)]
    pub fn test_insert_map(mut self, map: NodeIdToProcessHandleMap) -> BT<Preparing> {
        self.map.extend(map);
        self
    }
}

impl<T: NotDone> BT<T> {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn set_engine(mut self, engine: Engines) -> Self {
        self.engine_factory.set(engine);
        self
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

pub trait State {}
pub trait NotDone {}

// State transitions: Init -> new() -> Preparing -> root() -> Ready -> run() -> Done
pub struct Init;
pub struct Preparing;
pub struct Ready;
pub struct Done;

// Seperate for tree construction macros: Init -> Builder via action()/condition()/fb()/seq(). Is the argument for root()
pub struct Builder;

impl NotDone for Init {}
impl NotDone for Preparing {}
impl NotDone for Ready {}
impl<T: NotDone> State for T {}
impl State for Done {}

impl State for Builder {}