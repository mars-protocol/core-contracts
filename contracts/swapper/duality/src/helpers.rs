use std::{collections::HashSet, hash::Hash};

use cosmwasm_std::CosmosMsg;
use mars_swapper_base::{ContractError, ContractResult};
use neutron_sdk::{
    bindings::msg::NeutronMsg,
    proto_types::neutron::dex::MsgPlaceLimitOrder,
    stargate::{aux::create_stargate_msg, dex::types::PlaceLimitOrderRequest},
};

// Precision of the decimal values used in the Neutron DEX
// They use 27 decimal places for their PrecDec type
const PREC_DEC_PRECISION: usize = 27;

// Fully qualified protobuf type URL for the Neutron DEX limit order message.
// This path is defined in the Neutron proto files, https://github.com/neutron-org/neutron/blob/main/proto/neutron/dex/tx.proto#L135.
const PLACE_LIMIT_ORDER_MSG_PATH: &str = "/neutron.dex.MsgPlaceLimitOrder";

/// Build a hashset from array data
pub(crate) fn hashset<T: Eq + Clone + Hash>(data: &[T]) -> HashSet<T> {
    data.iter().cloned().collect()
}
/// Creates a Cosmos message for placing a limit order on the Neutron DEX.
///
/// This function wraps the MsgPlaceLimitOrder in a CosmosMsg that can be directly
/// returned from contract execution. It uses our custom serialization for PrecDec
/// values to ensure proper handling of decimal prices.
///
/// # Arguments
///
/// * `req` - The PlaceLimitOrderRequest containing all parameters for the limit order
///
/// # Returns
///
/// A CosmosMsg that can be included in the response of a contract execution
pub(crate) fn msg_place_limit_order(
    req: PlaceLimitOrderRequest,
) -> ContractResult<CosmosMsg<NeutronMsg>> {
    Ok(create_stargate_msg(PLACE_LIMIT_ORDER_MSG_PATH, from(req)?))
}

/// Converts a PlaceLimitOrderRequest into a MsgPlaceLimitOrder with proper price serialization.
///
/// This function exists primarily to intercept and fix the serialization of the limit_sell_price
/// field, ensuring that decimal values are properly formatted for the Neutron chain. Without this
/// conversion, prices with leading zeros (like "0.01") would fail to serialize correctly.
///
/// # Arguments
///
/// * `v` - The PlaceLimitOrderRequest to convert
///
/// # Returns
///
/// A properly formatted MsgPlaceLimitOrder with correct PrecDec serialization
fn from(v: PlaceLimitOrderRequest) -> ContractResult<MsgPlaceLimitOrder> {
    let price = v.limit_sell_price.clone();
    let mut msg = MsgPlaceLimitOrder::from(v);
    msg.limit_sell_price = serialize_prec_dec(&price)?;
    Ok(msg)
}

/// Serializes a decimal string into the format expected by PrecDec in the Neutron SDK.
///
/// This custom implementation fixes a bug in the standard PrecDec serialization that
/// fails when handling decimal values with a zero integer part (e.g. "0.09999").
/// The issue occurs because the standard implementation incorrectly preserves leading zeros
/// after conversion, resulting in invalid strings like "09999..." that are rejected by big.Int.
///
/// This implementation properly handles leading zeros and maintains the required
/// fixed-point precision of 27 decimal places used by the PrecDec type.
///
/// # Arguments
///
/// * `decimal_str` - A decimal value as a string (e.g. "1.23", "0.01")
///
/// # Returns
///
/// A string representation of the decimal as a fixed-point integer with the leading
/// zeros properly removed, ready for PrecDec serialization.
fn serialize_prec_dec(decimal_str: &str) -> ContractResult<String> {
    // Basic validation
    if decimal_str.is_empty() {
        return Err(ContractError::InvalidInput {
            reason: "Empty input".to_string(),
        });
    }

    // Split into parts
    let (integer_part, fractional_part) = match decimal_str.split_once('.') {
        Some((int_part, frac_part)) => (int_part, frac_part),
        None => (decimal_str, ""),
    };

    // Remove leading zeros from integer part, keep at least one "0"
    let integer_clean = integer_part.trim_start_matches('0');
    let integer_clean = if integer_clean.is_empty() {
        "0"
    } else {
        integer_clean
    };

    // Remove trailing zeros from fractional part
    let fractional_clean = fractional_part.trim_end_matches('0');

    // Handle fractional part that's too long
    let fractional_to_use = if fractional_clean.len() > PREC_DEC_PRECISION {
        &fractional_clean[..PREC_DEC_PRECISION]
    } else {
        fractional_clean
    };

    // Build result efficiently
    let mut result = String::with_capacity(integer_clean.len() + PREC_DEC_PRECISION);

    // Special case for zero
    if integer_clean == "0" && fractional_to_use.is_empty() {
        result.push('0');
        result.push_str(&"0".repeat(PREC_DEC_PRECISION));
        return Ok(result);
    }

    // Build result
    result.push_str(integer_clean);
    result.push_str(fractional_to_use);

    // Add missing zeros
    let zeros_to_add = PREC_DEC_PRECISION.saturating_sub(fractional_to_use.len());
    result.push_str(&"0".repeat(zeros_to_add));

    // Remove leading zeros from result (keep at least one)
    let final_result = result.trim_start_matches('0');
    Ok(if final_result.is_empty() {
        "0".to_string()
    } else {
        final_result.to_string()
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashset() {
        let data = vec![1, 2, 3, 4, 5];
        let set = hashset(&data);
        assert_eq!(set.len(), 5);
        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
        assert!(set.contains(&4));
        assert!(set.contains(&5));
    }

    #[test]
    fn test_serialize_prec_dec() {
        // Standard case with both integer and decimal parts
        assert_eq!(serialize_prec_dec("1.23").unwrap(), "1230000000000000000000000000");

        // Case with leading zero in integer part (the buggy case)
        assert_eq!(serialize_prec_dec("0.01").unwrap(), "10000000000000000000000000");

        // Case with only integer part
        assert_eq!(serialize_prec_dec("42").unwrap(), "42000000000000000000000000000");

        // Zero case
        assert_eq!(serialize_prec_dec("0.0").unwrap(), "0000000000000000000000000000");

        // Case with trailing zeros in fractional part
        assert_eq!(serialize_prec_dec("1.2300").unwrap(), "1230000000000000000000000000");

        // Case with long fractional part
        assert_eq!(serialize_prec_dec("0.000123").unwrap(), "123000000000000000000000");

        // Edge case: exactly 27 digits in fractional part
        assert_eq!(
            serialize_prec_dec("0.123456789012345678901234567").unwrap(),
            "123456789012345678901234567"
        );
    }
}
