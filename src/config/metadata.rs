use serde::Serialize;
use std::collections::BTreeMap;

/// Convenience alias for the metadata map of a config section.
pub type SectionMetadata = BTreeMap<&'static str, FieldMetadata>;

/// Convenience alias for the full configuration metadata map.
pub type ConfigMetadata = BTreeMap<&'static str, SectionMetadata>;

/// Supported config field types exposed to the frontend renderer.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Boolean,
    Number,
    Integer,
    Array,
    String,
    Object,
}

/// Metadata information describing how a config field should be rendered.
#[derive(Debug, Clone, Serialize)]
pub struct FieldMetadata {
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<FieldType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

impl FieldMetadata {
    /// Build metadata for a given field type using optional extras and docs.
    pub fn from_parts<T>(
        default_value: &T,
        extras: FieldMetadataExtras,
        docs: Option<&'static str>,
    ) -> Self
    where
        T: FieldTypeInfo + Serialize,
    {
        let default = serde_json::to_value(default_value).ok();

        FieldMetadata {
            field_type: T::field_type(),
            item_type: T::item_type(),
            label: extras.label,
            hint: extras.hint,
            unit: extras.unit,
            impact: extras.impact,
            category: extras.category,
            min: extras.min,
            max: extras.max,
            step: extras.step,
            placeholder: extras.placeholder,
            docs,
            default,
        }
    }
}

/// Optional metadata overrides supplied via the `#[metadata(...)]` attribute.
#[derive(Debug, Clone, Default)]
pub struct FieldMetadataExtras {
    pub label: Option<&'static str>,
    pub hint: Option<&'static str>,
    pub unit: Option<&'static str>,
    pub impact: Option<&'static str>,
    pub category: Option<&'static str>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub placeholder: Option<&'static str>,
}

/// Trait implemented for every supported config field type to expose rendering hints.
pub trait FieldTypeInfo {
    fn field_type() -> FieldType;
    fn item_type() -> Option<FieldType> {
        None
    }
}

impl FieldTypeInfo for bool {
    fn field_type() -> FieldType {
        FieldType::Boolean
    }
}

impl FieldTypeInfo for String {
    fn field_type() -> FieldType {
        FieldType::String
    }
}

impl FieldTypeInfo for &str {
    fn field_type() -> FieldType {
        FieldType::String
    }
}

impl FieldTypeInfo for f64 {
    fn field_type() -> FieldType {
        FieldType::Number
    }
}

impl FieldTypeInfo for f32 {
    fn field_type() -> FieldType {
        FieldType::Number
    }
}

impl FieldTypeInfo for usize {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for isize {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for u64 {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for i64 {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for u32 {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for i32 {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for u16 {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for i16 {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for u8 {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl FieldTypeInfo for i8 {
    fn field_type() -> FieldType {
        FieldType::Integer
    }
}

impl<T> FieldTypeInfo for Option<T>
where
    T: FieldTypeInfo,
{
    fn field_type() -> FieldType {
        T::field_type()
    }

    fn item_type() -> Option<FieldType> {
        T::item_type()
    }
}

impl<T> FieldTypeInfo for Vec<T>
where
    T: FieldTypeInfo,
{
    fn field_type() -> FieldType {
        FieldType::Array
    }

    fn item_type() -> Option<FieldType> {
        Some(T::field_type())
    }
}

impl<T> FieldTypeInfo for std::collections::VecDeque<T>
where
    T: FieldTypeInfo,
{
    fn field_type() -> FieldType {
        FieldType::Array
    }

    fn item_type() -> Option<FieldType> {
        Some(T::field_type())
    }
}

impl<T> FieldTypeInfo for std::collections::BTreeMap<String, T>
where
    T: FieldTypeInfo,
{
    fn field_type() -> FieldType {
        FieldType::Object
    }
}

/// Aggregate metadata for all config sections that expose UI controls.
pub fn collect_config_metadata() -> ConfigMetadata {
    let mut map = ConfigMetadata::new();

    map.insert("rpc", super::RpcConfig::field_metadata());
    map.insert("trader", super::TraderConfig::field_metadata());
    map.insert("positions", super::PositionsConfig::field_metadata());
    map.insert("filtering", super::FilteringConfig::field_metadata());
    map.insert("swaps", super::SwapsConfig::field_metadata());
    map.insert("tokens", super::TokensConfig::field_metadata());
    map.insert("sol_price", super::SolPriceConfig::field_metadata());
    map.insert("events", super::EventsConfig::field_metadata());
    map.insert("services", super::ServicesConfig::field_metadata());
    map.insert("monitoring", super::MonitoringConfig::field_metadata());
    map.insert("ohlcv", super::OhlcvConfig::field_metadata());

    for section in map.values_mut() {
        for field in section.values_mut() {
            let normalized = field.category.map(normalize_category).unwrap_or("General");
            field.category = Some(normalized);
        }
    }

    map
}

fn normalize_category(category: &str) -> &'static str {
    if category.contains("Timeout") {
        return "Developer";
    }

    match category {
        "Activity" | "Age" | "Blacklist" | "Community" | "Confirmation" | "Core Trading"
        | "Display" | "Liquidity" | "Maintenance" | "Market Cap" | "Performance" | "Profit"
        | "Profit Management" | "Requirements" | "Security" | "Tokens Tab" | "Transactions" => {
            "General"
        }
        "Debug" | "RPC" | "Validation" => "Developer",
        _ => "Advanced",
    }
}

/// Helper macro used within config schemas to populate metadata extras.
#[macro_export]
macro_rules! field_metadata {
    () => {{
        $crate::config::metadata::FieldMetadataExtras::default()
    }};
    ({}) => {{
        $crate::config::metadata::FieldMetadataExtras::default()
    }};
    (@assign $meta:ident,) => {};
    (@assign $meta:ident) => {};
    (@assign $meta:ident, label: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.label = Some($value);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, hint: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.hint = Some($value);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, unit: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.unit = Some($value);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, impact: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.impact = Some($value);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, category: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.category = Some($value);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, min: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.min = Some($value as f64);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, max: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.max = Some($value as f64);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, step: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.step = Some($value as f64);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, placeholder: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.placeholder = Some($value);
        $crate::field_metadata!(@assign $meta $(, $($rest)*)?);
    }};
    (@assign $meta:ident, $unexpected:ident : $_value:expr $(, $($rest:tt)*)?) => {{
        compile_error!(concat!("Unsupported metadata key: ", stringify!($unexpected)));
    }};
    ({ $($tokens:tt)* }) => {{
        let mut extras = $crate::config::metadata::FieldMetadataExtras::default();
        $crate::field_metadata!(@assign extras, $($tokens)*);
        extras
    }};
    ($($tokens:tt)*) => {{
        let mut extras = $crate::config::metadata::FieldMetadataExtras::default();
        $crate::field_metadata!(@assign extras, $($tokens)*);
        extras
    }};
    ($unexpected:tt) => {{
        compile_error!("Invalid metadata declaration");
    }};
}
