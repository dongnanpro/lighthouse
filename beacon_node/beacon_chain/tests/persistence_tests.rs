// #![cfg(not(debug_assertions))]

#[macro_use]
extern crate lazy_static;

use beacon_chain::test_utils::{AttestationStrategy, BeaconChainHarness, BlockStrategy};
use sloggers::{null::NullLoggerBuilder, Build};
use std::sync::Arc;
use store::DiskStore;
use tempfile::{tempdir, TempDir};
use types::{EthSpec, Keypair, MinimalEthSpec};

type E = MinimalEthSpec;

// Should ideally be divisible by 3.
pub const VALIDATOR_COUNT: usize = 24;

lazy_static! {
    /// A cached set of keys.
    static ref KEYPAIRS: Vec<Keypair> = types::test_utils::generate_deterministic_keypairs(VALIDATOR_COUNT);
}

fn get_store(db_path: &TempDir) -> Arc<DiskStore> {
    let spec = E::default_spec();
    let hot_path = db_path.path().join("hot_db");
    let cold_path = db_path.path().join("cold_db");
    let log = NullLoggerBuilder.build().expect("logger should build");
    Arc::new(
        DiskStore::open(&hot_path, &cold_path, spec, log).expect("disk store should initialize"),
    )
}

#[test]
fn finalizes_after_resuming_from_db() {
    let validator_count = 16;
    let num_blocks_produced = MinimalEthSpec::slots_per_epoch() * 5;
    let first_half = num_blocks_produced / 2;

    let db_path = tempdir().unwrap();
    let store = get_store(&db_path);

    let harness = BeaconChainHarness::new_with_disk_store(
        MinimalEthSpec,
        store.clone(),
        KEYPAIRS[0..validator_count].to_vec(),
    );

    harness.advance_slot();

    harness.extend_chain(
        first_half as usize,
        BlockStrategy::OnCanonicalHead,
        AttestationStrategy::AllValidators,
    );

    let latest_slot = harness.chain.slot().expect("should have a slot");

    let original_head = harness.chain.head();
    let original_heads = harness.chain.heads();

    assert_eq!(
        original_head.beacon_state.slot, first_half,
        "head should be half way through test"
    );

    drop(harness);

    let resumed_harness = BeaconChainHarness::resume_from_disk_store(
        MinimalEthSpec,
        store,
        KEYPAIRS[0..validator_count].to_vec(),
    );

    // Set the slot clock of the resumed harness to be in the slot following the previous harness.
    //
    // This allows us to produce the block at the next slot.
    resumed_harness
        .chain
        .slot_clock
        .set_slot(latest_slot.as_u64() + 1);

    assert_eq!(
        original_head,
        resumed_harness.chain.head(),
        "resumed head should be same as previous head"
    );

    assert_eq!(
        original_heads,
        resumed_harness.chain.heads(),
        "resumed heads should be same as previous heads"
    );

    resumed_harness.extend_chain(
        (num_blocks_produced - first_half) as usize,
        BlockStrategy::OnCanonicalHead,
        AttestationStrategy::AllValidators,
    );

    let state = &resumed_harness.chain.head().beacon_state;
    assert_eq!(
        state.slot, num_blocks_produced,
        "head should be at the current slot"
    );
    assert_eq!(
        state.current_epoch(),
        num_blocks_produced / MinimalEthSpec::slots_per_epoch(),
        "head should be at the expected epoch"
    );
    assert_eq!(
        state.current_justified_checkpoint.epoch,
        state.current_epoch() - 1,
        "the head should be justified one behind the current epoch"
    );
    assert_eq!(
        state.finalized_checkpoint.epoch,
        state.current_epoch() - 2,
        "the head should be finalized two behind the current epoch"
    );
}