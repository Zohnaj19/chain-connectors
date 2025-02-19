/*
 * Rosetta
 *
 * Build Once. Integrate Your Blockchain Everywhere.
 *
 * The version of the OpenAPI document: 1.4.13
 *
 * Generated by: https://openapi-generator.tech
 */

/// BlockEvent : BlockEvent represents the addition or removal of a BlockIdentifier from storage. Streaming BlockEvents allows lightweight clients to update their own state without needing to implement their own syncing logic.

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BlockEvent {
    /// sequence is the unique identifier of a BlockEvent within the context of a NetworkIdentifier.
    #[serde(rename = "sequence")]
    pub sequence: i64,
    #[serde(rename = "block_identifier")]
    pub block_identifier: crate::BlockIdentifier,
    #[serde(rename = "type")]
    pub r#type: crate::BlockEventType,
}

impl BlockEvent {
    /// BlockEvent represents the addition or removal of a BlockIdentifier from storage. Streaming BlockEvents allows lightweight clients to update their own state without needing to implement their own syncing logic.
    pub fn new(
        sequence: i64,
        block_identifier: crate::BlockIdentifier,
        r#type: crate::BlockEventType,
    ) -> BlockEvent {
        BlockEvent {
            sequence,
            block_identifier,
            r#type,
        }
    }
}
