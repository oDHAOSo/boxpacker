use serde::{Deserialize, Serialize};

/// Compatibility input document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InputData {
    pub containers: Vec<InputContainer>,
    pub contents: Vec<Item>,
}

/// A rectangular container in the compatibility input document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InputContainer {
    pub name: String,
    pub width: f64,
    pub length: f64,
    pub height: f64,
}

/// A rectangular item in either a compatibility input or output document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Item {
    pub name: String,
    pub width: f64,
    pub length: f64,
    pub height: f64,
}

/// Compatibility output document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OutputData {
    pub containers: Vec<OutputContainer>,
    pub unplaced_items: Vec<Item>,
}

/// A container and its placements in the compatibility output document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OutputContainer {
    pub name: String,
    pub width: f64,
    pub length: f64,
    pub height: f64,
    #[serde(default)]
    pub placed_items: Vec<PlacedItem>,
}

/// One placed item in a compatibility output document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlacedItem {
    pub name: String,
    pub coords: Cuboid,
    pub color: String,
}

/// Legacy output coordinates and oriented dimensions.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Cuboid {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub l: f64,
    pub h: f64,
}
