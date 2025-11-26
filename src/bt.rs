use std::{collections::HashMap, marker::PhantomData};

use actify::Handle;
use uuid::Uuid;

use crate::{Action, Condition, execution::engine_factory::{Engine, EngineFactory, Engines}, nodes::{action::Executor, condition::Evaluator}, nodes_bin::{node::Node, node_map::NodeHandleMap}};

pub(crate) const CHANNEL_SIZE: usize = 20;

// TODO remove all the Option<> for Typestate with variables
pub struct BT<T: State> {
    name: String,
    pub(crate) root: Node,
    pub(crate) map: NodeHandleMap,
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
    pub fn new() -> BT<Building> {
        Self {
            name: "Unnamed Behavior Tree".to_string(),
            root: Node::Action("Initial action".to_string()), //TODO Risky non-existant mapping here, maybe Option<..>
            map: HashMap::new(),
            engine_factory: EngineFactory { engine: Engines::Default },
            result: None,
            marker: PhantomData,
        }.into_state::<Building>()
    }

    pub fn action<T: Executor + Send + Sync + 'static>(inner: T) -> BT<Tree>{
        let uid = Uuid::new_v4();
        let root = Node::Action(uid.into());
        let mut map = HashMap::new();
        map.insert(uid.into(), Action::new(inner));
        Self {
            name: "Unnamed Behavior Tree".to_string(),
            root,
            map,
            engine_factory: EngineFactory { engine: Engines::Default },
            result: None,
            marker: PhantomData,
        }.into_state::<Tree>()
    }

    pub fn condition<V,T>(handle: Handle<V>, inner: T) -> BT<Tree> 
    where
        V: Clone + std::fmt::Debug + Send + Sync + Clone + 'static,
        T: Evaluator<V> + Fn(V) -> bool + Sync + Send + Clone + 'static,
    {
        let uid = Uuid::new_v4();
        let root = Node::Condition(uid.into());
        let mut map = HashMap::new();
        map.insert(uid.into(), Condition::new(uid.to_string(),handle, inner));
        Self {
            name: "Unnamed Behavior Tree".to_string(),
            root,
            map,
            engine_factory: EngineFactory { engine: Engines::Default },
            result: None,
            marker: PhantomData,
        }.into_state::<Tree>()
    }

    pub fn seq(children: Vec<BT<Tree>>) -> BT<Tree>{
        let mut map = HashMap::new();
        let mut node_children = vec![];
        for child in children {
            map.extend(child.map);
            node_children.push(child.root);
        }
        let root = Node::Sequence(node_children);
        Self {
            name: "Unnamed Behavior Tree".to_string(),
            root,
            map,
            engine_factory: EngineFactory { engine: Engines::Default },
            result: None,
            marker: PhantomData,
        }.into_state::<Tree>()
    }

    pub fn fb(children: Vec<BT<Tree>>) -> BT<Tree>{
        let mut map = HashMap::new();
        let mut node_children = vec![];
        for child in children {
            map.extend(child.map);
            node_children.push(child.root);
        }
        let root = Node::Fallback(node_children);
        Self {
            name: "Unnamed Behavior Tree".to_string(),
            root,
            map,
            engine_factory: EngineFactory { engine: Engines::Default },
            result: None,
            marker: PhantomData,
        }.into_state::<Tree>()
    }
}

impl BT<Building> {
    pub fn root(mut self, tree: BT<Tree>) -> BT<Ready> {
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
    // TODO Fix state here, create better API than manually defining the mapping
    pub fn test_insert_map(mut self, map: NodeHandleMap) -> BT<Building> {
        self.map.extend(map);
        self
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

pub trait State {}
pub trait NotDone {}

// State transitions: Init -> new() -> Building -> root() -> Ready -> run() -> Done
pub struct Init;
pub struct Building;
pub struct Ready;
pub struct Done;

// Seperate for tree construction macros: Init -> Tree via action()/condition()/fb()/seq(). Is the argument for root()
pub struct Tree;

impl NotDone for Init {}
impl NotDone for Building {}
impl NotDone for Ready {}
impl<T: NotDone> State for T {}
impl State for Done {}

impl State for Tree {}