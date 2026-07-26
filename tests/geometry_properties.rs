use boxpacker::geometry::{
    Dimensions, Length, LengthConversionError, MAX_EXACT_SCALED_LENGTH, SCALE,
};

#[test]
fn one_decimal_input_values_convert_to_exact_scaled_integers() {
    for scaled in 1..=100_000_u64 {
        let input_value = scaled as f64 / SCALE as f64;
        let length = Length::from_input_units(input_value)
            .expect("a value with one decimal place should convert");

        assert_eq!(length.get(), scaled);
    }
}

#[test]
fn conversion_rejects_values_that_cannot_be_exact_positive_lengths() {
    assert_eq!(
        Length::from_input_units(f64::NAN),
        Err(LengthConversionError::NonFinite)
    );
    assert_eq!(
        Length::from_input_units(f64::INFINITY),
        Err(LengthConversionError::NonFinite)
    );
    assert_eq!(
        Length::from_input_units(0.0),
        Err(LengthConversionError::NonPositive)
    );
    assert_eq!(
        Length::from_input_units(-0.1),
        Err(LengthConversionError::NonPositive)
    );
    assert_eq!(
        Length::from_input_units(1.25),
        Err(LengthConversionError::OverPrecision)
    );
    assert_eq!(
        Length::from_input_units(f64::MAX),
        Err(LengthConversionError::OutOfRange)
    );
}

#[test]
fn largest_exact_scaled_integer_is_accepted() {
    let input_value = MAX_EXACT_SCALED_LENGTH as f64 / SCALE as f64;
    let length =
        Length::from_input_units(input_value).expect("exact f64 integer limit should convert");

    assert_eq!(length.get(), MAX_EXACT_SCALED_LENGTH);
}

#[test]
fn scaled_volume_is_checked_instead_of_wrapping() {
    let ordinary_width = Length::from_input_units(1.0).unwrap();
    let ordinary_length = Length::from_input_units(2.0).unwrap();
    let ordinary_height = Length::from_input_units(3.0).unwrap();
    assert_eq!(
        Dimensions::new(ordinary_width, ordinary_length, ordinary_height).checked_volume(),
        Some(6_000)
    );

    let maximum = Length::from_input_units(MAX_EXACT_SCALED_LENGTH as f64 / SCALE as f64).unwrap();
    assert_eq!(
        Dimensions::new(maximum, maximum, maximum).checked_volume(),
        None
    );
}

#[test]
fn rotations_are_exact_unique_and_deterministically_ordered() {
    let dimensions = Dimensions::new(
        Length::from_input_units(1.0).expect("exact width"),
        Length::from_input_units(2.0).expect("exact length"),
        Length::from_input_units(3.0).expect("exact height"),
    );
    let rotations = dimensions.unique_rotations();

    assert_eq!(rotations.len(), 6);
    assert!(rotations.windows(2).all(|pair| {
        let left = (
            pair[0].width().get(),
            pair[0].length().get(),
            pair[0].height().get(),
        );
        let right = (
            pair[1].width().get(),
            pair[1].length().get(),
            pair[1].height().get(),
        );
        left < right
    }));
    assert!(
        rotations
            .iter()
            .all(|rotation| dimensions.is_permutation_of(*rotation))
    );

    let cube = Dimensions::new(
        Length::from_input_units(2.0).expect("exact side"),
        Length::from_input_units(2.0).expect("exact side"),
        Length::from_input_units(2.0).expect("exact side"),
    );
    assert_eq!(cube.unique_rotations(), vec![cube]);
}

#[test]
fn bounded_rotation_properties_hold_for_varied_side_multiplicities() {
    let mut state = 0x7865_2d91_43ab_cdef_u64;

    for _case in 0..10_000 {
        let mut next_side = || {
            state =
                state.wrapping_add(0x9e37_79b9_7f4a_7c15).rotate_left(17) ^ 0xbf58_476d_1ce4_e5b9;
            1 + state % 50
        };
        let sides = [next_side(), next_side(), next_side()];
        let dimensions = Dimensions::new(
            Length::from_scaled_units(sides[0]).expect("generated side is positive"),
            Length::from_scaled_units(sides[1]).expect("generated side is positive"),
            Length::from_scaled_units(sides[2]).expect("generated side is positive"),
        );
        let rotations = dimensions.unique_rotations();
        let distinct_side_count = {
            let mut distinct = sides;
            distinct.sort_unstable();
            if distinct[0] == distinct[2] {
                1
            } else if distinct[0] == distinct[1] || distinct[1] == distinct[2] {
                2
            } else {
                3
            }
        };
        let expected_rotation_count = match distinct_side_count {
            1 => 1,
            2 => 3,
            3 => 6,
            _ => unreachable!("three sides cannot have another distinct count"),
        };

        assert_eq!(rotations.len(), expected_rotation_count);
        assert!(
            rotations
                .windows(2)
                .all(|pair| rotation_key(pair[0]) < rotation_key(pair[1]))
        );
        assert!(
            rotations
                .iter()
                .all(|rotation| dimensions.is_permutation_of(*rotation))
        );
    }
}

fn rotation_key(dimensions: Dimensions) -> (u64, u64, u64) {
    (
        dimensions.width().get(),
        dimensions.length().get(),
        dimensions.height().get(),
    )
}
