use super::*;
use crate::case_result::compare_beacon_state_results_without_caches;
use serde_derive::Deserialize;
use state_processing::per_epoch_processing::process_crosslinks;
use types::{BeaconState, EthSpec};

#[derive(Debug, Clone, Deserialize)]
pub struct EpochProcessingCrosslinks<E: EthSpec> {
    pub description: String,
    #[serde(bound = "E: EthSpec")]
    pub pre: BeaconState<E>,
    #[serde(bound = "E: EthSpec")]
    pub post: Option<BeaconState<E>>,
}

impl<E: EthSpec> YamlDecode for EpochProcessingCrosslinks<E> {
    fn yaml_decode(yaml: &String) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(&yaml.as_str()).unwrap())
    }
}

impl<E: EthSpec> Case for EpochProcessingCrosslinks<E> {
    fn description(&self) -> String {
        self.description.clone()
    }

    fn result(&self, _case_index: usize) -> Result<(), Error> {
        let mut state = self.pre.clone();
        let mut expected = self.post.clone();

        // Processing requires the epoch cache.
        state.build_all_caches(&E::spec()).unwrap();

        let mut result = process_crosslinks(&mut state, &E::spec()).map(|_| state);

        compare_beacon_state_results_without_caches(&mut result, &mut expected)
    }
}