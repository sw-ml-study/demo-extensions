//! Bounded, renderer-neutral system-layout interchange parsing and validation.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

const SCHEMA: &str = "sw-ml-study.system-layout";
const VERSION: u32 = 1;
const MAX_SPACES: usize = 64;
const MAX_REGIONS: usize = 65_536;
const MAX_RELATIONSHIPS: usize = 131_072;
const MAX_TEXT_BYTES: usize = 1_024;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    schema: String,
    version: u32,
    provenance: Provenance,
    spaces: Vec<Space>,
    regions: Vec<Region>,
    relationships: Vec<Relationship>,
    #[serde(default)]
    composition: Option<Composition>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    producer: String,
    producer_revision: String,
    generated_at: String,
    source_description: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Space {
    id: String,
    address_unit: String,
    extent: u64,
    #[serde(default)]
    block_size: Option<u64>,
    #[serde(default)]
    sector_size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    id: String,
    space_id: String,
    address: u64,
    extent: u64,
    purpose: String,
    owner: String,
    location: String,
    state: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relationship {
    id: String,
    from_region_id: String,
    to_region_id: String,
    kind: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Composition {
    format: String,
    image_id: String,
    part_index: u32,
    part_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    Malformed,
    UnsupportedSchema,
    UnsupportedVersion(u32),
    Budget(&'static str),
    InvalidText(&'static str),
    DuplicateId(String),
    InvalidSpace(String),
    UnknownSpace(String),
    InvalidRegion(String),
    UnknownRegion(String),
    InvalidRelationship(String),
    InvalidComposition,
}

impl Layout {
    /// Parses and validates one complete layout without retaining borrowed input.
    ///
    /// # Errors
    ///
    /// Returns the first error in the documented validation order.
    pub fn parse(source: &str) -> Result<Self, ValidationError> {
        let layout: Self = serde_json::from_str(source).map_err(|_| ValidationError::Malformed)?;
        layout.validate()?;
        Ok(layout)
    }

    #[must_use]
    pub fn spaces(&self) -> &[Space] {
        &self.spaces
    }

    #[must_use]
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    #[must_use]
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    #[must_use]
    pub const fn composition(&self) -> Option<&Composition> {
        self.composition.as_ref()
    }

    fn validate(&self) -> Result<(), ValidationError> {
        if self.schema != SCHEMA {
            return Err(ValidationError::UnsupportedSchema);
        }
        if self.version != VERSION {
            return Err(ValidationError::UnsupportedVersion(self.version));
        }
        validate_budgets(self)?;
        validate_provenance(&self.provenance)?;
        let spaces = validate_spaces(&self.spaces)?;
        let region_ids = validate_regions(&self.regions, &spaces)?;
        validate_relationships(&self.relationships, &region_ids)?;
        validate_composition(self.composition.as_ref())
    }
}

impl Provenance {
    #[must_use]
    pub fn producer(&self) -> &str {
        &self.producer
    }
    #[must_use]
    pub fn producer_revision(&self) -> &str {
        &self.producer_revision
    }
    #[must_use]
    pub fn generated_at(&self) -> &str {
        &self.generated_at
    }
    #[must_use]
    pub fn source_description(&self) -> &str {
        &self.source_description
    }
}

impl Space {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn address_unit(&self) -> &str {
        &self.address_unit
    }
    #[must_use]
    pub const fn extent(&self) -> u64 {
        self.extent
    }
    #[must_use]
    pub const fn block_size(&self) -> Option<u64> {
        self.block_size
    }
    #[must_use]
    pub const fn sector_size(&self) -> Option<u64> {
        self.sector_size
    }
}

impl Region {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn space_id(&self) -> &str {
        &self.space_id
    }
    #[must_use]
    pub const fn address(&self) -> u64 {
        self.address
    }
    #[must_use]
    pub const fn extent(&self) -> u64 {
        self.extent
    }
    #[must_use]
    pub fn purpose(&self) -> &str {
        &self.purpose
    }
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }
    #[must_use]
    pub fn location(&self) -> &str {
        &self.location
    }
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }
}

impl Relationship {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn from_region_id(&self) -> &str {
        &self.from_region_id
    }
    #[must_use]
    pub fn to_region_id(&self) -> &str {
        &self.to_region_id
    }
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
}

impl Composition {
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }
    #[must_use]
    pub fn image_id(&self) -> &str {
        &self.image_id
    }
    #[must_use]
    pub const fn part_index(&self) -> u32 {
        self.part_index
    }
    #[must_use]
    pub const fn part_count(&self) -> u32 {
        self.part_count
    }
}

fn validate_budgets(layout: &Layout) -> Result<(), ValidationError> {
    if layout.spaces.is_empty() || layout.spaces.len() > MAX_SPACES {
        return Err(ValidationError::Budget("spaces"));
    }
    if layout.regions.len() > MAX_REGIONS {
        return Err(ValidationError::Budget("regions"));
    }
    if layout.relationships.len() > MAX_RELATIONSHIPS {
        return Err(ValidationError::Budget("relationships"));
    }
    Ok(())
}

fn validate_provenance(value: &Provenance) -> Result<(), ValidationError> {
    validate_text(&value.producer, "provenance.producer")?;
    validate_text(&value.producer_revision, "provenance.producer_revision")?;
    validate_text(&value.generated_at, "provenance.generated_at")?;
    validate_text(&value.source_description, "provenance.source_description")
}

fn validate_spaces(spaces: &[Space]) -> Result<HashMap<&str, u64>, ValidationError> {
    let mut ids = HashMap::with_capacity(spaces.len());
    for space in spaces {
        validate_text(&space.id, "space.id")?;
        validate_text(&space.address_unit, "space.address_unit")?;
        if ids.insert(space.id.as_str(), space.extent).is_some() {
            return Err(ValidationError::DuplicateId(space.id.clone()));
        }
        if space.extent == 0
            || space.block_size == Some(0)
            || space.sector_size == Some(0)
            || space
                .block_size
                .is_some_and(|size| space.extent % size != 0)
            || space
                .sector_size
                .is_some_and(|size| space.extent % size != 0)
        {
            return Err(ValidationError::InvalidSpace(space.id.clone()));
        }
    }
    Ok(ids)
}

fn validate_regions<'a>(
    regions: &'a [Region],
    spaces: &HashMap<&str, u64>,
) -> Result<HashSet<&'a str>, ValidationError> {
    let mut ids = HashSet::with_capacity(regions.len());
    for region in regions {
        for (value, field) in [
            (&region.id, "region.id"),
            (&region.purpose, "region.purpose"),
            (&region.owner, "region.owner"),
            (&region.location, "region.location"),
            (&region.state, "region.state"),
        ] {
            validate_text(value, field)?;
        }
        if !ids.insert(region.id.as_str()) {
            return Err(ValidationError::DuplicateId(region.id.clone()));
        }
        let Some(space_extent) = spaces.get(region.space_id.as_str()) else {
            return Err(ValidationError::UnknownSpace(region.space_id.clone()));
        };
        let Some(end) = region.address.checked_add(region.extent) else {
            return Err(ValidationError::InvalidRegion(region.id.clone()));
        };
        if region.extent == 0 || end > *space_extent {
            return Err(ValidationError::InvalidRegion(region.id.clone()));
        }
    }
    Ok(ids)
}

fn validate_relationships(
    relationships: &[Relationship],
    region_ids: &HashSet<&str>,
) -> Result<(), ValidationError> {
    let mut ids = HashSet::with_capacity(relationships.len());
    for relationship in relationships {
        validate_text(&relationship.id, "relationship.id")?;
        validate_text(&relationship.kind, "relationship.kind")?;
        if !ids.insert(relationship.id.as_str()) {
            return Err(ValidationError::DuplicateId(relationship.id.clone()));
        }
        for region_id in [&relationship.from_region_id, &relationship.to_region_id] {
            if !region_ids.contains(region_id.as_str()) {
                return Err(ValidationError::UnknownRegion(region_id.clone()));
            }
        }
        if relationship.from_region_id == relationship.to_region_id {
            return Err(ValidationError::InvalidRelationship(
                relationship.id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_composition(value: Option<&Composition>) -> Result<(), ValidationError> {
    let Some(value) = value else { return Ok(()) };
    if value.format != "C24IMG"
        || validate_text(&value.image_id, "composition.image_id").is_err()
        || value.part_count == 0
        || value.part_index >= value.part_count
    {
        return Err(ValidationError::InvalidComposition);
    }
    Ok(())
}

fn validate_text(value: &str, field: &'static str) -> Result<(), ValidationError> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(ValidationError::InvalidText(field));
    }
    Ok(())
}
