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
    /// Visibility level for UI rendering: "primary", "secondary", or "technical"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<&'static str>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<SectionMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
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
            visibility: None, // Set by collect_config_metadata based on category
            min: extras.min,
            max: extras.max,
            step: extras.step,
            placeholder: extras.placeholder,
            docs,
            default,
            children: None,
            hidden: if extras.hidden { Some(true) } else { None },
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
    pub hidden: bool,
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

/// Trait describing whether a field exposes nested metadata for structured rendering.
pub trait NestedMetadata {
    fn nested_metadata() -> Option<SectionMetadata> {
        None
    }
}

macro_rules! impl_nested_metadata_for_primitives {
    ($($ty:ty),+ $(,)?) => {
        $(impl NestedMetadata for $ty {})+
    };
}

impl_nested_metadata_for_primitives!(
    bool, String, &str, f64, f32, usize, isize, u64, i64, u32, i32, u16, i16, u8, i8
);

impl<T> NestedMetadata for Option<T>
where
    T: NestedMetadata,
{
    fn nested_metadata() -> Option<SectionMetadata> {
        T::nested_metadata()
    }
}

impl<T> NestedMetadata for Vec<T> where T: NestedMetadata {}

impl<T> NestedMetadata for std::collections::VecDeque<T> where T: NestedMetadata {}

impl<T> NestedMetadata for std::collections::BTreeMap<String, T> where T: NestedMetadata {}

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
    map.insert("webserver", super::WebserverConfig::field_metadata());
    map.insert("telegram", super::TelegramConfig::field_metadata());

    for section in map.values_mut() {
        section.retain(|_, field| !field.hidden.unwrap_or(false));

        for field in section.values_mut() {
            if let Some(ref mut children) = field.children {
                children.retain(|_, child| !child.hidden.unwrap_or(false));
            }

            // Keep original category, derive visibility from it
            let category = field.category.unwrap_or("General");
            field.category = Some(category);
            field.visibility = Some(derive_visibility(category));
        }
    }

    map
}

/// Determines visibility level for a category
/// - "primary": Essential settings, expanded by default
/// - "secondary": Important but less frequently changed, collapsed
/// - "technical": Power-user settings (timeouts, retries, etc.), collapsed and grouped at bottom
fn derive_visibility(category: &str) -> &'static str {
    match category {
        // Primary - Essential user settings (expand by default)
        "Core Trading" | "ROI Exit" | "DCA" | "Trailing Stop" | "Liquidity" | "Market Cap"
        | "Security" | "Age" | "Source Control" | "Global Control" | "Connection"
        | "Notifications" | "Endpoints" | "Router" | "Slippage" | "Profit" | "Loss Detection"
        | "Partial Exit" | "General" | "Commands" | "Features" => "primary",

        // Secondary - Authentication, thresholds, etc.
        "Authentication" | "Thresholds" => "secondary",

        // Technical - Power-user settings (collapsed, grouped at bottom)
        "Timeouts" | "Retries" | "Rate Limiting" | "Circuit Breaker" | "Connection Pooling"
        | "Statistics" | "Cache" | "Retention" | "Debug" | "Validation" | "Provider Selection" => {
            "technical"
        }

        // Secondary - Everything else (collapsed)
        _ => "secondary",
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
    (@assign $meta:ident, hidden: $value:expr $(, $($rest:tt)*)?) => {{
        $meta.hidden = $value;
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
