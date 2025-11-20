use actify::{CacheRecvNewestError, Handle};
use anyhow::Result;
use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;
use tokio::sync::broadcast::{channel, Receiver, Sender};

use crate::nodes_bin::{
    node::{Node, NodeType},
    node_error::NodeError,
    node_handle::NodeHandle,
    node_message::{ChildMessage, ParentMessage},
    node_status::Status,
};
use crate::bt::CHANNEL_SIZE;

// Any custom (async) evaluator can be made with this trait
pub trait Evaluator<V> {
    fn get_name(&self) -> String;
    fn evaluate(&mut self, val: V) -> impl Future<Output = Result<bool>> + Send;
}

// If you pass in just a sync closure to condition::new(), this hidden wrapper is used beneath
#[derive(Clone)]
pub struct ClosureEvaluator<V, F>
where
    F: Fn(V) -> bool + Clone,
{
    name: String,
    function: F,
    phantom: PhantomData<V>,
}

impl<V, F> ClosureEvaluator<V, F>
where
    V: Clone + Debug + Send + Sync + Clone + 'static,
    F: Fn(V) -> bool + Sync + Send + Clone + 'static,
{
    pub fn new(name: String, function: F) -> ClosureEvaluator<V, F> {
        Self {
            name,
            function,
            phantom: PhantomData,
        }
    }
}

impl<V, F> Evaluator<V> for ClosureEvaluator<V, F>
where
    V: Clone + Debug + Send + Sync + Clone + 'static,
    F: Fn(V) -> bool + Sync + Send + Clone + 'static,
{
    fn get_name(&self) -> String {
        self.name.clone()
    }

    async fn evaluate(&mut self, val: V) -> Result<bool> {
        Ok((self.function)(val))
    }
}

pub struct Condition {}

impl Condition {
    pub fn new_from<V, T>(evaluator: T, handle: Handle<V>) -> NodeHandle
    where
        T: Evaluator<V> + Clone + Send + Sync + 'static,
        V: Clone + Debug + Send + Sync + Clone + 'static,
    {
        ConditionProcess::new(handle, evaluator)
    }

    pub fn new<V, S, F>(name: S, handle: Handle<V>, function: F) -> NodeHandle
    where
        S: Into<String> + Clone,
        F: Fn(V) -> bool + Sync + Send + Clone + 'static,
        V: Clone + Debug + Send + Sync + Clone + 'static,
    {
        let evaluator = ClosureEvaluator::new(name.into(), function);
        ConditionProcess::new(handle, evaluator)
    }
}

struct ConditionProcess<V, T>
where
    T: Evaluator<V> + Clone + Send + Sync + 'static,
{
    handle: Handle<V>,
    tx: Sender<ParentMessage>,
    rx: Receiver<ChildMessage>,
    status: Status,
    evaluator: T,
    prev_evaluation: bool,
}

impl<V, T> ConditionProcess<V, T>
where
    T: Evaluator<V> + Clone + Send + Sync + 'static,
    V: Clone + Debug + Send + Sync + Clone + 'static,
{
    pub fn new(handle: Handle<V>, evaluator: T) -> NodeHandle {
        let (parent_tx, parent_rx) = channel(CHANNEL_SIZE);
        let (child_tx, child_rx) = channel(CHANNEL_SIZE);

        let node = Self::_new(
            evaluator.clone(),
            handle,
            parent_tx.clone(),
            child_rx,
        );
        tokio::spawn(Self::serve(node));

        NodeHandle::new(child_tx, parent_rx, NodeType::Condition, evaluator.get_name(), vec![], vec![], vec![])
    }

    fn _new(
        evaluator: T,
        handle: Handle<V>,
        tx: Sender<ParentMessage>,
        rx: Receiver<ChildMessage>,
    ) -> Self {
        Self {
            handle,
            evaluator,
            tx,
            rx,
            status: Status::Idle,
            prev_evaluation: false,
        }
    }

    fn update_status(&mut self, status: Status) -> Result<(), NodeError> {
        self.status = status.clone();
        self.notify_parent(ParentMessage::Status(status))?;
        Ok(())
    }

    fn notify_parent(&mut self, msg: ParentMessage) -> Result<(), NodeError> {
        log::debug!(
            "Condition {:?} - notify parent: {:?}",
            self.evaluator.get_name(),
            msg
        );
        self.tx.send(msg)?;
        Ok(())
    }

    async fn process_incoming_val(
        &mut self,
        val: Result<V, CacheRecvNewestError>,
    ) -> Result<(), NodeError> {

        // Skip errors
        let val = match val {
            Err(e) => {
                log::debug!("{e:?}");
                return Ok(());
            }
            Ok(v) => v,
        };

        match self.status {
            Status::Failure => {
                if !self.prev_evaluation && self.run_evaluator(val.clone()).await? {
                    self.update_status(Status::Success)?;
                }
            }
            Status::Success => {
                if self.prev_evaluation && !self.run_evaluator(val.clone()).await? {
                    self.update_status(Status::Failure)?;
                }
            }
            Status::Running => {} // Conditions should never be Running
            Status::Idle => {}
        }
        Ok(())
    }

    async fn process_msg_from_parent(&mut self, msg: ChildMessage) -> Result<(), NodeError> {
        match msg {
            ChildMessage::Start => self.start_workflow().await?,
            ChildMessage::Stop => {
                let status = self.stop_workflow().await?;
                self.update_status(status)?;
            }
            ChildMessage::Kill => return Err(NodeError::KillError),
        }
        Ok(())
    }

    async fn start_workflow(&mut self) -> Result<(), NodeError> {
        if !self.status.is_running() {
            match self.evaluate_now().await? {
                true => self.update_status(Status::Success)?, // Without a child a condition succeeds immediately
                false => self.update_status(Status::Failure)?, // Send failure to parent
            }
        }
        Ok(())
    }

    async fn stop_workflow(&mut self) -> Result<Status, NodeError> {
        Ok(Status::Failure) // Default failure
    }

    async fn run_evaluator(&mut self, val: V) -> Result<bool, NodeError> {
        match self.evaluator.evaluate(val).await {
            Ok(res) => {
                self.prev_evaluation = res;
                Ok(res)
            }
            Err(e) => Err(NodeError::ExecutionError(e.to_string())),
        }
    }

    async fn evaluate_now(&mut self) -> Result<bool, NodeError> {
        let val = self.handle.get().await;
        self.run_evaluator(val).await
    }

    async fn _serve(mut self) -> Result<(), NodeError> {
        let mut cache = self.handle.create_cache().await;
        loop {
            tokio::select! {
                Ok(msg) = self.rx.recv() => self.process_msg_from_parent(msg).await?,
                res = cache.recv_newest() => self.process_incoming_val(res.cloned()).await?,
                else => log::warn!("Only invalid messages received"),
            };
        }
    }
}

impl<V, T> Node for ConditionProcess<V, T>
where
    T: Evaluator<V> + Clone + Send + Sync + 'static,
    V: Clone + Debug + Send + Sync + Clone + 'static,
{
    async fn serve(self) {
        let poison_tx = self.tx.clone();
        let name = self.evaluator.get_name();
        let res = Self::_serve(self).await;

        log::debug!("Condition {name:?} exited with error: {res:?}");

        match res {
            Err(err) => match err {
                NodeError::KillError => {
                    // Notify the handles
                    if let Err(e) = poison_tx.send(ParentMessage::Killed) {
                        log::warn!("Condition {name:?} - killing acknowledgement failed! {e:?}")
                    }
                }
                NodeError::PoisonError(e) => poison_parent(poison_tx, name, e), // Propagate error
                err => poison_parent(poison_tx, name, err.to_string()), // If any error in itself, poison parent
            },
            Ok(_) => {} // Should never occur
        }
    }
}

fn poison_parent(poison_tx: Sender<ParentMessage>, name: String, err: String) {
    log::debug!("Condition {name:?} - poisoning parent");
    if let Err(e) = poison_tx.send(ParentMessage::Poison(NodeError::PoisonError(err))) {
        log::warn!("Condition {name:?} - poisoning the parent failed! {e:?}")
    }
}
