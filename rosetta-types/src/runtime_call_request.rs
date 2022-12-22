use serde_json::Value;

use crate::{NetworkIdentifier};

/// AccountFaucetRequest : AccountFaucetRequest is sent for faucet on an account.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RuntimeCallRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,
    #[serde(rename = "call_name")]
    pub call_name: String,
    #[serde(rename = "params")]
    pub params: Value,
}

impl RuntimeCallRequest {
    /// AccountCoinsRequest is utilized to make a request on the /account/coins endpoint.
    pub fn new(
        network_identifier: NetworkIdentifier,
        call_name: String,
        params: Value,
    ) -> RuntimeCallRequest {
        RuntimeCallRequest {
            network_identifier,
            call_name,
            params,
        }
    }
}
