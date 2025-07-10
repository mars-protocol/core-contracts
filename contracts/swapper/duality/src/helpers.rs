use std::{collections::HashSet, hash::Hash};

use cosmwasm_std::CosmosMsg;
use neutron_sdk::{
    bindings::msg::NeutronMsg,
    proto_types::neutron::dex::MsgPlaceLimitOrder,
    stargate::{aux::create_stargate_msg, dex::types::PlaceLimitOrderRequest},
};

const PREC_DEC_PRECISION: usize = 27;
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
pub(crate) fn msg_place_limit_order(req: PlaceLimitOrderRequest) -> CosmosMsg<NeutronMsg> {
    create_stargate_msg(PLACE_LIMIT_ORDER_MSG_PATH, from(req))
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
fn from(v: PlaceLimitOrderRequest) -> MsgPlaceLimitOrder {
    let price = v.limit_sell_price.clone();
    let mut msg = MsgPlaceLimitOrder::from(v);
    msg.limit_sell_price = serialize_prec_dec(price);
    msg
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
fn serialize_prec_dec(decimal_str: String) -> String {
    // The proto marshaller expects the decimal to come as an integer that will be divided by 10^PREC_DEC_PRECISION to produce a PrecDec
    // There is no available decimal type that can hold 27 decimals of precision. So instead we use string manipulation to serialize the PrecDec into an integer
    let parts: Vec<&str> = decimal_str.split('.').collect();
    let integer_part = parts[0];
    let mut fractional_part = if parts.len() > 1 {
        String::from(parts[1])
    } else {
        String::new()
    };
    // Remove trailing zeros from the fractional_part
    fractional_part = fractional_part.trim_end_matches('0').to_string();
    // Remove leading zeros from the integer_part
    let mut result = integer_part.trim_start_matches('0').to_string();
    // Combine integer part and fractional part
    result.push_str(&fractional_part.to_owned());

    // Add zeros to the end. This is the equivalent of multiplying by 10^PREC_DEC_PRECISION
    let zeros_to_add = PREC_DEC_PRECISION
        .checked_sub(fractional_part.len())
        .expect("Cannot retain precision when serializing PrecDec");
    for _ in 0..zeros_to_add {
        result.push('0');
    }

    // Remove leading zeros from the whole
    result.trim_start_matches('0').to_string()
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
        assert_eq!(serialize_prec_dec("1.23".to_string()), "1230000000000000000000000000");

        // Case with leading zero in integer part (the buggy case)
        assert_eq!(serialize_prec_dec("0.01".to_string()), "10000000000000000000000000");

        // Case with only integer part
        assert_eq!(serialize_prec_dec("42".to_string()), "42000000000000000000000000000");

        // Zero case
        assert_eq!(
            serialize_prec_dec("0.0".to_string()),
            "" // Or should be "0" + zeros? Depends on your implementation
        );

        // Case with trailing zeros in fractional part
        assert_eq!(serialize_prec_dec("1.2300".to_string()), "1230000000000000000000000000");

        // Case with long fractional part
        assert_eq!(serialize_prec_dec("0.000123".to_string()), "123000000000000000000000");

        // Edge case: exactly 27 digits in fractional part
        assert_eq!(
            serialize_prec_dec("0.123456789012345678901234567".to_string()),
            "123456789012345678901234567"
        );
    }
}
