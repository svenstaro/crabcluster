use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::sync::Arc;
use std::sync::Mutex;

use openraft::async_trait::async_trait;
use openraft::storage::{LogState, Snapshot};
use openraft::{
    AnyError, BasicNode, EffectiveMembership, Entry, EntryPayload, ErrorSubject, ErrorVerb, LogId,
    RaftLogReader, RaftSnapshotBuilder, RaftStorage, SnapshotMeta, StorageError, StorageIOError,
    Vote,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::node::{NodeId, RaftTypeConfig};

/// Here you will set the types of request that will interact with the raft nodes.
/// For example, the `Set` will be used to write data (key and value) to the raft database.
/// The `AddNode` will append a new node to the current existing shared list of nodes.
/// You will want to add any request that can write data in all nodes here.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RaftRequest {
    Set { key: String, value: String },
}

/// Here you will defined what type of answer you expect from reading the data of a node.
/// In this example it will return a optional value from a given key in
/// the `RaftRequest.Set`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RaftResponse {
    pub value: Option<String>,
}

#[derive(Debug)]
pub struct RaftSnapshot {
    pub meta: SnapshotMeta<NodeId, BasicNode>,

    /// The data of the state machine at the time of this snapshot.
    pub data: Vec<u8>,
}

/// Here defines a state machine of the raft, this state represents a copy of the data
/// between each node. Note that we are using `serde` to serialize the `data`, which has
/// a implementation to be serialized. Note that for this test we set both the key and
/// value as String, but you could set any type of value that has the serialization impl.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RaftStateMachine {
    pub last_applied_log: Option<LogId<NodeId>>,

    // TODO: it should not be Option.
    pub last_membership: EffectiveMembership<NodeId, BasicNode>,

    /// Application data.
    pub data: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
pub struct RaftStore {
    last_purged_log_id: RwLock<Option<LogId<NodeId>>>,

    /// The Raft log.
    log: RwLock<BTreeMap<u64, Entry<RaftTypeConfig>>>,

    /// The Raft state machine.
    pub state_machine: RwLock<RaftStateMachine>,

    /// The current granted vote.
    vote: RwLock<Option<Vote<NodeId>>>,

    snapshot_idx: Arc<Mutex<u64>>,

    current_snapshot: RwLock<Option<RaftSnapshot>>,
}

#[async_trait]
impl RaftLogReader<RaftTypeConfig> for Arc<RaftStore> {
    async fn get_log_state(&mut self) -> Result<LogState<RaftTypeConfig>, StorageError<NodeId>> {
        let log = self.log.read().await;
        let last = log.iter().rev().next().map(|(_, ent)| ent.log_id);

        let last_purged = *self.last_purged_log_id.read().await;

        let last = match last {
            None => last_purged,
            Some(x) => Some(x),
        };

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + Send + Sync>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<RaftTypeConfig>>, StorageError<NodeId>> {
        let log = self.log.read().await;
        let response = log
            .range(range.clone())
            .map(|(_, val)| val.clone())
            .collect::<Vec<_>>();
        Ok(response)
    }
}

#[async_trait]
impl RaftSnapshotBuilder<RaftTypeConfig, Cursor<Vec<u8>>> for Arc<RaftStore> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn build_snapshot(
        &mut self,
    ) -> Result<Snapshot<NodeId, BasicNode, Cursor<Vec<u8>>>, StorageError<NodeId>> {
        let data;
        let last_applied_log;
        let last_membership;

        {
            // Serialize the data of the state machine.
            let state_machine = self.state_machine.read().await;
            data = serde_json::to_vec(&*state_machine).map_err(|e| {
                StorageIOError::new(
                    ErrorSubject::StateMachine,
                    ErrorVerb::Read,
                    AnyError::new(&e),
                )
            })?;

            last_applied_log = state_machine.last_applied_log;
            last_membership = state_machine.last_membership.clone();
        }

        let snapshot_idx = {
            let mut l = self.snapshot_idx.lock().unwrap();
            *l += 1;
            *l
        };

        let snapshot_id = if let Some(last) = last_applied_log {
            format!("{}-{}-{}", last.leader_id, last.index, snapshot_idx)
        } else {
            format!("--{}", snapshot_idx)
        };

        let meta = SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership,
            snapshot_id,
        };

        let snapshot = RaftSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };

        {
            let mut current_snapshot = self.current_snapshot.write().await;
            *current_snapshot = Some(snapshot);
        }

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }
}

#[async_trait]
impl RaftStorage<RaftTypeConfig> for Arc<RaftStore> {
    type SnapshotData = Cursor<Vec<u8>>;
    type LogReader = Self;
    type SnapshotBuilder = Self;

    #[tracing::instrument(level = "trace", skip(self))]
    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        let mut v = self.vote.write().await;
        *v = Some(*vote);
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        Ok(*self.vote.read().await)
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn append_to_log(
        &mut self,
        entries: &[&Entry<RaftTypeConfig>],
    ) -> Result<(), StorageError<NodeId>> {
        let mut log = self.log.write().await;
        for entry in entries {
            log.insert(entry.log_id.index, (*entry).clone());
        }
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_conflict_logs_since(
        &mut self,
        log_id: LogId<NodeId>,
    ) -> Result<(), StorageError<NodeId>> {
        tracing::debug!("delete_log: [{:?}, +oo)", log_id);

        let mut log = self.log.write().await;
        let keys = log
            .range(log_id.index..)
            .map(|(k, _v)| *k)
            .collect::<Vec<_>>();
        for key in keys {
            log.remove(&key);
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn purge_logs_upto(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        tracing::debug!("delete_log: [{:?}, +oo)", log_id);

        {
            let mut ld = self.last_purged_log_id.write().await;
            assert!(*ld <= Some(log_id));
            *ld = Some(log_id);
        }

        {
            let mut log = self.log.write().await;

            let keys = log
                .range(..=log_id.index)
                .map(|(k, _v)| *k)
                .collect::<Vec<_>>();
            for key in keys {
                log.remove(&key);
            }
        }

        Ok(())
    }

    async fn last_applied_state(
        &mut self,
    ) -> Result<
        (
            Option<LogId<NodeId>>,
            EffectiveMembership<NodeId, BasicNode>,
        ),
        StorageError<NodeId>,
    > {
        let state_machine = self.state_machine.read().await;
        Ok((
            state_machine.last_applied_log,
            state_machine.last_membership.clone(),
        ))
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn apply_to_state_machine(
        &mut self,
        entries: &[&Entry<RaftTypeConfig>],
    ) -> Result<Vec<RaftResponse>, StorageError<NodeId>> {
        let mut res = Vec::with_capacity(entries.len());

        let mut sm = self.state_machine.write().await;

        for entry in entries {
            tracing::debug!(%entry.log_id, "replicate to sm");

            sm.last_applied_log = Some(entry.log_id);

            match entry.payload {
                EntryPayload::Blank => res.push(RaftResponse { value: None }),
                EntryPayload::Normal(ref req) => match req {
                    RaftRequest::Set { key, value } => {
                        sm.data.insert(key.clone(), value.clone());
                        res.push(RaftResponse {
                            value: Some(value.clone()),
                        })
                    }
                },
                EntryPayload::Membership(ref mem) => {
                    sm.last_membership = EffectiveMembership::new(Some(entry.log_id), mem.clone());
                    res.push(RaftResponse { value: None })
                }
            };
        }
        Ok(res)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Self::SnapshotData>, StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    #[tracing::instrument(level = "trace", skip(self, snapshot))]
    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, BasicNode>,
        snapshot: Box<Self::SnapshotData>,
    ) -> Result<(), StorageError<NodeId>> {
        tracing::info!(
            { snapshot_size = snapshot.get_ref().len() },
            "decoding snapshot for installation"
        );

        let new_snapshot = RaftSnapshot {
            meta: meta.clone(),
            data: snapshot.into_inner(),
        };

        // Update the state machine.
        {
            let updated_state_machine: RaftStateMachine =
                serde_json::from_slice(&new_snapshot.data).map_err(|e| {
                    StorageIOError::new(
                        ErrorSubject::Snapshot(new_snapshot.meta.signature()),
                        ErrorVerb::Read,
                        AnyError::new(&e),
                    )
                })?;
            let mut state_machine = self.state_machine.write().await;
            *state_machine = updated_state_machine;
        }

        // Update current snapshot.
        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(new_snapshot);
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<NodeId, BasicNode, Self::SnapshotData>>, StorageError<NodeId>> {
        match &*self.current_snapshot.read().await {
            Some(snapshot) => {
                let data = snapshot.data.clone();
                Ok(Some(Snapshot {
                    meta: snapshot.meta.clone(),
                    snapshot: Box::new(Cursor::new(data)),
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}
