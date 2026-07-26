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
