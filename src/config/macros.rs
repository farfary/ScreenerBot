/// Configuration macros for zero-repetition config definitions
///
/// This module provides the `config_struct!` macro that allows defining
/// configuration structures with embedded defaults in a single declaration.

/// Define a configuration struct with embedded defaults
///
/// This macro eliminates repetition by allowing you to define:
/// - Field name
/// - Field type
/// - Default value
/// All in one place, and it generates:
/// - The struct with public fields
/// - The Default implementation
/// - Serde serialization/deserialization with defaults
///
/// # Example
/// ```
/// config_struct! {
///     pub struct TraderConfig {
///         max_open_positions: usize = 2,
///         trade_size_sol: f64 = 0.005,
///         enabled: bool = true,
///     }
/// }
/// ```
///
/// This generates:
/// - A struct with public fields
/// - A Default implementation with the specified values
/// - Serde support with `#[serde(default)]`
#[macro_export]
macro_rules! config_struct {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[doc = $doc:expr])*
                $(#[serde($($serde_attr:tt)*)])?
                $(#[metadata($metadata:expr)])?
                $field_name:ident: $field_type:ty = $default_value:expr
            ),*
            $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(default)]
        $vis struct $name {
            $(
                $(#[doc = $doc])*
                $(#[serde($($serde_attr)*)])?
                pub $field_name: $field_type,
            )*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $(
                        $field_name: $default_value,
                    )*
                }
            }
        }

        impl $crate::config::metadata::FieldTypeInfo for $name {
            fn field_type() -> $crate::config::metadata::FieldType {
                $crate::config::metadata::FieldType::Object
            }
        }

        impl $crate::config::metadata::NestedMetadata for $name {
            fn nested_metadata() -> Option<$crate::config::metadata::SectionMetadata> {
                Some(Self::field_metadata())
            }
        }

        impl $name {
            /// Generate metadata for all fields defined in this configuration struct.
            pub fn field_metadata() -> $crate::config::metadata::SectionMetadata {
                let mut fields = $crate::config::metadata::SectionMetadata::new();

                $(
                    #[allow(unused_mut)]
                    let mut extras = $crate::config::metadata::FieldMetadataExtras::default();
                    $(extras = $metadata;)?

                    let docs: Option<&'static str> = {
                        let doc = concat!($($doc, "\n",)* "");
                        let doc = doc.trim();
                        if doc.is_empty() { None } else { Some(doc) }
                    };

                    let default_value: $field_type = $default_value;
                    let mut metadata = $crate::config::metadata::FieldMetadata::from_parts::<$field_type>(
                        &default_value,
                        extras,
                        docs,
                    );
                    metadata.children = <$field_type as $crate::config::metadata::NestedMetadata>::nested_metadata();
                    fields.insert(stringify!($field_name), metadata);
                )*

                fields
            }
        }
    };
}
