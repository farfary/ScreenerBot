/**
 * Presentation for who manages a position, in one place.
 *
 * Ids are the serde form of `PositionManagement` (src/positions/types.rs).
 */

/** Message key of each mode's label; keep in step with `PositionManagement`. */
export const POSITION_MANAGEMENT_LABELS = Object.freeze({
  auto_trader: "positions-management-auto-trader",
  user_only: "positions-management-user-only",
  copy_task: "positions-management-copy-task",
  hybrid: "positions-management-hybrid",
});
