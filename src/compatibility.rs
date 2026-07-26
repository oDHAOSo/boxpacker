//! Temporary bridge from the saved compatibility output to exact domain data.
//!
//! This module resolves display names back to the stable IDs from a validated
//! input instance and converts saved floating-point coordinates exactly once.
//! It deliberately does not decide whether placements are in bounds,
//! non-overlapping, or valid rotations; those checks belong to the independent
//! solution validator.

use std::fmt;

use crate::geometry::{
    Aabb, Coordinate, CoordinateConversionError, Dimensions, Length, LengthConversionError, Point,
    SCALE,
};
use crate::model::{Cuboid, Item, OutputContainer, OutputData, PlacedItem};
use crate::solution::{Placement, Solution};
use crate::validate::{ContainerId, ItemId, PackingInstance};

const ITEM_COLORS: [&str; 8] = [
    "#ff4d4d", "#4d79ff", "#4dff4d", "#ffdb4d", "#9933ff", "#ff8c1a", "#4dffff", "#ff4dff",
];

/// A saved compatibility output paired with stable input identities.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedSolution {
    output: OutputData,
    containers: Vec<SavedContainer>,
    unplaced_items: Vec<ItemId>,
}

impl SavedSolution {
    #[must_use]
    pub fn output(&self) -> &OutputData {
        &self.output
    }

    #[must_use]
    pub fn containers(&self) -> &[SavedContainer] {
        &self.containers
    }

    #[must_use]
    pub fn unplaced_items(&self) -> &[ItemId] {
        &self.unplaced_items
    }

    /// Copy the adapted fixture into the backend-neutral solution model.
    #[must_use]
    pub fn to_solution(&self) -> Solution {
        let placements = self
            .containers
            .iter()
            .flat_map(|container| {
                container.placed_items.iter().map(|placement| {
                    Placement::new(container.container_id, placement.item_id, placement.bounds)
                })
            })
            .collect();
        Solution::new(placements, self.unplaced_items.clone())
    }
}

/// Placements associated with one stable input container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedContainer {
    container_id: ContainerId,
    placed_items: Vec<SavedPlacement>,
}

impl SavedContainer {
    #[must_use]
    pub const fn container_id(&self) -> ContainerId {
        self.container_id
    }

    #[must_use]
    pub fn placed_items(&self) -> &[SavedPlacement] {
        &self.placed_items
    }
}

/// One saved placement converted to stable identity and exact geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SavedPlacement {
    item_id: ItemId,
    bounds: Aabb,
}

impl SavedPlacement {
    #[must_use]
    pub const fn item_id(self) -> ItemId {
        self.item_id
    }

    #[must_use]
    pub const fn bounds(self) -> Aabb {
        self.bounds
    }
}

/// Convert a compatibility output into exact data suitable for report and
/// independent-validator tests.
pub fn adapt_saved_solution(
    instance: &PackingInstance,
    output: OutputData,
) -> Result<SavedSolution, SavedSolutionAdapterError> {
    let mut available_containers = vec![true; instance.containers().len()];
    let mut available_items = vec![true; instance.items().len()];
    let mut containers = Vec::with_capacity(output.containers.len());

    for (container_index, output_container) in output.containers.iter().enumerate() {
        let dimensions = convert_dimensions(
            &format!("containers[{container_index}]"),
            ["width", "length", "height"],
            output_container.width,
            output_container.length,
            output_container.height,
        )?;
        let container_id = find_container(
            instance,
            &available_containers,
            &output_container.name,
            dimensions,
        )
        .ok_or_else(|| SavedSolutionAdapterError::UnknownContainer {
            index: container_index,
            name: output_container.name.clone(),
        })?;
        available_containers[container_id.index()] = false;

        let mut placed_items = Vec::with_capacity(output_container.placed_items.len());
        for (item_index, output_item) in output_container.placed_items.iter().enumerate() {
            let path = format!("containers[{container_index}].placed_items[{item_index}].coords");
            let dimensions = convert_dimensions(
                &path,
                ["w", "l", "h"],
                output_item.coords.w,
                output_item.coords.l,
                output_item.coords.h,
            )?;
            let origin = Point::new(
                convert_coordinate(&format!("{path}.x"), output_item.coords.x)?,
                convert_coordinate(&format!("{path}.y"), output_item.coords.y)?,
                convert_coordinate(&format!("{path}.z"), output_item.coords.z)?,
            );
            let item_id = find_item(
                instance,
                &available_items,
                &output_item.name,
                dimensions,
                true,
            )
            .ok_or_else(|| SavedSolutionAdapterError::UnknownPlacedItem {
                container_index,
                item_index,
                name: output_item.name.clone(),
            })?;
            available_items[item_id.index()] = false;
            placed_items.push(SavedPlacement {
                item_id,
                bounds: Aabb::new(origin, dimensions),
            });
        }

        containers.push(SavedContainer {
            container_id,
            placed_items,
        });
    }

    let mut unplaced_items = Vec::with_capacity(output.unplaced_items.len());
    for (item_index, output_item) in output.unplaced_items.iter().enumerate() {
        let dimensions = convert_dimensions(
            &format!("unplaced_items[{item_index}]"),
            ["width", "length", "height"],
            output_item.width,
            output_item.length,
            output_item.height,
        )?;
        let item_id = find_item(
            instance,
            &available_items,
            &output_item.name,
            dimensions,
            false,
        )
        .ok_or_else(|| SavedSolutionAdapterError::UnknownUnplacedItem {
            index: item_index,
            name: output_item.name.clone(),
        })?;
        available_items[item_id.index()] = false;
        unplaced_items.push(item_id);
    }

    let missing_items = available_items
        .iter()
        .enumerate()
        .filter_map(|(index, available)| available.then_some(instance.items()[index].id()))
        .collect::<Vec<_>>();
    if !missing_items.is_empty() {
        return Err(SavedSolutionAdapterError::MissingItems(missing_items));
    }

    Ok(SavedSolution {
        output,
        containers,
        unplaced_items,
    })
}

/// Map an independently validated domain solution to the legacy output shape.
///
/// Stable IDs select names and original dimensions. Placements and unplaced
/// items are sorted by those IDs so serialization is reproducible regardless
/// of backend construction order.
#[must_use]
pub fn output_from_solution(instance: &PackingInstance, solution: &Solution) -> OutputData {
    let mut placements = solution.placements().to_vec();
    placements.sort_unstable_by_key(|placement| {
        (
            placement.container_id(),
            placement.item_id(),
            placement.bounds().origin().z().get(),
            placement.bounds().origin().y().get(),
            placement.bounds().origin().x().get(),
        )
    });

    let containers = instance
        .containers()
        .iter()
        .map(|container| {
            let dimensions = container.dimensions();
            let placed_items = placements
                .iter()
                .copied()
                .filter(|placement| placement.container_id() == container.id())
                .map(|placement| {
                    let item = &instance.items()[placement.item_id().index()];
                    let bounds = placement.bounds();
                    let origin = bounds.origin();
                    let dimensions = bounds.dimensions();
                    PlacedItem {
                        name: item.name().to_owned(),
                        coords: Cuboid {
                            x: to_input_units(origin.x().get()),
                            y: to_input_units(origin.y().get()),
                            z: to_input_units(origin.z().get()),
                            w: to_input_units(dimensions.width().get()),
                            l: to_input_units(dimensions.length().get()),
                            h: to_input_units(dimensions.height().get()),
                        },
                        color: ITEM_COLORS[placement.item_id().index() % ITEM_COLORS.len()]
                            .to_owned(),
                    }
                })
                .collect();

            OutputContainer {
                name: container.name().to_owned(),
                width: to_input_units(dimensions.width().get()),
                length: to_input_units(dimensions.length().get()),
                height: to_input_units(dimensions.height().get()),
                placed_items,
            }
        })
        .collect();

    let mut unplaced_item_ids = solution.unplaced_items().to_vec();
    unplaced_item_ids.sort_unstable();
    let unplaced_items = unplaced_item_ids
        .into_iter()
        .map(|item_id| {
            let item = &instance.items()[item_id.index()];
            let dimensions = item.dimensions();
            Item {
                name: item.name().to_owned(),
                width: to_input_units(dimensions.width().get()),
                length: to_input_units(dimensions.length().get()),
                height: to_input_units(dimensions.height().get()),
            }
        })
        .collect();

    OutputData {
        containers,
        unplaced_items,
    }
}

fn to_input_units(scaled: u64) -> f64 {
    scaled as f64 / SCALE as f64
}

fn find_container(
    instance: &PackingInstance,
    available: &[bool],
    name: &str,
    dimensions: Dimensions,
) -> Option<ContainerId> {
    let named = instance
        .containers()
        .iter()
        .filter(|container| available[container.id().index()] && container.name() == name);

    named
        .clone()
        .find(|container| container.dimensions() == dimensions)
        .or_else(|| named.into_iter().next())
        .map(|container| container.id())
}

fn find_item(
    instance: &PackingInstance,
    available: &[bool],
    name: &str,
    dimensions: Dimensions,
    dimensions_may_be_rotated: bool,
) -> Option<ItemId> {
    let named = instance
        .items()
        .iter()
        .filter(|item| available[item.id().index()] && item.name() == name);

    named
        .clone()
        .find(|item| {
            if dimensions_may_be_rotated {
                dimensions_are_permutations(item.dimensions(), dimensions)
            } else {
                item.dimensions() == dimensions
            }
        })
        .or_else(|| named.into_iter().next())
        .map(|item| item.id())
}

fn dimensions_are_permutations(left: Dimensions, right: Dimensions) -> bool {
    left.is_permutation_of(right)
}

fn convert_dimensions(
    path: &str,
    fields: [&str; 3],
    width: f64,
    length: f64,
    height: f64,
) -> Result<Dimensions, SavedSolutionAdapterError> {
    Ok(Dimensions::new(
        convert_length(&format!("{path}.{}", fields[0]), width)?,
        convert_length(&format!("{path}.{}", fields[1]), length)?,
        convert_length(&format!("{path}.{}", fields[2]), height)?,
    ))
}

fn convert_length(path: &str, value: f64) -> Result<Length, SavedSolutionAdapterError> {
    Length::from_input_units(value).map_err(|reason| SavedSolutionAdapterError::InvalidDimension {
        path: path.to_owned(),
        value,
        reason,
    })
}

fn convert_coordinate(path: &str, value: f64) -> Result<Coordinate, SavedSolutionAdapterError> {
    Coordinate::from_input_units(value).map_err(|reason| {
        SavedSolutionAdapterError::InvalidCoordinate {
            path: path.to_owned(),
            value,
            reason,
        }
    })
}

/// Failure to resolve or exactly convert a saved compatibility output.
#[derive(Clone, Debug, PartialEq)]
pub enum SavedSolutionAdapterError {
    InvalidDimension {
        path: String,
        value: f64,
        reason: LengthConversionError,
    },
    InvalidCoordinate {
        path: String,
        value: f64,
        reason: CoordinateConversionError,
    },
    UnknownContainer {
        index: usize,
        name: String,
    },
    UnknownPlacedItem {
        container_index: usize,
        item_index: usize,
        name: String,
    },
    UnknownUnplacedItem {
        index: usize,
        name: String,
    },
    MissingItems(Vec<ItemId>),
}

impl fmt::Display for SavedSolutionAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension {
                path,
                value,
                reason,
            } => write!(formatter, "{path} {reason} (got {value})"),
            Self::InvalidCoordinate {
                path,
                value,
                reason,
            } => write!(formatter, "{path} {reason} (got {value})"),
            Self::UnknownContainer { index, name } => {
                write!(
                    formatter,
                    "containers[{index}] does not match input container {name:?}"
                )
            }
            Self::UnknownPlacedItem {
                container_index,
                item_index,
                name,
            } => write!(
                formatter,
                "containers[{container_index}].placed_items[{item_index}] does not match input item {name:?}"
            ),
            Self::UnknownUnplacedItem { index, name } => write!(
                formatter,
                "unplaced_items[{index}] does not match input item {name:?}"
            ),
            Self::MissingItems(item_ids) => write!(
                formatter,
                "saved output omits {} input item(s): {item_ids:?}",
                item_ids.len()
            ),
        }
    }
}

impl std::error::Error for SavedSolutionAdapterError {}
