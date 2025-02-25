/*
 * Rosetta
 *
 * Build Once. Integrate Your Blockchain Everywhere.
 *
 * The version of the OpenAPI document: 1.4.13
 *
 * Generated by: https://openapi-generator.tech
 */

/// ConstructionCombineRequest : ConstructionCombineRequest is the input to the `/construction/combine` endpoint. It contains the unsigned transaction blob returned by `/construction/payloads` and all required signatures to create a network transaction.

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ConstructionCombineRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: crate::NetworkIdentifier,
    #[serde(rename = "unsigned_transaction")]
    pub unsigned_transaction: String,
    #[serde(rename = "signatures")]
    pub signatures: Vec<crate::Signature>,
}

impl ConstructionCombineRequest {
    /// ConstructionCombineRequest is the input to the `/construction/combine` endpoint. It contains the unsigned transaction blob returned by `/construction/payloads` and all required signatures to create a network transaction.
    pub fn new(
        network_identifier: crate::NetworkIdentifier,
        unsigned_transaction: String,
        signatures: Vec<crate::Signature>,
    ) -> ConstructionCombineRequest {
        ConstructionCombineRequest {
            network_identifier,
            unsigned_transaction,
            signatures,
        }
    }
}
