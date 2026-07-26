use std::fmt;

use crate::geometry::{Dimensions, Length, LengthConversionError};
use crate::model::InputData;

/// Stable identity of a container within one validated input document.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContainerId(usize);

impl ContainerId {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Stable identity of an item within one validated input document.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ItemId(usize);

impl ItemId {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// One validated container, addressed independently of its display name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Container {
    id: ContainerId,
    name: String,
    dimensions: Dimensions,
}

impl Container {
    #[must_use]
    pub const fn id(&self) -> ContainerId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn dimensions(&self) -> Dimensions {
        self.dimensions
    }
}

/// One validated packable item, addressed independently of its display name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item {
    id: ItemId,
    name: String,
    dimensions: Dimensions,
}

impl Item {
    #[must_use]
    pub const fn id(&self) -> ItemId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn dimensions(&self) -> Dimensions {
        self.dimensions
    }
}

/// Compatibility input after all dimensions have been converted and checked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackingInstance {
    containers: Vec<Container>,
    items: Vec<Item>,
}

impl PackingInstance {
    #[must_use]
    pub fn containers(&self) -> &[Container] {
        &self.containers
    }

    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }
}

impl TryFrom<&InputData> for PackingInstance {
    type Error = InputValidationErrors;

    fn try_from(input: &InputData) -> Result<Self, Self::Error> {
        let mut errors = Vec::new();

        let containers = input
            .containers
            .iter()
            .enumerate()
            .filter_map(|(index, container)| {
                convert_dimensions(
                    InputSection::Containers,
                    index,
                    container.width,
                    container.length,
                    container.height,
                    &mut errors,
                )
                .map(|dimensions| Container {
                    id: ContainerId(index),
                    name: container.name.clone(),
                    dimensions,
                })
            })
            .collect();

        let items = input
            .contents
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                convert_dimensions(
                    InputSection::Contents,
                    index,
                    item.width,
                    item.length,
                    item.height,
                    &mut errors,
                )
                .map(|dimensions| Item {
                    id: ItemId(index),
                    name: item.name.clone(),
                    dimensions,
                })
            })
            .collect();

        if errors.is_empty() {
            Ok(Self { containers, items })
        } else {
            Err(InputValidationErrors(errors))
        }
    }
}

fn convert_dimensions(
    section: InputSection,
    index: usize,
    width: f64,
    length: f64,
    height: f64,
    errors: &mut Vec<InputValidationError>,
) -> Option<Dimensions> {
    let width = convert_dimension(section, index, DimensionField::Width, width, errors);
    let length = convert_dimension(section, index, DimensionField::Length, length, errors);
    let height = convert_dimension(section, index, DimensionField::Height, height, errors);

    let dimensions = Dimensions::new(width?, length?, height?);
    if dimensions.checked_volume().is_none() {
        errors.push(InputValidationError::VolumeOverflow { section, index });
        return None;
    }

    Some(dimensions)
}

fn convert_dimension(
    section: InputSection,
    index: usize,
    field: DimensionField,
    value: f64,
    errors: &mut Vec<InputValidationError>,
) -> Option<Length> {
    match Length::from_input_units(value) {
        Ok(length) => Some(length),
        Err(reason) => {
            errors.push(InputValidationError::InvalidDimension {
                section,
                index,
                field,
                value,
                reason,
            });
            None
        }
    }
}

/// Compatibility array containing an invalid value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputSection {
    Containers,
    Contents,
}

impl InputSection {
    const fn field_name(self) -> &'static str {
        match self {
            Self::Containers => "containers",
            Self::Contents => "contents",
        }
    }
}

/// Dimension field containing an invalid value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DimensionField {
    Width,
    Length,
    Height,
}

impl DimensionField {
    const fn field_name(self) -> &'static str {
        match self {
            Self::Width => "width",
            Self::Length => "length",
            Self::Height => "height",
        }
    }
}

/// One precise input-validation diagnostic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputValidationError {
    InvalidDimension {
        section: InputSection,
        index: usize,
        field: DimensionField,
        value: f64,
        reason: LengthConversionError,
    },
    VolumeOverflow {
        section: InputSection,
        index: usize,
    },
}

impl fmt::Display for InputValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension {
                section,
                index,
                field,
                value,
                reason,
            } => write!(
                formatter,
                "{}[{index}].{} {reason} (got {value})",
                section.field_name(),
                field.field_name(),
            ),
            Self::VolumeOverflow { section, index } => write!(
                formatter,
                "{}[{index}] has a scaled volume too large to represent",
                section.field_name(),
            ),
        }
    }
}

/// All dimension errors found during one compatibility-input conversion.
#[derive(Clone, Debug, PartialEq)]
pub struct InputValidationErrors(Vec<InputValidationError>);

impl InputValidationErrors {
    #[must_use]
    pub fn errors(&self) -> &[InputValidationError] {
        &self.0
    }
}

impl fmt::Display for InputValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "input validation failed with {} error(s)",
            self.0.len()
        )?;
        for error in &self.0 {
            write!(formatter, "\n- {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for InputValidationErrors {}
