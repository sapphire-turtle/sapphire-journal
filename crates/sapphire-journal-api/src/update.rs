//! A field of a change: set, cleared, or left alone.

/// Represents the three possible states for an optional field in an update
/// operation: set it to a new value, clear it (set to `None`), or leave it
/// unchanged.
///
/// # JSON / MCP representation
///
/// When used as a struct field with `#[serde(default)]`:
/// - field absent  → `Unchanged` (via `Default`)
/// - field `null`  → `Clear`
/// - field `<value>` → `Set(value)`
///
/// Serializing mirrors that: `Set` is the value and `Clear` is `null`.
/// `Unchanged` must be left out of the output by the containing struct
/// (`#[serde(skip_serializing_if = "UpdateOption::is_unchanged")]`).
///
/// The JSON schema is identical to `Option<T>` (nullable value).
#[derive(Debug, Default)]
pub enum UpdateOption<T> {
    Set(T),
    Clear,
    #[default]
    Unchanged,
}

impl<T> UpdateOption<T> {
    pub fn is_unchanged(&self) -> bool {
        matches!(self, UpdateOption::Unchanged)
    }
}

impl<T: serde::Serialize> serde::Serialize for UpdateOption<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            UpdateOption::Set(v) => serializer.serialize_some(v),
            UpdateOption::Clear | UpdateOption::Unchanged => serializer.serialize_none(),
        }
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for UpdateOption<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            None    => UpdateOption::Clear,
            Some(v) => UpdateOption::Set(v),
        })
    }
}

impl<T: schemars::JsonSchema> schemars::JsonSchema for UpdateOption<T> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        format!("Nullable_{}", T::schema_name()).into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // Reuse Option<T>'s schema: the inner value or null.
        Option::<T>::json_schema(generator)
    }

    fn inline_schema() -> bool {
        true
    }
}
