/*
 * Rosetta
 *
 * Build Once. Integrate Your Blockchain Everywhere.
 *
 * The version of the OpenAPI document: 1.4.13
 *
 * Generated by: https://openapi-generator.tech
 */

/// SearchTransactionsResponse : SearchTransactionsResponse contains an ordered collection of BlockTransactions that match the query in SearchTransactionsRequest. These BlockTransactions are sorted from most recent block to oldest block.

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SearchTransactionsResponse {
    /// transactions is an array of BlockTransactions sorted by most recent BlockIdentifier (meaning that transactions in recent blocks appear first).  If there are many transactions for a particular search, transactions may not contain all matching transactions. It is up to the caller to paginate these transactions using the max_block field.
    #[serde(rename = "transactions")]
    pub transactions: Vec<crate::BlockTransaction>,
    /// total_count is the number of results for a given search. Callers typically use this value to concurrently fetch results by offset or to display a virtual page number associated with results.
    #[serde(rename = "total_count")]
    pub total_count: i64,
    /// next_offset is the next offset to use when paginating through transaction results. If this field is not populated, there are no more transactions to query.
    #[serde(rename = "next_offset", skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<i64>,
}

impl SearchTransactionsResponse {
    /// SearchTransactionsResponse contains an ordered collection of BlockTransactions that match the query in SearchTransactionsRequest. These BlockTransactions are sorted from most recent block to oldest block.
    pub fn new(
        transactions: Vec<crate::BlockTransaction>,
        total_count: i64,
    ) -> SearchTransactionsResponse {
        SearchTransactionsResponse {
            transactions,
            total_count,
            next_offset: None,
        }
    }
}
