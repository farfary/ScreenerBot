# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = Position created

# Result of changing who manages a position. Id comes from set_management in
# src/webserver/routes/positions/manage.rs. $management is the PositionManagement
# id (auto_trader, user_only, copy_task, hybrid).
positions-result-management-set = Position management set to { $management }
