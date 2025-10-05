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
                $(#[$field_meta:meta])*
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
                $(#[$field_meta])*
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
    };
}
