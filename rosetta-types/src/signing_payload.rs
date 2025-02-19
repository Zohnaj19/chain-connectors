/*
 * Rosetta
 *
 * Build Once. Integrate Your Blockchain Everywhere.
 *
 * The version of the OpenAPI document: 1.4.13
 *
 * Generated by: https://openapi-generator.tech
 */

/// SigningPayload : SigningPayload is signed by the client with the keypair associated with an AccountIdentifier using the specified SignatureType.  SignatureType can be optionally populated if there is a restriction on the signature scheme that can be used to sign the payload.

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SigningPayload {
    /// [DEPRECATED by `account_identifier` in `v1.4.4`] The network-specific address of the account that should sign the payload.
    #[serde(rename = "address", skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(rename = "account_identifier", skip_serializing_if = "Option::is_none")]
    pub account_identifier: Option<crate::AccountIdentifier>,
    /// Hex-encoded string of the payload bytes.
    #[serde(rename = "hex_bytes")]
    pub hex_bytes: String,
    #[serde(rename = "signature_type", skip_serializing_if = "Option::is_none")]
    pub signature_type: Option<crate::SignatureType>,
}

impl SigningPayload {
    /// SigningPayload is signed by the client with the keypair associated with an AccountIdentifier using the specified SignatureType.  SignatureType can be optionally populated if there is a restriction on the signature scheme that can be used to sign the payload.
    pub fn new(hex_bytes: String) -> SigningPayload {
        SigningPayload {
            address: None,
            account_identifier: None,
            hex_bytes,
            signature_type: None,
        }
    }
}
