# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = In-memory configuration differs from disk version
system-result-config-matches = In-memory configuration matches disk version

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    Successfully imported { $count ->
        [one] { $count } section
       *[other] { $count } sections
    }
system-result-config-imported-with-warnings =
    Imported { $count ->
        [one] { $count } section
       *[other] { $count } sections
    } with { $warnings ->
        [one] { $warnings } warning
       *[other] { $warnings } warnings
    }: { $details }
