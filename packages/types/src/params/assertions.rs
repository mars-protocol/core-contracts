use cosmwasm_std::{Decimal, Uint128};
use mars_utils::error::ValidationError;

pub(super) fn assert_lqt_gt_max_ltv(
    max_ltv: Decimal,
    liq_threshold: Decimal,
) -> Result<(), ValidationError> {
    if liq_threshold <= max_ltv {
        return Err(ValidationError::InvalidParam {
            param_name: "liquidation_threshold".to_string(),
            invalid_value: liq_threshold.to_string(),
            predicate: format!("> {} (max LTV)", max_ltv),
        });
    }
    Ok(())
}

pub(super) fn assert_lqt_usdc_gt_max_ltv_usdc(
    max_ltv: Option<Decimal>,
    liq_threshold: Option<Decimal>,
) -> Result<(), ValidationError> {
    if let (Some(max_ltv_unwrapped), Some(liq_threshold_unwrapped)) = (max_ltv, liq_threshold) {
        if liq_threshold_unwrapped <= max_ltv_unwrapped {
            return Err(ValidationError::InvalidParam {
                param_name: "liquidation_threshold_usdc".to_string(),
                invalid_value: liq_threshold_unwrapped.to_string(),
                predicate: format!("> {} (max LTV USDC)", max_ltv_unwrapped),
            });
        }
    }
    Ok(())
}

pub(super) fn assert_hls_lqt_gt_max_ltv(
    max_ltv: Decimal,
    liq_threshold: Decimal,
) -> Result<(), ValidationError> {
    if liq_threshold <= max_ltv {
        return Err(ValidationError::InvalidParam {
            param_name: "hls_liquidation_threshold".to_string(),
            invalid_value: liq_threshold.to_string(),
            predicate: format!("> {} (hls max LTV)", max_ltv),
        });
    }
    Ok(())
}

pub(super) fn assert_max_net_oi_le_max_oi_long(
    max_long_oi_value: Uint128,
    max_net_oi_value: Uint128,
) -> Result<(), ValidationError> {
    if max_net_oi_value > max_long_oi_value {
        return Err(ValidationError::InvalidParam {
            param_name: "max_long_oi_value".to_string(),
            invalid_value: max_long_oi_value.to_string(),
            predicate: format!(">= {} (max_net_oi_value)", max_net_oi_value),
        });
    }
    Ok(())
}

pub(super) fn assert_max_net_oi_le_max_oi_short(
    max_short_oi_value: Uint128,
    max_net_oi_value: Uint128,
) -> Result<(), ValidationError> {
    if max_net_oi_value > max_short_oi_value {
        return Err(ValidationError::InvalidParam {
            param_name: "max_short_oi_value".to_string(),
            invalid_value: max_short_oi_value.to_string(),
            predicate: format!(">= {} (max_net_oi_value)", max_net_oi_value),
        });
    }
    Ok(())
}

pub(super) fn assert_max_size_gt_min(
    max_position_value: Option<Uint128>,
    min_position_value: Uint128,
) -> Result<(), ValidationError> {
    // if we have a max position value, ensure it is greater than the min position value
    if let Some(max_value_unwrapped) = max_position_value {
        if max_value_unwrapped <= min_position_value {
            return Err(ValidationError::InvalidParam {
                param_name: "max_position_value".to_string(),
                invalid_value: max_value_unwrapped.to_string(),
                predicate: format!(">= {} (min position value)", min_position_value),
            });
        }
    };

    Ok(())
}

pub(super) fn assert_skew_scale(skew_scale: Uint128) -> Result<(), ValidationError> {
    if skew_scale.is_zero() {
        return Err(ValidationError::InvalidParam {
            param_name: "skew_scale".to_string(),
            invalid_value: skew_scale.to_string(),
            predicate: "> 0".to_string(),
        });
    }
    Ok(())
}

pub(super) fn assert_starting_lb_within_range(b: Decimal) -> Result<(), ValidationError> {
    if b > Decimal::percent(10) {
        return Err(ValidationError::InvalidParam {
            param_name: "starting_lb".to_string(),
            invalid_value: b.to_string(),
            predicate: "[0, 0.1]".to_string(),
        });
    }
    Ok(())
}

pub(super) fn assert_lb_slope_within_range(slope: Decimal) -> Result<(), ValidationError> {
    if slope < Decimal::one() || slope > Decimal::from_ratio(5u8, 1u8) {
        return Err(ValidationError::InvalidParam {
            param_name: "slope".to_string(),
            invalid_value: slope.to_string(),
            predicate: "[1, 5]".to_string(),
        });
    }
    Ok(())
}

pub(super) fn assert_min_lb_within_range(min_lb: Decimal) -> Result<(), ValidationError> {
    if min_lb > Decimal::percent(10) {
        return Err(ValidationError::InvalidParam {
            param_name: "min_lb".to_string(),
            invalid_value: min_lb.to_string(),
            predicate: "[0, 0.1]".to_string(),
        });
    }
    Ok(())
}

pub(super) fn assert_max_lb_within_range(max_lb: Decimal) -> Result<(), ValidationError> {
    if max_lb < Decimal::percent(5) || max_lb > Decimal::percent(30) {
        return Err(ValidationError::InvalidParam {
            param_name: "max_lb".to_string(),
            invalid_value: max_lb.to_string(),
            predicate: "[0.05, 0.3]".to_string(),
        });
    }
    Ok(())
}

pub(super) fn assert_max_lb_gt_min_lb(
    min_lb: Decimal,
    max_lb: Decimal,
) -> Result<(), ValidationError> {
    if min_lb > max_lb {
        return Err(ValidationError::InvalidParam {
            param_name: "max_lb".to_string(),
            invalid_value: max_lb.to_string(),
            predicate: format!("> {} (min LB)", min_lb),
        });
    }
    Ok(())
}
