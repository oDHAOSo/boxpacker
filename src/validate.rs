use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::geometry::{Dimensions, Length, LengthConversionError};
use crate::model::InputData;
use crate::solution::{Placement, Solution};

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

/// Validate a backend-neutral solution independently from every solver.
pub fn validate_solution(
    instance: &PackingInstance,
    solution: &Solution,
) -> Result<SolutionSummary, SolutionValidationErrors> {
    let mut errors = Vec::new();
    let mut seen_items = vec![None; instance.items().len()];
    let mut extents = vec![None; solution.placements().len()];

    for (placement_index, placement) in solution.placements().iter().copied().enumerate() {
        let container = instance.containers().get(placement.container_id().index());
        if container.is_none() {
            errors.push(SolutionValidationError::UnknownContainer {
                placement_index,
                container_id: placement.container_id(),
            });
        }

        let item = instance.items().get(placement.item_id().index());
        if let Some(item) = item {
            record_item(
                &mut seen_items,
                item.id(),
                ItemLocation::Placed(placement_index),
                &mut errors,
            );
            if !item
                .dimensions()
                .is_permutation_of(placement.bounds().dimensions())
            {
                errors.push(SolutionValidationError::InvalidOrientation {
                    placement_index,
                    item_id: item.id(),
                    expected: item.dimensions(),
                    actual: placement.bounds().dimensions(),
                });
            }
        } else {
            errors.push(SolutionValidationError::UnknownPlacedItem {
                placement_index,
                item_id: placement.item_id(),
            });
        }

        let placement_extents = exact_extents(placement_index, placement, &mut errors);
        if let (Some(container), Some(placement_extents)) = (container, placement_extents) {
            check_bounds(
                placement_index,
                container.id(),
                container.dimensions(),
                placement_extents,
                &mut errors,
            );
            extents[placement_index] = Some(placement_extents);
        }
    }

    for (unplaced_index, item_id) in solution.unplaced_items().iter().copied().enumerate() {
        if instance.items().get(item_id.index()).is_some() {
            record_item(
                &mut seen_items,
                item_id,
                ItemLocation::Unplaced(unplaced_index),
                &mut errors,
            );
        } else {
            errors.push(SolutionValidationError::UnknownUnplacedItem {
                unplaced_index,
                item_id,
            });
        }
    }

    for (item_index, location) in seen_items.iter().enumerate() {
        if location.is_none() {
            errors.push(SolutionValidationError::MissingItem {
                item_id: instance.items()[item_index].id(),
            });
        }
    }

    for first_index in 0..solution.placements().len() {
        for second_index in (first_index + 1)..solution.placements().len() {
            let first = solution.placements()[first_index];
            let second = solution.placements()[second_index];
            if first.container_id() != second.container_id() {
                continue;
            }
            if let (Some(first_extents), Some(second_extents)) =
                (extents[first_index], extents[second_index])
                && first_extents.overlaps(second_extents)
            {
                errors.push(SolutionValidationError::Overlap {
                    container_id: first.container_id(),
                    first_placement: first_index,
                    second_placement: second_index,
                });
            }
        }
    }

    if !errors.is_empty() {
        return Err(SolutionValidationErrors(errors));
    }

    summarize_solution(instance, solution)
}

fn record_item(
    seen_items: &mut [Option<ItemLocation>],
    item_id: ItemId,
    location: ItemLocation,
    errors: &mut Vec<SolutionValidationError>,
) {
    let seen = &mut seen_items[item_id.index()];
    if let Some(first) = *seen {
        errors.push(SolutionValidationError::DuplicateItem {
            item_id,
            first,
            duplicate: location,
        });
    } else {
        *seen = Some(location);
    }
}

#[derive(Clone, Copy, Debug)]
struct Extents {
    x_min: u64,
    x_max: u64,
    y_min: u64,
    y_max: u64,
    z_min: u64,
    z_max: u64,
}

impl Extents {
    const fn overlaps(self, other: Self) -> bool {
        self.x_min < other.x_max
            && other.x_min < self.x_max
            && self.y_min < other.y_max
            && other.y_min < self.y_max
            && self.z_min < other.z_max
            && other.z_min < self.z_max
    }
}

fn exact_extents(
    placement_index: usize,
    placement: Placement,
    errors: &mut Vec<SolutionValidationError>,
) -> Option<Extents> {
    let origin = placement.bounds().origin();
    let dimensions = placement.bounds().dimensions();
    let x_max = checked_end(
        placement_index,
        Axis::X,
        origin.x().get(),
        origin.x().checked_add(dimensions.width()),
        errors,
    );
    let y_max = checked_end(
        placement_index,
        Axis::Y,
        origin.y().get(),
        origin.y().checked_add(dimensions.length()),
        errors,
    );
    let z_max = checked_end(
        placement_index,
        Axis::Z,
        origin.z().get(),
        origin.z().checked_add(dimensions.height()),
        errors,
    );

    Some(Extents {
        x_min: origin.x().get(),
        x_max: x_max?,
        y_min: origin.y().get(),
        y_max: y_max?,
        z_min: origin.z().get(),
        z_max: z_max?,
    })
}

fn checked_end(
    placement_index: usize,
    axis: Axis,
    origin: u64,
    end: Option<u64>,
    errors: &mut Vec<SolutionValidationError>,
) -> Option<u64> {
    match end {
        Some(end) => Some(end),
        None => {
            errors.push(SolutionValidationError::CoordinateOverflow {
                placement_index,
                axis,
                origin,
            });
            None
        }
    }
}

fn check_bounds(
    placement_index: usize,
    container_id: ContainerId,
    container: Dimensions,
    extents: Extents,
    errors: &mut Vec<SolutionValidationError>,
) {
    for (axis, end, limit) in [
        (Axis::X, extents.x_max, container.width().get()),
        (Axis::Y, extents.y_max, container.length().get()),
        (Axis::Z, extents.z_max, container.height().get()),
    ] {
        if end > limit {
            errors.push(SolutionValidationError::OutOfBounds {
                placement_index,
                container_id,
                axis,
                end,
                limit,
            });
        }
    }
}

fn summarize_solution(
    instance: &PackingInstance,
    solution: &Solution,
) -> Result<SolutionSummary, SolutionValidationErrors> {
    let placed_volume = checked_item_volume_sum(
        instance,
        solution
            .placements()
            .iter()
            .map(|placement| placement.item_id()),
        SummaryMetric::PlacedVolume,
    )?;
    let unplaced_volume = checked_item_volume_sum(
        instance,
        solution.unplaced_items().iter().copied(),
        SummaryMetric::UnplacedVolume,
    )?;

    let used_containers = solution
        .placements()
        .iter()
        .map(|placement| placement.container_id())
        .collect::<BTreeSet<_>>();
    let mut bounding_extents = BTreeMap::<ContainerId, (u64, u64, u64)>::new();
    let mut deterministic_rows = Vec::with_capacity(solution.placements().len());
    let mut exact_placement_extents = Vec::with_capacity(solution.placements().len());

    for placement in solution.placements().iter().copied() {
        let bounds = placement.bounds();
        let origin = bounds.origin();
        let dimensions = bounds.dimensions();
        let x_max = origin
            .x()
            .checked_add(dimensions.width())
            .expect("a validated placement must have exact x extents");
        let y_max = origin
            .y()
            .checked_add(dimensions.length())
            .expect("a validated placement must have exact y extents");
        let z_max = origin
            .z()
            .checked_add(dimensions.height())
            .expect("a validated placement must have exact z extents");
        exact_placement_extents.push(Extents {
            x_min: origin.x().get(),
            x_max,
            y_min: origin.y().get(),
            y_max,
            z_min: origin.z().get(),
            z_max,
        });
        let entry = bounding_extents
            .entry(placement.container_id())
            .or_insert((0, 0, 0));
        entry.0 = entry.0.max(x_max);
        entry.1 = entry.1.max(y_max);
        entry.2 = entry.2.max(z_max);

        let container_dimensions =
            instance.containers()[placement.container_id().index()].dimensions();
        let item_dimensions = instance.items()[placement.item_id().index()].dimensions();
        let mut item_sides = [
            item_dimensions.width().get(),
            item_dimensions.length().get(),
            item_dimensions.height().get(),
        ];
        item_sides.sort_unstable();
        deterministic_rows.push([
            container_dimensions.width().get(),
            container_dimensions.length().get(),
            container_dimensions.height().get(),
            item_sides[0],
            item_sides[1],
            item_sides[2],
            origin.x().get(),
            origin.y().get(),
            origin.z().get(),
            dimensions.width().get(),
            dimensions.length().get(),
            dimensions.height().get(),
        ]);
    }

    let mut bounding_volume = 0_u128;
    for (x_max, y_max, z_max) in bounding_extents.values().copied() {
        let volume = u128::from(x_max)
            .checked_mul(u128::from(y_max))
            .and_then(|area| area.checked_mul(u128::from(z_max)))
            .ok_or_else(|| metric_overflow(SummaryMetric::BoundingVolume))?;
        bounding_volume = bounding_volume
            .checked_add(volume)
            .ok_or_else(|| metric_overflow(SummaryMetric::BoundingVolume))?;
    }

    deterministic_rows.sort_unstable();
    let deterministic_key = deterministic_rows.into_iter().flatten().collect();
    let unsupported_area = calculate_unsupported_area(solution, &exact_placement_extents)?;

    Ok(SolutionSummary {
        placed_item_count: solution.placements().len(),
        unplaced_item_count: solution.unplaced_items().len(),
        placed_volume,
        unplaced_volume,
        used_container_count: used_containers.len(),
        unsupported_area,
        bounding_volume,
        deterministic_key,
    })
}

fn calculate_unsupported_area(
    solution: &Solution,
    extents: &[Extents],
) -> Result<u128, SolutionValidationErrors> {
    let mut total_unsupported = 0_u128;
    for (placement_index, placement) in solution.placements().iter().copied().enumerate() {
        let dimensions = placement.bounds().dimensions();
        let bottom_area = u128::from(dimensions.width().get())
            .checked_mul(u128::from(dimensions.length().get()))
            .ok_or_else(|| metric_overflow(SummaryMetric::UnsupportedArea))?;
        let supported_area = if extents[placement_index].z_min == 0 {
            bottom_area
        } else {
            let mut supported_area = 0_u128;
            for (support_index, support) in solution.placements().iter().copied().enumerate() {
                if support.container_id() != placement.container_id()
                    || extents[support_index].z_max != extents[placement_index].z_min
                {
                    continue;
                }
                let overlap_width = interval_overlap(
                    extents[placement_index].x_min,
                    extents[placement_index].x_max,
                    extents[support_index].x_min,
                    extents[support_index].x_max,
                );
                let overlap_length = interval_overlap(
                    extents[placement_index].y_min,
                    extents[placement_index].y_max,
                    extents[support_index].y_min,
                    extents[support_index].y_max,
                );
                let overlap_area = u128::from(overlap_width)
                    .checked_mul(u128::from(overlap_length))
                    .ok_or_else(|| metric_overflow(SummaryMetric::UnsupportedArea))?;
                supported_area = supported_area
                    .checked_add(overlap_area)
                    .ok_or_else(|| metric_overflow(SummaryMetric::UnsupportedArea))?;
            }
            supported_area.min(bottom_area)
        };
        total_unsupported = total_unsupported
            .checked_add(bottom_area - supported_area)
            .ok_or_else(|| metric_overflow(SummaryMetric::UnsupportedArea))?;
    }
    Ok(total_unsupported)
}

fn interval_overlap(first_min: u64, first_max: u64, second_min: u64, second_max: u64) -> u64 {
    first_max
        .min(second_max)
        .saturating_sub(first_min.max(second_min))
}

fn checked_item_volume_sum(
    instance: &PackingInstance,
    item_ids: impl Iterator<Item = ItemId>,
    metric: SummaryMetric,
) -> Result<u128, SolutionValidationErrors> {
    let mut total = 0_u128;
    for item_id in item_ids {
        let volume = instance.items()[item_id.index()]
            .dimensions()
            .checked_volume()
            .expect("validated input item volumes must be representable");
        total = total
            .checked_add(volume)
            .ok_or_else(|| metric_overflow(metric))?;
    }
    Ok(total)
}

fn metric_overflow(metric: SummaryMetric) -> SolutionValidationErrors {
    SolutionValidationErrors(vec![SolutionValidationError::MetricOverflow { metric }])
}

/// Exact metrics calculated only after a candidate passes validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolutionSummary {
    placed_item_count: usize,
    unplaced_item_count: usize,
    placed_volume: u128,
    unplaced_volume: u128,
    used_container_count: usize,
    unsupported_area: u128,
    bounding_volume: u128,
    deterministic_key: Vec<u64>,
}

impl SolutionSummary {
    #[must_use]
    pub const fn placed_item_count(&self) -> usize {
        self.placed_item_count
    }

    #[must_use]
    pub const fn unplaced_item_count(&self) -> usize {
        self.unplaced_item_count
    }

    #[must_use]
    pub const fn placed_volume(&self) -> u128 {
        self.placed_volume
    }

    #[must_use]
    pub const fn unplaced_volume(&self) -> u128 {
        self.unplaced_volume
    }

    #[must_use]
    pub const fn used_container_count(&self) -> usize {
        self.used_container_count
    }

    #[must_use]
    pub const fn bounding_volume(&self) -> u128 {
        self.bounding_volume
    }

    #[must_use]
    pub const fn unsupported_area(&self) -> u128 {
        self.unsupported_area
    }

    pub(crate) fn deterministic_key(&self) -> &[u64] {
        &self.deterministic_key
    }
}

/// Axis associated with an exact placement diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl fmt::Display for Axis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::X => formatter.write_str("x"),
            Self::Y => formatter.write_str("y"),
            Self::Z => formatter.write_str("z"),
        }
    }
}

/// Where an item first or subsequently appeared in a candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemLocation {
    Placed(usize),
    Unplaced(usize),
}

/// Summary component whose checked arithmetic overflowed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummaryMetric {
    PlacedVolume,
    UnplacedVolume,
    UnsupportedArea,
    BoundingVolume,
}

/// One exact, backend-independent candidate diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SolutionValidationError {
    UnknownContainer {
        placement_index: usize,
        container_id: ContainerId,
    },
    UnknownPlacedItem {
        placement_index: usize,
        item_id: ItemId,
    },
    UnknownUnplacedItem {
        unplaced_index: usize,
        item_id: ItemId,
    },
    InvalidOrientation {
        placement_index: usize,
        item_id: ItemId,
        expected: Dimensions,
        actual: Dimensions,
    },
    CoordinateOverflow {
        placement_index: usize,
        axis: Axis,
        origin: u64,
    },
    OutOfBounds {
        placement_index: usize,
        container_id: ContainerId,
        axis: Axis,
        end: u64,
        limit: u64,
    },
    Overlap {
        container_id: ContainerId,
        first_placement: usize,
        second_placement: usize,
    },
    DuplicateItem {
        item_id: ItemId,
        first: ItemLocation,
        duplicate: ItemLocation,
    },
    MissingItem {
        item_id: ItemId,
    },
    MetricOverflow {
        metric: SummaryMetric,
    },
}

impl fmt::Display for SolutionValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownContainer {
                placement_index,
                container_id,
            } => write!(
                formatter,
                "placement[{placement_index}] references unknown container index {}",
                container_id.index()
            ),
            Self::UnknownPlacedItem {
                placement_index,
                item_id,
            } => write!(
                formatter,
                "placement[{placement_index}] references unknown item index {}",
                item_id.index()
            ),
            Self::UnknownUnplacedItem {
                unplaced_index,
                item_id,
            } => write!(
                formatter,
                "unplaced_items[{unplaced_index}] references unknown item index {}",
                item_id.index()
            ),
            Self::InvalidOrientation {
                placement_index,
                item_id,
                ..
            } => write!(
                formatter,
                "placement[{placement_index}] dimensions are not a rotation of item index {}",
                item_id.index()
            ),
            Self::CoordinateOverflow {
                placement_index,
                axis,
                origin,
            } => write!(
                formatter,
                "placement[{placement_index}] {axis}-extent overflows from origin {origin}"
            ),
            Self::OutOfBounds {
                placement_index,
                container_id,
                axis,
                end,
                limit,
            } => write!(
                formatter,
                "placement[{placement_index}] ends at {axis}={end}, beyond container index {} limit {limit}",
                container_id.index()
            ),
            Self::Overlap {
                container_id,
                first_placement,
                second_placement,
            } => write!(
                formatter,
                "placements[{first_placement}] and [{second_placement}] overlap in container index {}",
                container_id.index()
            ),
            Self::DuplicateItem {
                item_id,
                first,
                duplicate,
            } => write!(
                formatter,
                "item index {} appears more than once ({first:?}, {duplicate:?})",
                item_id.index()
            ),
            Self::MissingItem { item_id } => {
                write!(formatter, "item index {} is missing", item_id.index())
            }
            Self::MetricOverflow { metric } => {
                write!(
                    formatter,
                    "{metric:?} overflowed while summarizing solution"
                )
            }
        }
    }
}

/// All independent-validation failures found in one candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolutionValidationErrors(Vec<SolutionValidationError>);

impl SolutionValidationErrors {
    #[must_use]
    pub fn errors(&self) -> &[SolutionValidationError] {
        &self.0
    }
}

impl fmt::Display for SolutionValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "solution validation failed with {} error(s)",
            self.0.len()
        )?;
        for error in &self.0 {
            write!(formatter, "\n- {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SolutionValidationErrors {}
